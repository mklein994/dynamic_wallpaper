//! Dynamic Wallpaper
//!
//! Print the index of the image to use depending on the time of day and
//! location. These are set in `~/.config/dynamic_wallpaper/config.toml`.

mod config;
mod error;

pub use self::config::{Config, Wallpaper};
pub use self::error::Error;

use chrono::{Date, DateTime, Datelike, Duration, Local, TimeZone};
use std::convert::TryFrom;
use std::path::PathBuf;

/// Result type alias to handle errors.
type Result<T> = std::result::Result<T, Error>;

/// Main entry point.
pub fn run() -> Result<i64> {
    let filename = std::env::var("DYNAMIC_WALLPAPER_CONFIG").map_or_else(
        |_| {
            dirs::config_dir()
                .expect("Couldn't find $XDG_CONFIG_DIR (~/.config/)")
                .join("dynamic_wallpaper")
                .join("config.toml")
        },
        PathBuf::from,
    );

    let config = Config::try_from(filename)?;
    let now = config.now;
    let wallpaper = config.wallpaper;

    let sun = Sun::new(now.date(), config.lat, config.lon)?;

    let image = get_image(now, &sun, &wallpaper);

    Ok(image)
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

/// Sunrise and sunset times.
#[derive(Debug)]
struct Sun {
    /// Today's sunrise.
    sunrise: DateTime<Local>,

    /// Today's sunset.
    sunset: DateTime<Local>,
}

impl Sun {
    /// Get the time of sunrise and sunset depending on the date and location.
    fn new(date: Date<Local>, lat: f64, lon: f64) -> Result<Self> {
        let (sunrise, sunset) = {
            let (sunrise, sunset) =
                sunrise::sunrise_sunset(lat, lon, date.year(), date.month(), date.day());
            (Local.timestamp(sunrise, 0), Local.timestamp(sunset, 0))
        };

        Ok(Self { sunrise, sunset })
    }
}

/// Time of day according to the sun.
#[derive(Debug, PartialEq, Copy, Clone)]
enum TimePeriod {
    /// Before the sun has risen.
    ///
    /// Marks time starting at midnight up to but not including sunrise.
    ///
    /// interval: `[midnight, sunrise)`
    BeforeSunrise,

    /// Time from sunrise to sunset, inclusive.
    ///
    /// interval: `[sunrise, sunset]`
    DayTime,

    /// After the sun has set.
    ///
    /// Marks time starting just after sunset up to but not including midnight.
    ///
    /// interval: `(sunset, midnight)`
    AfterSunset,
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
    use lazy_static::lazy_static;

    lazy_static! {
        static ref SUN: Sun = Sun {
            sunrise: Local.ymd(2018, 8, 6).and_hms(6, 0, 0),
            sunset: Local.ymd(2018, 8, 6).and_hms(20, 0, 0),
        };
    }

    mod time_period {
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

    mod get_image {
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

    mod firewatch {
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
