extern crate chrono;
extern crate chrono_humanize;
extern crate dirs;
extern crate env_logger;
extern crate failure;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate spa;
extern crate toml;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, Duration, Local, Timelike, Utc};
use std::fmt;

type Result<T> = std::result::Result<T, failure::Error>;

fn init() {
    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .default_format_timestamp(false)
        .init();
    info!("logging enabled");
}

pub fn run() -> Result<()> {
    init();

    let config = Config::new()?;
    let now = config.now;
    let wallpaper = config.wallpaper;

    let sun = Sun::new(now, config.lat, config.lon)?;

    let time_period = TimePeriod::new(&now, &sun.sunrise, &sun.sunset);
    info!("{}", time_period);

    let image_count = wallpaper.image_count(&time_period);
    debug!("image count: {}", image_count);

    let index = get_index(now, &sun, &time_period, image_count);
    debug!("index: {}/{}", index, image_count);

    let image = get_image(index, &time_period, &wallpaper);

    println!("{}", image);

    Ok(())
}

/// Using the index, add the correct offset to add based on the time period.
///
/// The offset comes from either daybreak or nightfall, depending on the time period.
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

/// Get the index for the current time, within the time period, from the `image_count` number of
/// images.
fn get_index(now: DateTime<Utc>, sun: &Sun, time_period: &TimePeriod, image_count: i64) -> i64 {
    let (start, end) = sun.start_end(time_period);
    let elapsed_time = now - start;
    debug!(
        "elapsed time: {} ({:.2}%)",
        format_duration(elapsed_time),
        // calculate as a percent
        elapsed_time.num_nanoseconds().unwrap() as f64 * 100_f64
            / (end - start).num_nanoseconds().unwrap() as f64
    );

    //  elapsed_time
    // ━━━━━━━━━━━━━━━
    //  (end - start)
    //  ─────────────
    //   image_count
    (elapsed_time.num_nanoseconds().unwrap() * image_count)
        / (end - start).num_nanoseconds().unwrap()
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default = "default_time")]
    now: DateTime<Utc>,
    lat: f64,
    lon: f64,
    #[serde(default)]
    wallpaper: Wallpaper,
}

fn default_time() -> DateTime<Utc> {
    Utc::now()
}

impl Config {
    fn new() -> Result<Self> {
        use std::fs;

        let filename = dirs::config_dir()
            .expect("Couldn't find $XDG_CONFIG_DIR (~/.config/)")
            .join("dynamic_wallpaper")
            .join("config.toml");

        let contents = fs::read_to_string(filename)?;

        let config: Config = toml::from_str(&contents)?;

        Ok(config)
    }
}

#[derive(Debug, Deserialize)]
struct Wallpaper {
    count: i64,
    daybreak: i64,
    nightfall: i64,
}

impl Wallpaper {
    fn image_count(&self, time_period: &TimePeriod) -> i64 {
        match time_period {
            TimePeriod::DayTime => self.nightfall - self.daybreak,
            _ => self.count - self.nightfall + self.daybreak,
        }
    }
}

impl Default for Wallpaper {
    fn default() -> Self {
        Self {
            count: 16,
            daybreak: 2,
            nightfall: 13,
        }
    }
}

#[derive(Debug)]
struct Sun {
    last_sunset: DateTime<Utc>,
    sunrise: DateTime<Utc>,
    sunset: DateTime<Utc>,
    next_sunrise: DateTime<Utc>,
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

        Ok(Self {
            last_sunset,
            sunrise,
            sunset,
            next_sunrise,
        })
    }

    fn start_end(&self, time_period: &TimePeriod) -> (DateTime<Utc>, DateTime<Utc>) {
        match time_period {
            TimePeriod::BeforeSunrise => (self.last_sunset, self.sunrise),
            TimePeriod::DayTime => (self.sunrise, self.sunset),
            TimePeriod::AfterSunset => (self.sunset, self.next_sunrise),
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

impl TimePeriod {
    fn new(now: &DateTime<Utc>, sunrise: &DateTime<Utc>, sunset: &DateTime<Utc>) -> Self {
        if *now > *sunset {
            TimePeriod::AfterSunset
        } else if *now >= *sunrise {
            TimePeriod::DayTime
        } else {
            TimePeriod::BeforeSunrise
        }
    }
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
        let time_period = TimePeriod::new(
            &Local.ymd(2018, 8, 6).and_hms(12, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_last_midnight() {
        let time_period = TimePeriod::new(
            &Local.ymd(2018, 8, 6).and_hms(0, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::BeforeSunrise, time_period);
    }

    #[test]
    fn time_period_next_midnight() {
        let time_period = TimePeriod::new(
            &Local.ymd(2018, 8, 7).and_hms(0, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::AfterSunset, time_period);
    }

    #[test]
    fn time_period_sunrise() {
        let time_period = TimePeriod::new(&SUN.sunrise, &SUN.sunrise, &SUN.sunset);
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_sunset() {
        let time_period = TimePeriod::new(&SUN.sunset, &SUN.sunrise, &SUN.sunset);
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_just_before_sunset() {
        let time_period = TimePeriod::new(
            &(SUN.sunset - Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_just_after_sunset() {
        let time_period = TimePeriod::new(
            &(SUN.sunset + Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::AfterSunset, time_period);
    }

    #[test]
    fn time_period_just_before_sunrise() {
        let time_period = TimePeriod::new(
            &(SUN.sunrise - Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::BeforeSunrise, time_period);
    }

    #[test]
    fn time_period_just_after_sunrise() {
        let time_period = TimePeriod::new(
            &(SUN.sunrise + Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn get_index_sunrise() {
        let index = get_index(SUN.sunrise, &SUN, &TimePeriod::DayTime, DAYTIME_IMAGE_COUNT);
        assert_eq!(0, index);
    }

    #[test]
    fn get_index_sunset() {
        let index = get_index(SUN.sunset, &SUN, &TimePeriod::DayTime, DAYTIME_IMAGE_COUNT);
        assert_eq!(DAYTIME_IMAGE_COUNT, index);
    }

    #[test]
    fn get_index_just_past_sunrise() {
        let now = SUN.sunrise + Duration::nanoseconds(1);
        let index = get_index(now, &SUN, &TimePeriod::DayTime, DAYTIME_IMAGE_COUNT);
        assert_eq!(0, index);
    }

    #[test]
    fn get_index_just_before_sunrise() {
        let now = SUN.sunrise - Duration::nanoseconds(1);
        debug_assert!(now < SUN.sunrise);
        let index = get_index(now, &SUN, &TimePeriod::BeforeSunrise, NIGHTTIME_IMAGE_COUNT);
        assert_eq!(NIGHTTIME_IMAGE_COUNT - 1, index);
    }

    #[test]
    fn get_index_just_before_sunset() {
        let now = SUN.sunset - Duration::nanoseconds(1);
        debug_assert!(now < SUN.sunset);
        let index = get_index(now, &SUN, &TimePeriod::DayTime, DAYTIME_IMAGE_COUNT);
        assert_eq!(DAYTIME_IMAGE_COUNT - 1, index);
    }

    #[test]
    fn get_index_just_past_sunset() {
        let now = SUN.sunset + Duration::nanoseconds(1);
        let index = get_index(now, &SUN, &TimePeriod::AfterSunset, NIGHTTIME_IMAGE_COUNT);
        assert_eq!(0, index);
    }

}
