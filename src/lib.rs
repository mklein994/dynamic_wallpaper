#[macro_use]
extern crate log;
extern crate chrono;
extern crate chrono_humanize;
extern crate dotenv;
extern crate env_logger;
extern crate failure;
extern crate spa;

use chrono::{DateTime, Local, Utc};
use failure::ResultExt;
use std::fmt;

const FORMAT: &str = "%F %l:%M:%S %P (%Z)";

type Result<T> = std::result::Result<T, failure::Error>;

pub fn run() -> Result<()> {
    init();

    let config = Config::new()?;
    info!("config set ✓");
    debug!("{:#?}", config);

    let wallpaper = Wallpaper::new()?;
    debug!("{:?}", wallpaper);

    let sun = Sun::new(config.now, config.lat, config.lon)?;
    debug!("{}", sun);

    let time_period = get_time_period(config.now, sun.sunrise, sun.sunset);
    info!("{}", time_period);

    let duration = match time_period {
        TimePeriod::BeforeSunrise => {
            let duration = sun.sunrise - sun.last_sunset;
            info!(
                "duration from last sunset to sunrise: {}",
                format_duration(duration)
            );
            duration
        }
        TimePeriod::AfterSunset => {
            let duration = sun.next_sunrise - sun.sunset;
            info!(
                "duration from sunset to next sunrise: {}",
                format_duration(duration),
            );
            duration
        }
        TimePeriod::DayTime => {
            let duration = sun.sunset - sun.sunrise;
            info!(
                "duration from sunrise to sunset: {}",
                format_duration(duration),
            );
            duration
        }
    };
    debug!("duration: {}", format_duration(duration));

    let count = match time_period {
        TimePeriod::DayTime => wallpaper.sunset - wallpaper.sunrise,
        _ => wallpaper.count - wallpaper.sunset + wallpaper.sunrise,
    };
    debug!("image count: {}", count);

    // TODO: fix this. Everything else works.
    let index = match time_period {
        TimePeriod::DayTime => ((config.now - sun.sunrise).num_seconds()) % i64::from(count),
        TimePeriod::BeforeSunrise => ((config.now - sun.last_sunset).num_seconds()) % i64::from(count),
        TimePeriod::AfterSunset => ((config.now - sun.sunset).num_seconds()) % i64::from(count),
    };
    debug!("index: {:?}", index);

    Ok(())
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

fn get_time_period(
    now: DateTime<Local>,
    sunrise: DateTime<Local>,
    sunset: DateTime<Local>,
) -> TimePeriod {
    use chrono::Timelike;
    let now = now.num_seconds_from_midnight();
    let sunrise = sunrise.num_seconds_from_midnight();
    let sunset = sunset.num_seconds_from_midnight();
    if now > sunset {
        TimePeriod::AfterSunset
    } else if now < sunrise {
        TimePeriod::BeforeSunrise
    } else {
        TimePeriod::DayTime
    }
}

fn format_duration(d: chrono::Duration) -> String {
    use chrono_humanize::{Accuracy, HumanTime, Tense};

    HumanTime::from(d).to_text_en(Accuracy::Precise, Tense::Present)
}

#[derive(Debug)]
struct Config {
    now: DateTime<Local>,
    lat: f64,
    lon: f64,
}

impl Config {
    fn new() -> Result<Self> {
        use std::env;

        let now = if let Ok(n) = env::var("WALLPAPER_NOW") {
            DateTime::parse_from_rfc3339(&n)?.with_timezone(&Local)
        } else {
            Local::now()
        };

        let lat = env::var("WALLPAPER_LAT")
            .context("WALLPAPER_LAT is not set.")?
            .parse::<f64>()?;
        let lon = env::var("WALLPAPER_LON")
            .context("WALLPAPER_LON is not set.")?
            .parse::<f64>()?;

        debug!("now :: {}", now.format(FORMAT));

        Ok(Self { now, lat, lon })
    }
}

#[derive(Debug)]
struct Wallpaper {
    count: u8,
    sunrise: u8,
    sunset: u8,
}

impl Wallpaper {
    fn new() -> Result<Self> {
        use std::env;

        let count = env::var("WALLPAPER_COUNT")
            .context("WALLPAPER_COUNT not set.")?
            .parse::<u8>()?;
        let sunrise = env::var("WALLPAPER_SUNRISE")
            .context("WALLPAPER_SUNRISE not set.")?
            .parse::<u8>()?;
        let sunset = env::var("WALLPAPER_SUNSET")
            .context("WALLPAPER_SUNSET not set.")?
            .parse::<u8>()?;

        Ok(Self {
            count,
            sunrise,
            sunset,
        })
    }
}

#[derive(Debug)]
struct Sun {
    last_sunset: DateTime<Local>,
    sunrise: DateTime<Local>,
    sunset: DateTime<Local>,
    next_sunrise: DateTime<Local>,
}

impl Sun {
    fn new(now: DateTime<Local>, lat: f64, lon: f64) -> Result<Self> {
        use chrono::Duration;

        let (sunrise, sunset) = match spa::calc_sunrise_and_set(now.with_timezone(&Utc), lat, lon)?
        {
            spa::SunriseAndSet::Daylight(sunrise, sunset) => {
                (sunrise.with_timezone(&Local), sunset.with_timezone(&Local))
            }
            _ => unimplemented!(),
        };

        let last_sunset = match spa::calc_sunrise_and_set(
            (now.date().and_hms(12, 0, 0) - Duration::hours(24)).with_timezone(&Utc),
            lat,
            lon,
        )? {
            spa::SunriseAndSet::Daylight(_, sunset) => sunset.with_timezone(&Local),
            _ => unimplemented!(),
        };

        let next_sunrise = match spa::calc_sunrise_and_set(
            (now.date().and_hms(12, 0, 0) + Duration::hours(24)).with_timezone(&Utc),
            lat,
            lon,
        )? {
            spa::SunriseAndSet::Daylight(sunrise, _) => sunrise.with_timezone(&Local),
            _ => unimplemented!(),
        };

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
            "-".repeat(45),
            "last sunset:",
            self.last_sunset.format(FORMAT),
            "sunrise:",
            self.sunrise.format(FORMAT),
            "sunset:",
            self.sunset.format(FORMAT),
            "next sunrise:",
            self.next_sunrise.format(FORMAT)
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
