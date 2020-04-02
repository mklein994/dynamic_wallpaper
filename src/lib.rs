//! Dynamic Wallpaper
//!
//! Print the index of the image to use depending on the time of day and
//! location. These are set in `~/.config/dynamic_wallpaper/config.toml`.

mod error;

#[cfg(test)]
use lazy_static::lazy_static;

use self::error::Error;

use chrono::{DateTime, Duration, Local, Utc};
use serde::Deserialize;
use std::convert::TryFrom;
use std::path::PathBuf;

/// Result type alias to handle errors.
type Result<T> = std::result::Result<T, Error>;

/// Main entry point.
pub fn run() -> Result<()> {
    let filename = dirs::config_dir()
        .expect("Couldn't find $XDG_CONFIG_DIR (~/.config/)")
        .join("dynamic_wallpaper")
        .join("config.toml");

    let config = Config::try_from(filename)?;
    let now = config.now;
    let wallpaper = config.wallpaper;

    let sun = Sun::new(now, config.lat, config.lon)?;

    let image = get_image(now, &sun, &wallpaper);

    println!("{}", image);

    Ok(())
}

fn get_image(now: DateTime<Local>, sun: &Sun, wallpaper: &Wallpaper) -> i64 {
    let (sunrise, sunset) = (sun.sunrise, sun.sunset);
    let day_duration = sunset - sunrise;
    let night_duration = Duration::days(1) - day_duration;

    let day_size = f64::from(wallpaper.day_images);
    let night_size = f64::from(wallpaper.night_images);

    let time_period = TimePeriod::new(&now, &sun);

    let index = match time_period {
        TimePeriod::BeforeSunrise => {
            day_size
                + (now + Duration::days(1) - sunset).num_seconds() as f64
                    / (night_duration.num_seconds() as f64 / night_size)
        }
        TimePeriod::DayTime => {
            (now - sunrise).num_seconds() as f64 / (day_duration.num_seconds() as f64 / day_size)
        }
        TimePeriod::AfterSunset => {
            day_size
                + (now - sunset).num_seconds() as f64
                    / (night_duration.num_seconds() as f64 / night_size)
        }
    };

    index as i64 + 1
}

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
    /// Needs to be in rfc3339 format, e.g. `2018-08-31T13:45:00-05:00`. See
    /// [here](chrono::DateTime::parse_from_rfc3339) for details.
    #[serde(default = "default_time")]
    pub now: DateTime<Local>,

    /// latitude
    pub lat: f64,

    /// longitude
    pub lon: f64,

    /// Wallpaper configuration
    ///
    /// Defaults to Mojave wallpaper.
    #[serde(default)]
    pub wallpaper: Wallpaper,
}

/// Get the current time in UTC.
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

    /// Try to read a config file from `~/.config/dynamic_wallpaper/config.toml`.
    fn try_from(filename: PathBuf) -> Result<Self> {
        let contents = std::fs::read_to_string(filename)?;

        let config: Self = toml::from_str(&contents)?;

        config.validate()?;

        Ok(config)
    }
}

/// Wallpaper configuration settings.
#[derive(Debug, Deserialize)]
pub struct Wallpaper {
    /// Number of images to use during the day.
    pub day_images: u32,

    /// Number of images to use at night.
    pub night_images: u32,
}

impl Wallpaper {
    fn validate(&self) -> Result<()> {
        if self.day_images == 0 || self.night_images == 0 {
            return Err(Error::Config(
                "Number of day or night images must be greater than zero.",
            ));
        }

        Ok(())
    }
}

impl Default for Wallpaper {
    fn default() -> Self {
        Self {
            day_images: 13,
            night_images: 3,
        }
    }
}

/// Sunrise and sunset times for yesterday, today and tomorrow.
#[derive(Debug)]
struct Sun {
    /// Today's sunrise.
    sunrise: DateTime<Local>,

    /// Today's sunset.
    sunset: DateTime<Local>,
}

impl Sun {
    /// Get the sunrise and sunset times depending on the current time and location.
    fn new(now: DateTime<Local>, lat: f64, lon: f64) -> Result<Self> {
        use spa::SunriseAndSet;

        // Ensure that the time we use to calculate yesterday's sunset and tomorrow's sunrise is at
        // noon today before converting to UTC. The goal is to use a time in `TimePeriod::DayTime`
        // to calculate with.
        //
        // If we didn't do this, converting to UTC might change the date and get the wrong sunrise
        // and sunset times.
        let noon_today = now.date().and_hms(12, 0, 0).with_timezone(&Utc);

        let (sunrise, sunset) = match spa::calc_sunrise_and_set(noon_today, lat, lon)? {
            SunriseAndSet::Daylight(sunrise, sunset) => {
                (sunrise.with_timezone(&Local), sunset.with_timezone(&Local))
            }
            _ => unimplemented!(),
        };

        Ok(Self { sunrise, sunset })
    }
}

/// Time of day according to the sun.
#[derive(Debug, PartialEq, Copy, Clone)]
enum TimePeriod {
    /// After the sun has set, including sunset itself.
    ///
    /// Marks time starting at sunset until but not including midnight.
    AfterSunset,

