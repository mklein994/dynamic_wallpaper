extern crate chrono;
extern crate chrono_humanize;
extern crate dotenv;
extern crate env_logger;
extern crate failure;
#[macro_use]
extern crate log;
extern crate spa;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, Duration, Local, Timelike, Utc};
use failure::ResultExt;
use std::fmt;

type Result<T> = std::result::Result<T, failure::Error>;

fn init() {
    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .default_format_timestamp(false)
        .init();
    info!("logging enabled");

    dotenv::dotenv().ok();
    info!("dotenv ok ✓");
}

pub fn run() -> Result<()> {
    init();

    let config = Config::new()?;
    let now = config.now;

    let wallpaper = Wallpaper::new()?;

    let sun = Sun::new(config.now, config.lat, config.lon)?;

    info!("{}", sun.time_period);

    let image_count = wallpaper.image_count(&sun.time_period);
    debug!("image count: {}", image_count);

    let index = get_index(now, &sun, image_count);
    debug!("index: {}/{}", index, image_count);

    let image = get_image(index, &sun.time_period, &wallpaper);

    println!("{}", image);

    Ok(())
}

fn get_image(index: i64, time_period: &TimePeriod, wallpaper: &Wallpaper) -> i64 {
    let mut image = match time_period {
        TimePeriod::DayTime => index + wallpaper.daybreak,
        _ => index + wallpaper.nightfall,
    };

    if image > wallpaper.count {
        image -= wallpaper.count;
    }

    image
}

fn get_index(now: DateTime<Utc>, sun: &Sun, image_count: i64) -> i64 {
    let (start, end) = sun.start_end();
    let elapsed_time = now - start;
    let elapsed_percent = elapsed_time.num_nanoseconds().unwrap() as f64 * 100_f64
        / (end - start).num_nanoseconds().unwrap() as f64;
    debug!(
        "elapsed time: {} ({}%)",
        format_duration(elapsed_time),
        elapsed_percent
    );

    debug!(
        "{}",
        elapsed_time.num_nanoseconds().unwrap() as f64
            / ((end - start).num_nanoseconds().unwrap() as f64 / image_count as f64)
    );

    // alternate
    debug!(
        "{}",
        (elapsed_time.num_nanoseconds().unwrap() * image_count) as f64
            / (end - start).num_nanoseconds().unwrap() as f64
    );
    (elapsed_time.num_nanoseconds().unwrap() * image_count)
        / (end - start).num_nanoseconds().unwrap()

    //  elapsed_time
    // ━━━━━━━━━━━━━━━
    //  (end - start)
    //  ─────────────
    //   image_count
}

#[derive(Debug)]
struct Config {
    now: DateTime<Utc>,
    lat: f64,
    lon: f64,
}

impl Config {
    fn new() -> Result<Self> {
        use std::env;

        let now = if let Ok(n) = env::var("WALLPAPER_NOW") {
            warn!("Using WALLPAPER_NOW as current time");
            DateTime::parse_from_rfc3339(&n)
                .with_context(|c| format!("WALLPAPER_NOW: {}", c))?
                .with_timezone(&Utc)
        } else {
            Utc::now()
        };

        let lat = env::var("WALLPAPER_LAT")
            .with_context(|c| format!("WALLPAPER_LAT: {}", c))?
            .parse::<f64>()
            .with_context(|c| format!("WALLPAPER_LAT: {}", c))?;
        let lon = env::var("WALLPAPER_LON")
            .with_context(|c| format!("WALLPAPER_LON: {}", c))?
            .parse::<f64>()
            .with_context(|c| format!("WALLPAPER_LON: {}", c))?;

        Ok(Self { now, lat, lon })
    }
}

#[derive(Debug)]
struct Wallpaper {
    count: i64,
    daybreak: i64,
    nightfall: i64,
}

