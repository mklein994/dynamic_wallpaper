//! Dynamic Wallpaper
//!
//! Print the index of the image to use depending on the time of day and
//! location. These are set in `~/.config/dynamic_wallpaper/config.toml`.

mod config;
mod error;

pub use self::config::{Config, Wallpaper};
pub use self::error::Error;

use jiff::{tz::TimeZone, Timestamp, ToSpan, Unit, Zoned};
use std::path::PathBuf;

/// Result type alias to handle errors.
type Result<T> = std::result::Result<T, Error>;

/// Main entry point.
pub fn run() -> Result<i64> {
    let config = get_config()?;
    let now = config.now;
    let wallpaper = config.wallpaper;

    let sun = Sun::new(&now, config.lat, config.lon);

    let image = get_image(&now, &sun, &wallpaper);

    Ok(image)
}

fn get_config() -> Result<Config> {
    let filename = std::env::var("DYNAMIC_WALLPAPER_CONFIG").map_or_else(
        |_| {
            dirs::config_dir()
                .expect("Couldn't find $XDG_CONFIG_DIR (~/.config/)")
                .join(env!("CARGO_PKG_NAME"))
                .join("config.toml")
        },
        PathBuf::from,
    );
    let config = Config::try_from(filename)?;
    Ok(config)
}

fn get_image(now: &Zoned, sun: &Sun, wallpaper: &Wallpaper) -> i64 {
    let (sunrise, sunset) = (&sun.sunrise, &sun.sunset);
    let day_duration = sunrise.until(sunset).unwrap();
    let night_duration = 1.day().checked_sub(day_duration).unwrap();

    let day_size = f64::from(wallpaper.day_images.get());
    let night_size = f64::from(wallpaper.night_images.get());

    let time_period = TimePeriod::new(now, sun);

    let index = match time_period {
        TimePeriod::BeforeSunrise => {
            day_size
                + now
                    .tomorrow()
                    .unwrap()
                    .since(sunset)
                    .unwrap()
                    .total(Unit::Second)
                    .unwrap()
                    / (night_duration.total(Unit::Second).unwrap() / night_size)
        }
        TimePeriod::DayTime => {
            now.since(sunrise).unwrap().total(Unit::Second).unwrap()
                / (day_duration.total(Unit::Second).unwrap() / day_size)
        }
        TimePeriod::AfterSunset => {
            day_size
                + now.since(sunset).unwrap().total(Unit::Second).unwrap()
                    / (night_duration.total(Unit::Second).unwrap() / night_size)
        }
    };

    index as i64 + 1
}

/// Sunrise and sunset times.
#[derive(Debug)]
struct Sun {
    /// Today's sunrise.
    sunrise: Zoned,

    /// Today's sunset.
    sunset: Zoned,
}

impl Sun {
    /// Get the time of sunrise and sunset depending on the date and location.
    fn new(date: &Zoned, lat: f64, lon: f64) -> Self {
        let (sunrise, sunset) = {
            let (sunrise, sunset) =
                sunrise::sunrise_sunset(lat, lon, date.year(), date.month(), date.day());
            (
                Timestamp::new(sunrise, 0)
                    .unwrap()
                    .to_zoned(TimeZone::system()),
                Timestamp::new(sunset, 0)
                    .unwrap()
                    .to_zoned(TimeZone::system()),
            )
        };

        Self { sunrise, sunset }
    }
}