    /// Before the sun has risen.
    ///
    /// Marks time starting at midnight until but not including sunrise.
    BeforeSunrise,

    /// Between sunrise and sunset.
    ///
    /// Marks time starting at sunrise until but not including sunset.
    DayTime,
}

impl TimePeriod {
    /// Determine the time period depending on the given time and the times for
    /// sunrise and sunset.
    fn new(now: &DateTime<Local>, sun: &Sun) -> Self {
        if *now > sun.sunset {
            Self::AfterSunset
        } else if *now >= sun.sunrise {
            Self::DayTime
        } else {
            Self::BeforeSunrise
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    lazy_static! {
        static ref SUN: Sun = Sun {
            sunrise: Local.ymd(2018, 8, 6).and_hms(6, 0, 0),
            sunset: Local.ymd(2018, 8, 6).and_hms(20, 0, 0),
        };
    }

    mod time_period_tests {
        use super::*;

        #[test]
        fn noon() {
            let time_period = TimePeriod::new(&Local.ymd(2018, 8, 6).and_hms(12, 0, 0), &SUN);
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn last_midnight() {
            let time_period = TimePeriod::new(&Local.ymd(2018, 8, 6).and_hms(0, 0, 0), &SUN);
            assert_eq!(TimePeriod::BeforeSunrise, time_period);
        }

        #[test]
        fn next_midnight() {
            let time_period = TimePeriod::new(&Local.ymd(2018, 8, 7).and_hms(0, 0, 0), &SUN);
            assert_eq!(TimePeriod::AfterSunset, time_period);
        }

        #[test]
        fn sunrise() {
            let time_period = TimePeriod::new(&SUN.sunrise, &SUN);
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn sunset() {
            let time_period = TimePeriod::new(&SUN.sunset, &SUN);
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn just_before_sunset() {
            let time_period = TimePeriod::new(&(SUN.sunset - Duration::nanoseconds(1)), &SUN);
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn just_after_sunset() {
            let time_period = TimePeriod::new(&(SUN.sunset + Duration::nanoseconds(1)), &SUN);
            assert_eq!(TimePeriod::AfterSunset, time_period);
        }

        #[test]
        fn just_before_sunrise() {
            let time_period = TimePeriod::new(&(SUN.sunrise - Duration::nanoseconds(1)), &SUN);
            assert_eq!(TimePeriod::BeforeSunrise, time_period);
        }

        #[test]
        fn just_after_sunrise() {
            let time_period = TimePeriod::new(&(SUN.sunrise + Duration::nanoseconds(1)), &SUN);
            assert_eq!(TimePeriod::DayTime, time_period);
        }
    }

    mod get_image_tests {
        use super::*;

        const WALLPAPER: Wallpaper = Wallpaper {
            day_images: 13,
            night_images: 3,
        };

        #[test]
        fn sunrise() {
            let image = get_image(SUN.sunrise, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn sunset() {
            let image = get_image(SUN.sunset, &SUN, &WALLPAPER);
            assert_eq!(14, image);
        }

        #[test]
        fn after_sunrise() {
            let now = SUN.sunrise + Duration::hours(1);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn just_past_sunrise() {
            let now = SUN.sunrise + Duration::nanoseconds(1);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn before_sunrise() {
            let now = SUN.sunrise - Duration::hours(1);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(16, image);
        }

        #[test]
        fn just_before_sunrise() {
            let now = SUN.sunrise - Duration::nanoseconds(1);
            debug_assert!(now < SUN.sunrise);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(16, image);
        }

        #[test]
        fn before_sunset() {
            let now = SUN.sunset - Duration::hours(1);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(13, image);
        }

        #[test]
        fn just_before_sunset() {
            let now = SUN.sunset - Duration::nanoseconds(1);
            debug_assert!(now < SUN.sunset);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(13, image);
        }

        #[test]
        fn past_sunset() {
            let now = SUN.sunset + Duration::hours(1);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(14, image);
        }

        #[test]
        fn just_past_sunset() {
            let now = SUN.sunset + Duration::nanoseconds(1);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(14, image);
        }
    }

    mod firewatch_tests {
        use super::*;

        const WALLPAPER: Wallpaper = Wallpaper {
            day_images: 3,
            night_images: 1,
        };

        #[test]
        fn before_sunrise() {
            let now = SUN.sunrise - Duration::hours(1);

            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(4, image);
        }

        #[test]
        fn sunrise() {
            let now = SUN.sunrise;

            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn after_sunrise() {
            let now = SUN.sunrise + Duration::hours(1);

            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn solar_noon() {
            let now = SUN.sunrise
                + Duration::nanoseconds((SUN.sunset - SUN.sunrise).num_nanoseconds().unwrap() / 2);
            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(2, image);
        }

        #[test]
        fn before_sunset() {
            let now = SUN.sunset - Duration::hours(1);

            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(3, image);
        }

        #[test]
        fn sunset() {
            let now = SUN.sunset;

            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(4, image);
        }

        #[test]
        fn after_sunset() {
            let now = SUN.sunset + Duration::hours(1);

            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(4, image);
        }
    }
}