impl Wallpaper {
    fn new() -> Result<Self> {
        use std::env;

        let count = env::var("WALLPAPER_COUNT")
            .with_context(|c| format!("WALLPAPER_COUNT: {}", c))?
            .parse::<u8>()?;
        let daybreak = env::var("WALLPAPER_DAYBREAK")
            .with_context(|c| format!("WALLPAPER_DAYBREAK: {} ", c))?
            .parse::<u8>()
            .with_context(|c| format!("WALLPAPER_DAYBREAK: {}", c))?;
        let nightfall = env::var("WALLPAPER_NIGHTFALL")
            .with_context(|c| format!("WALLPAPER_NIGHTFALL: {}", c))?
            .parse::<u8>()
            .with_context(|c| format!("WALLPAPER_NIGHTFALL: {}", c))?;

        Ok(Self {
            count: i64::from(count),
            daybreak: i64::from(daybreak),
            nightfall: i64::from(nightfall),
        })
    }

    fn image_count(&self, time_period: &TimePeriod) -> i64 {
        match time_period {
            TimePeriod::DayTime => self.nightfall - self.daybreak,
            _ => self.count - self.nightfall + self.daybreak,
        }
    }
}

#[derive(Debug)]
struct Sun {
    last_sunset: DateTime<Utc>,
    sunrise: DateTime<Utc>,
    sunset: DateTime<Utc>,
    next_sunrise: DateTime<Utc>,
    time_period: TimePeriod,
}

impl Sun {
    fn new(now: DateTime<Utc>, lat: f64, lon: f64) -> Result<Self> {
        use spa::SunriseAndSet;

        let utc_now = now
            .with_timezone(&Local)
            .date()
            .and_hms(12, 0, 0)
            .with_timezone(&Utc);

        info!("now:          {}", now.with_timezone(&Local));
        debug!("UTC now:      {}", utc_now);

        debug_assert!(Utc::today() - Duration::days(1) <= now.date());
        debug_assert!(now.date() <= Utc::today() + Duration::days(1));

        fn halfway(start: DateTime<Utc>, end: DateTime<Utc>) {
            debug!(
                "½ way:        {}",
                start.with_timezone(&Local)
                    + Duration::nanoseconds((end - start).num_nanoseconds().unwrap() / 2)
            );
        }

        let last_sunset = match spa::calc_sunrise_and_set(utc_now - Duration::days(1), lat, lon)? {
            SunriseAndSet::Daylight(_, sunset) => sunset,
            _ => unimplemented!(),
        };
        info!("last sunset:  {}", last_sunset.with_timezone(&Local));

        let (sunrise, sunset) = match spa::calc_sunrise_and_set(utc_now, lat, lon)? {
            SunriseAndSet::Daylight(sunrise, sunset) => (sunrise, sunset),
            _ => unimplemented!(),
        };
        halfway(last_sunset, sunrise);
        info!("sunrise:      {}", sunrise.with_timezone(&Local));
        halfway(sunrise, sunset);
        info!("sunset:       {}", sunset.with_timezone(&Local));

        let next_sunrise = match spa::calc_sunrise_and_set(utc_now + Duration::days(1), lat, lon)? {
            spa::SunriseAndSet::Daylight(sunrise, _) => sunrise,
            _ => unimplemented!(),
        };
        halfway(sunset, next_sunrise);
        info!("next sunrise: {}", next_sunrise.with_timezone(&Local));

        debug_assert!(last_sunset < sunrise);
        debug_assert!(sunrise < sunset);
        debug_assert!(sunset < next_sunrise);

        debug_assert!(last_sunset.with_nanosecond(0).unwrap() <= now.with_nanosecond(0).unwrap());
        debug_assert!(now.with_nanosecond(0).unwrap() <= next_sunrise.with_nanosecond(0).unwrap());

        let time_period = Sun::time_period(&now, &sunrise, &sunset);

        Ok(Self {
            last_sunset,
            sunrise,
            sunset,
            next_sunrise,
            time_period,
        })
    }

    fn start_end(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        match self.time_period {
            TimePeriod::BeforeSunrise => (self.last_sunset, self.sunrise),
            TimePeriod::DayTime => (self.sunrise, self.sunset),
            TimePeriod::AfterSunset => (self.sunset, self.next_sunrise),
        }
    }

