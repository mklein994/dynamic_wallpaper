extern crate chrono;
extern crate dotenv;
extern crate failure;
extern crate spa;
#[macro_use]
extern crate log;
extern crate env_logger;

use chrono::{DateTime, Local, Utc};

type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Debug)]
struct Config {
    pub lat: f64,
    pub lon: f64,
    pub now: DateTime<Local>,
    sunrise: DateTime<Local>,
    sunset: DateTime<Local>,
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

        let is_daylight = now >= sunrise && now <= sunset;

        let duration = if is_daylight {
            sunset - sunrise
        } else {
            sunrise - sunset
        };

        Ok(Config {
            now,
            lat,
            lon,
            sunrise,
            sunset,
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

impl Wallpaper {
    fn new() -> Result<Self> {
        use std::env;
        Ok(Self {
            count: env::var("WALLPAPER_COUNT")?.parse::<u8>()?,
            sunrise: env::var("WALLPAPER_SUNRISE")?.parse::<u8>()?,
            sunset: env::var("WALLPAPER_SUNSET")?.parse::<u8>()?,
        })
    }
}

pub fn run() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("logging enabled");

    let config = Config::new()?;
    debug!("{:#?}", config);

    let wallpaper = Wallpaper::new()?;
    debug!("{:#?}", wallpaper);

    Ok(())
}
