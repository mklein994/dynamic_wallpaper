//! Dynamic Wallpaper
//!
//! Print the index of the image to use depending on the time of day and
//! location. These are set in `~/.config/dynamic_wallpaper/config.toml`.

use serde::Deserialize;

mod error;

#[cfg(test)]
use lazy_static::lazy_static;

use self::error::Error;

use chrono::{DateTime, Duration, Local, Utc};
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

    let time_period = TimePeriod::new(&now, &sun.sunrise, &sun.sunset);

    let image = get_image(now, &sun, time_period, &wallpaper);

    println!("{}", image);

    Ok(())
}

/// Get the image index for the current time, within the time period, from the `image_count` number
/// of images.
fn get_image(
    now: DateTime<Local>,
    sun: &Sun,
    time_period: TimePeriod,
    wallpaper: &Wallpaper,
) -> i64 {
    let offset = wallpaper.offset(time_period);

    let image_count = wallpaper.image_count(time_period);

    let (dawn, dusk) = (sun.sunrise, sun.sunset);
    let duration = (dusk - dawn).num_nanoseconds().unwrap();
    let elapsed_time = (now - dawn).num_nanoseconds().unwrap();

    //  elapsed_time
    // ━━━━━━━━━━━━━━━
    //  (end - start)
    //  ─────────────
    //   image_count
    (offset + elapsed_time * image_count / duration) % wallpaper.count
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
/// count = 16
/// daybreak = 2
/// nightfall = 13
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
    /// Number of images to cycle through.
    pub count: i64,

    /// Image index to use at the beginning of day time.
    pub daybreak: i64,

    /// Image index to use at the beginning of night time.
    pub nightfall: i64,
}

impl Wallpaper {
    fn validate(&self) -> Result<()> {
        if self.daybreak > self.nightfall {
            return Err(Error::Config("daybreak needs to be greater than nightfall"));
        }

        if self.daybreak > self.count || self.nightfall > self.count {
            return Err(Error::Config("wallpaper.count needs to be larger than wallpaper.daybreak and wallpaper.nightfall"));
        }

        Ok(())
    }

    /// Number of images for the time period.
    fn image_count(&self, time_period: TimePeriod) -> i64 {
        if time_period == TimePeriod::DayTime {
            self.nightfall - self.daybreak
        } else {
            i64::abs(self.count - self.nightfall + self.daybreak) % self.count
        }
    }

    /// Index to use as the start of the phase (daybreak or nightfall).
    fn offset(&self, time_period: TimePeriod) -> i64 {
        match time_period {
            TimePeriod::DayTime => self.daybreak,
            _ => self.nightfall,
        }
    }
}