    fn time_period(
        now: &DateTime<Utc>,
        sunrise: &DateTime<Utc>,
        sunset: &DateTime<Utc>,
    ) -> TimePeriod {
        if *now > *sunset {
            TimePeriod::AfterSunset
        } else if *now >= *sunrise {
            TimePeriod::DayTime
        } else {
            TimePeriod::BeforeSunrise
        }
    }
}

impl fmt::Display for Sun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Sun:\n{}\n{:<13} {}\n{:<13} {}\n{:<13} {}\n{:<13} {}",
            "-".repeat(50),
            "last sunset:",
            self.last_sunset.with_timezone(&Local),
            "sunrise:",
            self.sunrise.with_timezone(&Local),
            "sunset:",
            self.sunset.with_timezone(&Local),
            "next sunrise:",
            self.next_sunrise.with_timezone(&Local)
        )
    }
}

#[derive(Debug, PartialEq)]
enum TimePeriod {
    AfterSunset,
    BeforeSunrise,
    DayTime,
}

impl fmt::Display for TimePeriod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TimePeriod::AfterSunset => write!(f, "\u{1f306} After Sunset"),
            TimePeriod::BeforeSunrise => write!(f, "\u{1f305} Before Sunrise"),
            TimePeriod::DayTime => write!(f, "\u{1f3d9} Daytime"),
        }
    }
}

