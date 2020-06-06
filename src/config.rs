use super::{Error, Result};
use chrono::{DateTime, Local};
use serde::Deserialize;
use std::convert::TryFrom;
use std::path::PathBuf;

/// Program configuration.
///
/// # Example
/// ```
/// # use dynamic_wallpaper::Config;
/// # let config: Config = toml::from_str(r#"
/// lat = 12.3456
/// lon = -65.4321
///
/// [wallpaper]
/// day_images = 13
/// night_images = 3
/// # "#).expect("Can't parse example config");
/// # config.validate().expect("Example config invalid");
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Current time. Defaults to now.
    ///
    /// Useful for debugging. Needs to be in [rfc3339
    /// format][chrono::DateTime::parse_from_rfc3339], e.g. `2018-08-31T13:45:00-05:00`.
    #[serde(default = "default_time")]
    pub now: DateTime<Local>,

    /// latitude
    pub lat: f64,

    /// longitude
    pub lon: f64,

    /// Wallpaper configuration
    pub wallpaper: Wallpaper,
}

/// Get the current time.
fn default_time() -> DateTime<Local> {
    Local::now()
}

impl Config {
    #[doc(hidden)]
    pub fn validate(&self) -> Result<()> {
        self.wallpaper.validate()?;
        Ok(())
    }
}

impl TryFrom<PathBuf> for Config {
    type Error = Error;

    fn try_from(filename: PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(filename)?;

        let config: Self = toml::from_str(&contents)?;

        config.validate()?;

        Ok(config)
    }
}

/// Wallpaper configuration settings.
#[derive(Debug, Deserialize)]
#[doc(inline)]
pub struct Wallpaper {
    /// Number of images to use during the day.
    pub day_images: u32,

    /// Number of images to use at night.
    pub night_images: u32,
}

impl Wallpaper {
    fn validate(&self) -> Result<()> {
        if self.day_images == 0 || self.night_images == 0 {
            Err(Error::Config(
                "Number of day or night images must be greater than zero.",
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn day_images_zero() {
        let config = Config {
            now: Local::now(),
            lat: 12.34,
            lon: 56.78,
            wallpaper: Wallpaper {
                day_images: 0,
                night_images: 1,
            },
        };
        config.validate().expect("day_images check failed");
    }

    #[test]
    #[should_panic]
    fn night_images_zero() {
        let config = Config {
            now: Local::now(),
            lat: 12.34,
            lon: 56.78,
            wallpaper: Wallpaper {
                day_images: 1,
                night_images: 0,
            },
        };
        config.validate().expect("night_images check failed")
    }
}