/// Time of day according to the sun.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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
    fn new(now: &Zoned, sun: &Sun) -> Self {
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
    use lazy_static::lazy_static;
    use std::num::NonZeroU32;

    lazy_static! {
        static ref SUN: Sun = Sun {
            sunrise: jiff::civil::datetime(2018, 8, 6, 6, 0, 0, 0)
                .to_zoned(TimeZone::system())
                .unwrap(),
            sunset: jiff::civil::datetime(2018, 8, 6, 20, 0, 0, 0)
                .to_zoned(TimeZone::system())
                .unwrap(),
        };
    }

    mod time_period {
        use super::*;

        #[test]
        fn noon() {
            let time_period = TimePeriod::new(
                &jiff::civil::datetime(2018, 8, 6, 12, 0, 0, 0)
                    .to_zoned(TimeZone::system())
                    .unwrap(),
                &SUN,
            );
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn last_midnight() {
            let time_period = TimePeriod::new(
                &jiff::civil::datetime(2018, 8, 6, 0, 0, 0, 0)
                    .to_zoned(TimeZone::system())
                    .unwrap(),
                &SUN,
            );
            assert_eq!(TimePeriod::BeforeSunrise, time_period);
        }

        #[test]
        fn next_midnight() {
            let time_period = TimePeriod::new(
                &jiff::civil::datetime(2018, 8, 7, 0, 0, 0, 0)
                    .to_zoned(TimeZone::system())
                    .unwrap(),
                &SUN,
            );
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
            let time_period =
                TimePeriod::new(&(SUN.sunset.checked_sub(1.nanosecond()).unwrap()), &SUN);
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn just_after_sunset() {
            let time_period =
                TimePeriod::new(&(SUN.sunset.checked_add(1.nanosecond()).unwrap()), &SUN);
            assert_eq!(TimePeriod::AfterSunset, time_period);
        }

        #[test]
        fn just_before_sunrise() {
            let time_period =
                TimePeriod::new(&(SUN.sunrise.checked_sub(1.nanosecond()).unwrap()), &SUN);
            assert_eq!(TimePeriod::BeforeSunrise, time_period);
        }

        #[test]
        fn just_after_sunrise() {
            let time_period =
                TimePeriod::new(&(SUN.sunrise.checked_add(1.nanosecond()).unwrap()), &SUN);
            assert_eq!(TimePeriod::DayTime, time_period);
        }
    }

    mod get_image {
        use super::*;
        use jiff::ToSpan;

        lazy_static! {
            static ref WALLPAPER: Wallpaper = Wallpaper {
                day_images: NonZeroU32::new(13).unwrap(),
                night_images: NonZeroU32::new(3).unwrap(),
            };
        }

        #[test]
        fn sunrise() {
            let image = get_image(&SUN.sunrise, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn sunset() {
            let image = get_image(&SUN.sunset, &SUN, &WALLPAPER);
            assert_eq!(14, image);
        }

        #[test]
        fn after_sunrise() {
            let now = SUN.sunrise.checked_add(1.hour()).unwrap();
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn just_past_sunrise() {
            let now = SUN.sunrise.checked_add(1.nanosecond()).unwrap();
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn before_sunrise() {
            let now = SUN.sunrise.checked_sub(1.hour()).unwrap();
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(16, image);
        }

        #[test]
        fn just_before_sunrise() {
            let now = SUN.sunrise.checked_sub(1.nanosecond()).unwrap();
            debug_assert!(now < SUN.sunrise);
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(16, image);
        }

        #[test]
        fn before_sunset() {
            let now = SUN.sunset.checked_sub(1.hour()).unwrap();
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(13, image);
        }

        #[test]
        fn just_before_sunset() {
            let now = SUN.sunset.checked_sub(1.nanosecond()).unwrap();
            debug_assert!(now < SUN.sunset);
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(13, image);
        }

        #[test]
        fn past_sunset() {
            let now = SUN.sunset.checked_add(1.hour()).unwrap();
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(14, image);
        }

        #[test]
        fn just_past_sunset() {
            let now = SUN.sunset.checked_add(1.nanosecond()).unwrap();
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(14, image);
        }
    }

    mod firewatch {
        use super::*;
        use jiff::{ToSpan, Unit};

        lazy_static! {
            static ref WALLPAPER: Wallpaper = Wallpaper {
                day_images: NonZeroU32::new(3).unwrap(),
                night_images: NonZeroU32::new(1).unwrap(),
            };
        }

        #[test]
        fn before_sunrise() {
            let now = SUN.sunrise.checked_sub(1.hour()).unwrap();

            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(4, image);
        }

        #[test]
        fn sunrise() {
            let now = &SUN.sunrise;

            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn after_sunrise() {
            let now = SUN.sunrise.checked_add(1.hour()).unwrap();

            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn solar_noon() {
            let diff_nanos = SUN
                .sunset
                .since(&SUN.sunrise)
                .unwrap()
                .total(Unit::Nanosecond)
                .unwrap();
            let half_nanos = ((diff_nanos as i64) / 2).nanoseconds();
            let now = SUN.sunrise.checked_add(half_nanos).unwrap();
            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(2, image);
        }

        #[test]
        fn before_sunset() {
            let now = SUN.sunset.checked_sub(1.hour()).unwrap();

            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(3, image);
        }

        #[test]
        fn sunset() {
            let now = &SUN.sunset;

            let image = get_image(now, &SUN, &WALLPAPER);
            assert_eq!(4, image);
        }

        #[test]
        fn after_sunset() {
            let now = SUN.sunset.checked_add(1.hour()).unwrap();

            let image = get_image(&now, &SUN, &WALLPAPER);
            assert_eq!(4, image);
        }
    }
}
