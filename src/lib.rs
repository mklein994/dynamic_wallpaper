extern crate chrono;
extern crate dotenv;
extern crate spa;
extern crate failure;

use chrono::{DateTime, Utc};

type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Debug)]
pub struct Config {
    pub lat: f64,
    pub lon: f64,
    pub now: DateTime<Utc>,
}

pub fn run() -> Result<()> {
    let config = get_config()?;
    main(&config)
}

pub fn main(config: &Config) -> Result<()> {
    let sun_position = spa::calc_solar_position(config.now, config.lat, config.lon)?;
    println!("{:#?}", sun_position);
    let sun_time = spa::calc_sunrise_and_set(config.now, config.lat, config.lon)?;
    println!("{:#?}", sun_time);
    Ok(())
}

pub fn get_config() -> Result<Config> {
    dotenv::dotenv().ok();
    let now = Utc::now();
    let lat = std::env::var("WALLPAPER_LAT")?.parse::<f64>()?;
    let lon = std::env::var("WALLPAPER_LON")?.parse::<f64>()?;
    Ok(Config { now, lat, lon })
}
