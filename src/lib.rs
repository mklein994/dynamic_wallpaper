extern crate chrono;
extern crate dotenv;
extern crate failure;
extern crate spa;
#[macro_use]
extern crate log;
extern crate chrono_humanize;
extern crate env_logger;

use chrono::{DateTime, Local, Utc};
use std::fmt;

type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Debug)]
struct Config {
    pub lat: f64,
    pub lon: f64,
    pub now: DateTime<Local>,
    sunrise: DateTime<Local>,
    sunset: DateTime<Local>,
    time_period: TimePeriod,
    duration: chrono::Duration,
}

impl Config {
    fn new() -> Result<Self> {
        let now = if let Ok(now_string) = std::env::var("WALLPAPER_NOW") {
            DateTime::parse_from_rfc3339(&now_string)?.with_timezone(&Local)
        } else {
            Local::now()
        };

        let lat = std::env::var("WALLPAPER_LAT")?.parse::<f64>()?;
        let lon = std::env::var("WALLPAPER_LON")?.parse::<f64>()?;

        let (sunrise, sunset) = get_sunset_sunrise(now, lat, lon)?;

        let (time_period, duration) = if now >= sunrise && now <= sunset {
            (TimePeriod::DayTime, sunset - sunrise)
        } else {
            (TimePeriod::NightTime, sunset - sunrise)
        };

        Ok(Config {
            now,
            lat,
            lon,
            sunrise,
            sunset,
            time_period,
            duration,
        })
    }
}

fn get_sunset_sunrise(
    now: DateTime<Local>,
    lat: f64,
    lon: f64,
) -> Result<(DateTime<Local>, DateTime<Local>)> {
    let daylight = spa::calc_sunrise_and_set(now.with_timezone(&Utc), lat, lon)?;
    match daylight {
        spa::SunriseAndSet::Daylight(sr, ss) => {
            Ok((sr.with_timezone(&Local), ss.with_timezone(&Local)))
        }
        _ => unimplemented!(),
    }
}

#[derive(Debug)]
struct Wallpaper {
    count: u8,
    sunrise: u8,
    sunset: u8,
}

#[derive(Debug)]
enum TimePeriod {
    DayTime,
    NightTime,
}

impl fmt::Display for TimePeriod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TimePeriod::DayTime => write!(f, "DayTime ☀️"),
            TimePeriod::NightTime => write!(f, "NightTime 🌙"),
        }
    }
}

impl Wallpaper {
    fn new() -> Result<Self> {
        use std::env;
        Ok(Self {
            count: env::var("WALLPAPER_COUNT")?.parse::<u8>()?,
            sunrise: env::var("WALLPAPER_SUNRISE")?.parse::<u8>()?,
            sunset: env::var("WALLPAPER_SUNSET")?.parse::<u8>()?,
        })
    }

    fn image_count_for_time_period(&self, time: &TimePeriod) -> u8 {
        match time {
            TimePeriod::DayTime => self.sunset - self.sunrise,
            TimePeriod::NightTime => self.count - self.sunset + self.sunrise,
        }
    }
}

pub fn run() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("logging enabled");

    let config = Config::new()?;
    debug!("{:#?}", config);
    info!("{}", config.time_period);
    info!("duration: {}", pretty_duration(config.duration));

    let wallpaper = Wallpaper::new()?;
    debug!("{:#?}", wallpaper);

    let image_step = image_step(config.duration, wallpaper.count);
    info!("image step: {}", pretty_duration(image_step));

    let image_count = wallpaper.image_count_for_time_period(&config.time_period);
    info!("image count: {:?}", image_count);

    Ok(())
}

fn image_step(duration: chrono::Duration, image_count: u8) -> chrono::Duration {
    duration / i32::from(image_count)
}

fn pretty_duration(duration: chrono::Duration) -> String {
    use chrono_humanize::{Accuracy, HumanTime, Tense};
    HumanTime::from(duration).to_text_en(Accuracy::Precise, Tense::Present)
}
