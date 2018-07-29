#[macro_use]
extern crate log;
extern crate chrono;
extern crate chrono_humanize;
extern crate dotenv;
extern crate env_logger;
extern crate failure;
extern crate spa;

use chrono::{DateTime, Duration, Local, Timelike, Utc};
use failure::ResultExt;
use std::fmt;

type Result<T> = std::result::Result<T, failure::Error>;

pub fn run() -> Result<()> {
    init();

    let config = Config::new().with_context(|c| format!("Config: {}", c))?;
    info!("config set ✓");
    debug!("{:#?}", config);

    let wallpaper = Wallpaper::new().with_context(|c| format!("Wallpaper: {}", c))?;
    debug!("{:#?}", wallpaper);

    let sun = Sun::new(config.now, config.lat, config.lon).with_context(|c| format!("Sun: {}", c))?;
    info!("{}", sun);
    info!("now:    {}", config.now.with_timezone(&Local));

    let time_period = TimePeriod::new(config.now, sun.sunrise, sun.sunset);
    info!("{}", time_period);

    let (start, end) = match time_period {
        TimePeriod::BeforeSunrise => (sun.last_sunset, sun.sunrise),
        TimePeriod::DayTime => (sun.sunrise, sun.sunset),
        TimePeriod::AfterSunset => (sun.sunset, sun.next_sunrise),
    };
    debug!(
        "start time: {} ({})",
        start.with_timezone(&Local),
        match time_period {
            TimePeriod::DayTime => "sunrise",
            TimePeriod::BeforeSunrise => "last sunset",
            TimePeriod::AfterSunset => "sunset",
        }
    );
    debug!("end time:   {}", end.with_timezone(&Local));

    debug!(
        "halfway:    {}",
        start.with_timezone(&Local) + (end - start) / 2
    );

    let duration = end - start;
    debug!("duration: {}", format_duration(duration));

    let time_since_start = config.now - start;
    debug!(
        "time since start: {} ({}%)",
        format_duration(time_since_start),
        100 * time_since_start.num_nanoseconds().unwrap() / duration.num_nanoseconds().unwrap()
    );

    let image_count = match time_period {
        TimePeriod::DayTime => wallpaper.sunset - wallpaper.sunrise,
        _ => wallpaper.count - wallpaper.sunset + wallpaper.sunrise,
    };
    debug!("image count: {}", image_count);

    let timer_length = duration.num_nanoseconds().unwrap() / image_count;
    debug!(
        "timer length: {}",
        format_duration(Duration::nanoseconds(timer_length))
    );

    let index = get_index(config.now, start, timer_length);

    info!(
        "index: {} of {} ({}%)",
        index,
        image_count,
        index * 100 / image_count
    );

    println!(
        "{}",
        match time_period {
            TimePeriod::DayTime => index + wallpaper.sunrise,
            TimePeriod::AfterSunset => index / 2 + wallpaper.sunset,
            TimePeriod::BeforeSunrise => index,
        }
    );

    Ok(())
}

fn get_index(now: DateTime<Utc>, start: DateTime<Utc>, timer_length: i64) -> i64 {
    let elapsed_time = now - start;
    debug!("elapsed time: {}", format_duration(elapsed_time));
    elapsed_time.num_nanoseconds().unwrap() / timer_length
}

fn init() {
    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .default_format_timestamp(false)
        .init();
    info!("logging enabled");

    dotenv::dotenv().ok();
    info!("dotenv ok ✓");
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
    sunrise: i64,
    sunset: i64,
}

impl Wallpaper {
    fn new() -> Result<Self> {
        use std::env;

        let count = env::var("WALLPAPER_COUNT")
            .with_context(|c| format!("WALLPAPER_COUNT: {}", c))?
            .parse::<u8>()?;
        let sunrise = env::var("WALLPAPER_SUNRISE")
            .with_context(|c| format!("WALLPAPER_SUNRISE: {} ", c))?
            .parse::<u8>()?;
        let sunset = env::var("WALLPAPER_SUNSET")
            .with_context(|c| format!("WALLPAPER_SUNSET: {}", c))?
            .parse::<u8>()
            .with_context(|c| format!("WALLPAPER_SUNSET: {}", c))?;

        Ok(Self {
            count: i64::from(count),
            sunrise: i64::from(sunrise),
            sunset: i64::from(sunset),
        })
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

        //let utc_now = Utc::now();
        let utc_now = now
            .with_timezone(&Local)
            .date()
            .and_hms(12, 0, 0)
            .with_timezone(&Utc);

        debug!("now:          {}", now.with_timezone(&Local));
        debug!("UTC now:      {}", utc_now.with_timezone(&Local));

        debug_assert!(Utc::today() - Duration::days(1) <= now.date());
        debug_assert!(now.date() <= Utc::today() + Duration::days(1));

        let (sunrise, sunset) = match spa::calc_sunrise_and_set(utc_now, lat, lon)? {
            SunriseAndSet::Daylight(sunrise, sunset) => (sunrise, sunset),
            _ => unimplemented!(),
        };
        debug!("sunrise:      {}", sunrise.with_timezone(&Local));
        debug!("sunset:       {}", sunset.with_timezone(&Local));

        let last_sunset = match spa::calc_sunrise_and_set(utc_now - Duration::days(1), lat, lon)? {
            SunriseAndSet::Daylight(_, sunset) => sunset,
            _ => unimplemented!(),
        };
        debug!("last sunset:  {}", last_sunset.with_timezone(&Local));

        let next_sunrise = match spa::calc_sunrise_and_set(utc_now + Duration::days(1), lat, lon)? {
            spa::SunriseAndSet::Daylight(sunrise, _) => sunrise,
            _ => unimplemented!(),
        };
        debug!("next sunrise: {}", next_sunrise.with_timezone(&Local));

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

#[derive(Debug)]
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

impl TimePeriod {
    fn new(now: DateTime<Utc>, sunrise: DateTime<Utc>, sunset: DateTime<Utc>) -> Self {
        if now <= sunrise {
            TimePeriod::BeforeSunrise
        } else if now <= sunset {
            TimePeriod::DayTime
        } else {
            TimePeriod::AfterSunset
        }
    }
}

fn format_duration(duration: Duration) -> String {
    use chrono_humanize::{Accuracy, HumanTime, Tense};
    HumanTime::from(duration).to_text_en(Accuracy::Precise, Tense::Present)
}