fn format_duration(duration: Duration) -> String {
    use chrono_humanize::{Accuracy, HumanTime, Tense};
    HumanTime::from(duration).to_text_en(Accuracy::Precise, Tense::Present)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    lazy_static! {
        static ref SUN: Sun = Sun {
            last_sunset: Local.ymd(2018, 8, 5).and_hms(21, 3, 24).with_timezone(&Utc),
            sunrise: Local.ymd(2018, 8, 6).and_hms(6, 4, 25).with_timezone(&Utc),
            sunset: Local.ymd(2018, 8, 6).and_hms(21, 1, 44).with_timezone(&Utc),
            next_sunrise: Local.ymd(2018, 8, 6).and_hms(6, 5, 52).with_timezone(&Utc),
            time_period: TimePeriod::DayTime,
        };
        static ref SUN_AFTER_SUNSET: Sun = Sun {
            last_sunset: Local.ymd(2018, 8, 5).and_hms(21, 3, 24).with_timezone(&Utc),
            sunrise: Local.ymd(2018, 8, 6).and_hms(6, 4, 25).with_timezone(&Utc),
            sunset: Local.ymd(2018, 8, 6).and_hms(21, 1, 44).with_timezone(&Utc),
            next_sunrise: Local.ymd(2018, 8, 6).and_hms(6, 5, 52).with_timezone(&Utc),
            time_period: TimePeriod::AfterSunset,
        };
        static ref SUN_BEFORE_SUNRISE: Sun = Sun {
            last_sunset: Local.ymd(2018, 8, 5).and_hms(21, 3, 24).with_timezone(&Utc),
            sunrise: Local.ymd(2018, 8, 6).and_hms(6, 4, 25).with_timezone(&Utc),
            sunset: Local.ymd(2018, 8, 6).and_hms(21, 1, 44).with_timezone(&Utc),
            next_sunrise: Local.ymd(2018, 8, 6).and_hms(6, 5, 52).with_timezone(&Utc),
            time_period: TimePeriod::BeforeSunrise,
        };
    }

    const WALLPAPER: Wallpaper = Wallpaper {
        count: 16,
        daybreak: 2,
        nightfall: 13,
    };
    const DAYTIME_IMAGE_COUNT: i64 = 11;
    const NIGHTTIME_IMAGE_COUNT: i64 = 5;

    #[test]
    fn image_count_daytime() {
        let image_count = WALLPAPER.image_count(&TimePeriod::DayTime);
        assert_eq!(11, image_count);
    }

    #[test]
    fn image_count_before_sunrise() {
        let image_count = WALLPAPER.image_count(&TimePeriod::BeforeSunrise);
        assert_eq!(5, image_count);
    }

    #[test]
    fn image_count_after_sunset() {
        let image_count = WALLPAPER.image_count(&TimePeriod::AfterSunset);
        assert_eq!(5, image_count);
    }

    #[test]
    #[ignore]
    fn image_count_daytime_daybreak_greater_than_nightfall() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 13,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::DayTime);
        assert_eq!(5, image_count);
    }

    #[test]
    #[ignore]
    fn image_count_before_sunrise_daybreak_greater_than_nightfall() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 13,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::BeforeSunrise);
        assert_eq!(11, image_count);
    }

    #[test]
    #[ignore]
    fn image_count_after_sunset_daybreak_greater_than_nightfall() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 13,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::AfterSunset);
        assert_eq!(11, image_count);
    }

    #[test]
    fn time_period_noon() {
        let time_period = Sun::time_period(
            &Local.ymd(2018, 8, 6).and_hms(12, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_last_midnight() {
        let time_period = Sun::time_period(
            &Local.ymd(2018, 8, 6).and_hms(0, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::BeforeSunrise, time_period);
    }

    #[test]
    fn time_period_next_midnight() {
        let time_period = Sun::time_period(
            &Local.ymd(2018, 8, 7).and_hms(0, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::AfterSunset, time_period);
    }

    #[test]
    fn time_period_sunrise() {
        let time_period = Sun::time_period(&SUN.sunrise, &SUN.sunrise, &SUN.sunset);
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_sunset() {
        let time_period = Sun::time_period(&SUN.sunset, &SUN.sunrise, &SUN.sunset);
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_just_before_sunset() {
        let time_period = Sun::time_period(
            &(SUN.sunset - Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_just_after_sunset() {
        let time_period = Sun::time_period(
            &(SUN.sunset + Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::AfterSunset, time_period);
    }

    #[test]
    fn time_period_just_before_sunrise() {
        let time_period = Sun::time_period(
            &(SUN.sunrise - Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::BeforeSunrise, time_period);
    }

    #[test]
    fn time_period_just_after_sunrise() {
        let time_period = Sun::time_period(
            &(SUN.sunrise + Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn get_index_sunrise() {
        let index = get_index(SUN.sunrise, &SUN, DAYTIME_IMAGE_COUNT);
        assert_eq!(0, index);
    }

    #[test]
    fn get_index_sunset() {
        let index = get_index(SUN.sunset, &SUN, DAYTIME_IMAGE_COUNT);
        assert_eq!(DAYTIME_IMAGE_COUNT, index);
    }

    #[test]
    fn get_index_just_past_sunrise() {
        let now = SUN.sunrise + Duration::nanoseconds(1);
        let index = get_index(now, &SUN, DAYTIME_IMAGE_COUNT);
        assert_eq!(0, index);
    }

    #[test]
    fn get_index_just_before_sunrise() {
        let now = SUN_BEFORE_SUNRISE.sunrise - Duration::nanoseconds(1);
        debug_assert!(now < SUN_BEFORE_SUNRISE.sunrise);
        let index = get_index(now, &SUN_BEFORE_SUNRISE, NIGHTTIME_IMAGE_COUNT);
        assert_eq!(NIGHTTIME_IMAGE_COUNT - 1, index);
    }

    #[test]
    fn get_index_just_before_sunset() {
        let now = SUN.sunset - Duration::nanoseconds(1);
        debug_assert!(now < SUN.sunset);
        let index = get_index(now, &SUN, DAYTIME_IMAGE_COUNT);
        assert_eq!(DAYTIME_IMAGE_COUNT - 1, index);
    }

    #[test]
    fn get_index_just_past_sunset() {
        let sun = &SUN_AFTER_SUNSET;
        let now = sun.sunset + Duration::nanoseconds(1);
        let index = get_index(now, sun, NIGHTTIME_IMAGE_COUNT);
        assert_eq!(0, index);
    }

}