impl Default for Wallpaper {
    fn default() -> Self {
        Self {
            count: 16,
            daybreak: 2,
            nightfall: 13,
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
    fn new(now: &DateTime<Local>, sunrise: &DateTime<Local>, sunset: &DateTime<Local>) -> Self {
        if *now > *sunset {
            Self::AfterSunset
        } else if *now >= *sunrise {
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

    mod image_count_tests {
        use super::*;

        const WALLPAPER: Wallpaper = Wallpaper {
            count: 16,
            daybreak: 2,
            nightfall: 13,
        };

        #[test]
        fn daytime() {
            let image_count = WALLPAPER.image_count(TimePeriod::DayTime);
            assert_eq!(11, image_count);
        }

        #[test]
        fn before_sunrise() {
            let image_count = WALLPAPER.image_count(TimePeriod::BeforeSunrise);
            assert_eq!(5, image_count);
        }

        #[test]
        fn after_sunset() {
            let image_count = WALLPAPER.image_count(TimePeriod::AfterSunset);
            assert_eq!(5, image_count);
        }

        #[test]
        fn daytime_nightfall_is_count() {
            let wallpaper = Wallpaper {
                count: 16,
                daybreak: 3,
                nightfall: 16,
            };
            let image_count = wallpaper.image_count(TimePeriod::DayTime);
            assert_eq!(13, image_count);
        }

        #[test]
        fn before_sunrise_nightfall_is_count() {
            let wallpaper = Wallpaper {
                count: 16,
                daybreak: 3,
                nightfall: 16,
            };
            let image_count = wallpaper.image_count(TimePeriod::BeforeSunrise);
            assert_eq!(3, image_count);
        }

        #[test]
        fn after_sunset_nightfall_is_count() {
            let wallpaper = Wallpaper {
                count: 16,
                daybreak: 3,
                nightfall: 16,
            };
            let image_count = wallpaper.image_count(TimePeriod::AfterSunset);
            assert_eq!(3, image_count);
        }
    }

    mod time_period_tests {
        use super::*;

        #[test]
        fn noon() {
            let time_period = TimePeriod::new(
                &Local.ymd(2018, 8, 6).and_hms(12, 0, 0),
                &SUN.sunrise,
                &SUN.sunset,
            );
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn last_midnight() {
            let time_period = TimePeriod::new(
                &Local.ymd(2018, 8, 6).and_hms(0, 0, 0),
                &SUN.sunrise,
                &SUN.sunset,
            );
            assert_eq!(TimePeriod::BeforeSunrise, time_period);
        }

        #[test]
        fn next_midnight() {
            let time_period = TimePeriod::new(
                &Local.ymd(2018, 8, 7).and_hms(0, 0, 0),
                &SUN.sunrise,
                &SUN.sunset,
            );
            assert_eq!(TimePeriod::AfterSunset, time_period);
        }

        #[test]
        fn sunrise() {
            let time_period = TimePeriod::new(&SUN.sunrise, &SUN.sunrise, &SUN.sunset);
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn sunset() {
            let time_period = TimePeriod::new(&SUN.sunset, &SUN.sunrise, &SUN.sunset);
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn just_before_sunset() {
            let time_period = TimePeriod::new(
                &(SUN.sunset - Duration::nanoseconds(1)),
                &SUN.sunrise,
                &SUN.sunset,
            );
            assert_eq!(TimePeriod::DayTime, time_period);
        }

        #[test]
        fn just_after_sunset() {
            let time_period = TimePeriod::new(
                &(SUN.sunset + Duration::nanoseconds(1)),
                &SUN.sunrise,
                &SUN.sunset,
            );
            assert_eq!(TimePeriod::AfterSunset, time_period);
        }

        #[test]
        fn just_before_sunrise() {
            let time_period = TimePeriod::new(
                &(SUN.sunrise - Duration::nanoseconds(1)),
                &SUN.sunrise,
                &SUN.sunset,
            );
            assert_eq!(TimePeriod::BeforeSunrise, time_period);
        }

        #[test]
        fn just_after_sunrise() {
            let time_period = TimePeriod::new(
                &(SUN.sunrise + Duration::nanoseconds(1)),
                &SUN.sunrise,
                &SUN.sunset,
            );
            assert_eq!(TimePeriod::DayTime, time_period);
        }
    }

    mod get_image_tests {
        use super::*;

        const WALLPAPER: Wallpaper = Wallpaper {
            count: 16,
            daybreak: 2,
            nightfall: 13,
        };

        #[test]
        fn sunrise() {
            let image = get_image(SUN.sunrise, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(WALLPAPER.daybreak, image);
        }

        #[test]
        fn sunset() {
            let image = get_image(SUN.sunset, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(WALLPAPER.nightfall, image);
        }

        #[test]
        fn after_sunrise() {
            let now = SUN.sunrise + Duration::hours(1);
            let image = get_image(now, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(2, image);
        }

        #[test]
        fn just_past_sunrise() {
            let now = SUN.sunrise + Duration::nanoseconds(1);
            let image = get_image(now, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(2, image);
        }

        #[test]
        fn before_sunrise() {
            let now = SUN.sunrise - Duration::hours(1);
            let image = get_image(now, &SUN, TimePeriod::BeforeSunrise, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn just_before_sunrise() {
            let now = SUN.sunrise - Duration::nanoseconds(1);
            debug_assert!(now < SUN.sunrise);
            let image = get_image(now, &SUN, TimePeriod::BeforeSunrise, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[test]
        fn before_sunset() {
            let now = SUN.sunset - Duration::hours(1);
            let image = get_image(now, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(12, image);
        }

        #[test]
        fn just_before_sunset() {
            let now = SUN.sunset - Duration::nanoseconds(1);
            debug_assert!(now < SUN.sunset);
            let image = get_image(now, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(12, image);
        }

        #[test]
        fn past_sunset() {
            let now = SUN.sunset + Duration::hours(1);
            let image = get_image(now, &SUN, TimePeriod::AfterSunset, &WALLPAPER);
            assert_eq!(13, image);
        }

        #[test]
        fn just_past_sunset() {
            let now = SUN.sunset + Duration::nanoseconds(1);
            let image = get_image(now, &SUN, TimePeriod::AfterSunset, &WALLPAPER);
            assert_eq!(13, image);
        }
    }

    mod offset_tests {
        use super::*;

        const WALLPAPER: Wallpaper = Wallpaper {
            count: 10,
            daybreak: 2,
            nightfall: 7,
        };

        #[test]
        fn daytime() {
            assert_eq!(WALLPAPER.daybreak, WALLPAPER.offset(TimePeriod::DayTime));
        }

        #[test]
        fn after_sunset() {
            assert_eq!(
                WALLPAPER.nightfall,
                WALLPAPER.offset(TimePeriod::AfterSunset)
            );
        }

        #[test]
        fn before_sunrise() {
            assert_eq!(
                WALLPAPER.nightfall,
                WALLPAPER.offset(TimePeriod::BeforeSunrise)
            );
        }
    }

    // FIXME
    // These tests fail because the logic in get_image() calls 4.01 % 4.0 which rounds to 0, but it
    // should return 1.0.
    mod firewatch_tests {
        use super::*;

        const WALLPAPER: Wallpaper = Wallpaper {
            count: 4,
            daybreak: 1,
            nightfall: 4,
        };

        #[ignore]
        #[test]
        fn before_sunrise() {
            let now = SUN.sunrise - Duration::hours(1);

            let image = get_image(now, &SUN, TimePeriod::BeforeSunrise, &WALLPAPER);
            assert_eq!(4, image);
        }

        #[ignore]
        #[test]
        fn sunrise() {
            let now = SUN.sunrise;

            let image = get_image(now, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[ignore]
        #[test]
        fn after_sunrise() {
            let now = SUN.sunrise + Duration::hours(1);

            let image = get_image(now, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(1, image);
        }

        #[ignore]
        #[test]
        fn solar_noon() {
            let now = SUN.sunrise
                + Duration::nanoseconds((SUN.sunset - SUN.sunrise).num_nanoseconds().unwrap() / 2);
            let image = get_image(now, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(2, image);
        }

        #[ignore]
        #[test]
        fn before_sunset() {
            let now = SUN.sunset - Duration::hours(1);

            let image = get_image(now, &SUN, TimePeriod::DayTime, &WALLPAPER);
            assert_eq!(3, image);
        }

        #[ignore]
        #[test]
        fn sunset() {
            let now = SUN.sunset;

            let image = get_image(now, &SUN, TimePeriod::AfterSunset, &WALLPAPER);
            assert_eq!(4, image);
        }

        #[ignore]
        #[test]
        fn after_sunset() {
            let now = SUN.sunset + Duration::hours(1);

            let image = get_image(now, &SUN, TimePeriod::AfterSunset, &WALLPAPER);
            assert_eq!(4, image);
        }
    }
}
