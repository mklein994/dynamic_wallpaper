use super::{Error, Result};
use jiff::Zoned;
use serde::Deserialize;
use std::num::NonZeroU32;
use std::{path::PathBuf, str::FromStr};

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
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Current time. Defaults to the current system time.
    ///
    /// Useful for debugging. Needs to be in [Temporal ISO 8601 grammar](https://tc39.es/proposal-temporal/#sec-temporal-iso8601grammar), e.g. `2020-08-21T02:21:58-04[America/New_York]`.
    #[serde(default = "default_time")]
    pub now: Zoned,

    /// latitude
    pub lat: f64,

    /// longitude
    pub lon: f64,

    /// Wallpaper configuration
    pub wallpaper: Wallpaper,
}

impl Config {
    /// Get the default path to the config file.
    #[must_use]
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap()
            .join(env!("CARGO_PKG_NAME"))
            .join("config.toml")
    }
}

/// Get the current time.
fn default_time() -> Zoned {
    Zoned::try_from(std::time::SystemTime::now()).unwrap()
}

impl TryFrom<PathBuf> for Config {
    type Error = Error;

    fn try_from(filename: PathBuf) -> Result<Self> {
        std::fs::read_to_string(filename)?
            .parse()
            .map_err(Error::from)
    }
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

/// Wallpaper configuration settings.
#[derive(Debug, Deserialize)]
pub struct Wallpaper {
    /// Number of images to use during the day.
    ///
    /// These should be numbered chronologically, starting from 1.
    pub day_images: NonZeroU32,

    /// Number of images to use at night.
    ///
    /// These should be numbered after the day images, in chronological order.
    pub night_images: NonZeroU32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_with_day_images_zero() {
        let config = r"
            lat = 12.34
            lon = -98.76

            [wallpaper]
            day_images = 0
            night_images = 12
        ";

        assert!(matches!(
            config.parse::<Config>().unwrap_err(),
            toml::de::Error { .. }
        ));
    }
}
