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
        Ok(Config { now, lat, lon })
    }

    fn get_sunset_sunrise(&self) -> Result<(DateTime<Local>, DateTime<Local>)> {
        let daylight = spa::calc_sunrise_and_set(self.now.with_timezone(&Utc), self.lat, self.lon)?;
        match daylight {
            spa::SunriseAndSet::Daylight(sr, ss) => {
                Ok((sr.with_timezone(&Local), ss.with_timezone(&Local)))
            }
            _ => unimplemented!(),
        }
    }
}

pub fn run() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("logging enabled");

    let config = Config::new()?;
    debug!("{:#?}", config);

    let (sunset, sunrise) = config.get_sunset_sunrise()?;
    info!("sunrise: {}", sunrise);
    info!("sunset:  {}", sunset);

    Ok(())
}
