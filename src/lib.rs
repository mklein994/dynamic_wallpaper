//! Dynamic Wallpaper
//!
//! Print the index of the image to use depending on the time of day and
//! location. These are set in `~/.config/dynamic_wallpaper/config.toml`.

extern crate chrono;
extern crate chrono_humanize;
extern crate dirs;
extern crate env_logger;
extern crate failure;
#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate spa;
extern crate toml;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, Duration, Local, Timelike, Utc};
use std::fmt;

/// Result type alias to handle errors.
type Result<T> = std::result::Result<T, failure::Error>;

/// Initialize logging.
fn setup() {
    env_logger::Builder::from_default_env()
        .default_format_module_path(false)
        .default_format_timestamp(false)
        .init();
    info!("logging enabled");
}

/// Main entry point.
pub fn run() -> Result<()> {
    setup();

    let config = Config::new()?;
    let now = config.now;
    let wallpaper = config.wallpaper;

    let sun = Sun::new(now, config.lat, config.lon)?;
    debug!("{}", sun);

    let time_period = TimePeriod::new(&now, &sun.sunrise, &sun.sunset);
    info!("{}", time_period);

    let image = get_image(now, &sun, &time_period, &wallpaper);

    println!("{}", image);

    Ok(())
}

/// Get the image index for the current time, within the time period, from the `image_count` number
/// of images.
fn get_image(
    now: DateTime<Utc>,
    sun: &Sun,
    time_period: &TimePeriod,
    wallpaper: &Wallpaper,
) -> i64 {
    let offset = wallpaper.offset(&time_period);

    let image_count = wallpaper.image_count(&time_period);

    let (start, end) = sun.start_end(time_period);
    let duration = (end - start).num_nanoseconds().unwrap();
    let elapsed_time = (now - start).num_nanoseconds().unwrap();

    //  elapsed_time
    // ━━━━━━━━━━━━━━━
    //  (end - start)
    //  ─────────────
    //   image_count
    (offset + elapsed_time * image_count / duration) % wallpaper.count
}

/// Program configuration.
///
/// Example config:
/// ```toml
/// # useful for debugging; defaults to now. Needs to be in RFC3339 format.
/// #now = "2018-08-31T01:45:00.123456789-05:00"
/// lat = 12.3456
/// lon = -65.4321
///
/// # these are the defaults:
/// [wallpaper]
/// # The total number of images.
/// count = 16
/// # The image to use just as the sun appears.
/// daybreak = 2
/// # The image to use just as the moon shows up.
/// nightfall = 13
/// ```
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Current time. Defaults to now.
    #[serde(default = "default_time")]
    pub now: DateTime<Utc>,
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
fn default_time() -> DateTime<Utc> {
    Utc::now()
}

impl Config {
    /// Read a config file from `~/.config/dynamic_wallpaper/config.toml`.
    fn new() -> Result<Self> {
        use std::fs;

        let filename = dirs::config_dir()
            .expect("Couldn't find $XDG_CONFIG_DIR (~/.config/)")
            .join("dynamic_wallpaper")
            .join("config.toml");

        let contents = fs::read_to_string(filename)?;

        let config: Self = toml::from_str(&contents)?;

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
    /// Number of images for the time period.
    fn image_count(&self, time_period: &TimePeriod) -> i64 {
        if self.daybreak < self.nightfall {
            if *time_period == TimePeriod::DayTime {
                self.nightfall - self.daybreak
            } else {
                i64::abs(self.count - self.nightfall + self.daybreak) % self.count
            }
        } else if *time_period == TimePeriod::DayTime {
            i64::abs(self.count - self.daybreak + self.nightfall) % self.count
        } else {
            i64::abs(self.nightfall - self.daybreak)
        }
    }

    /// Index to use as the start of the phase (daybreak or nightfall).
    fn offset(&self, time_period: &TimePeriod) -> i64 {
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
    /// Yesterday's sunset.
    last_sunset: DateTime<Utc>,
    /// Today's sunrise.
    sunrise: DateTime<Utc>,
    /// Today's sunset.
    sunset: DateTime<Utc>,
    /// Tomorrow's sunrise.
    next_sunrise: DateTime<Utc>,
}

impl Sun {
    /// Get the sunrise and sunset times depending on the current time and location.
    fn new(now: DateTime<Utc>, lat: f64, lon: f64) -> Result<Self> {
        use spa::SunriseAndSet;

        let utc_now = now
            .with_timezone(&Local)
            .date()
            .and_hms(12, 0, 0)
            .with_timezone(&Utc);

        info!("now:          {}", now.with_timezone(&Local));
        debug!("UTC now:      {}", utc_now);

        debug_assert!(Utc::today() - Duration::days(1) <= now.date());
        debug_assert!(Utc::today() + Duration::days(1) >= now.date());

        let last_sunset = match spa::calc_sunrise_and_set(utc_now - Duration::days(1), lat, lon)? {
            SunriseAndSet::Daylight(_, sunset) => sunset,
            _ => unimplemented!(),
        };

        let (sunrise, sunset) = match spa::calc_sunrise_and_set(utc_now, lat, lon)? {
            SunriseAndSet::Daylight(sunrise, sunset) => (sunrise, sunset),
            _ => unimplemented!(),
        };

        let next_sunrise = match spa::calc_sunrise_and_set(utc_now + Duration::days(1), lat, lon)? {
            spa::SunriseAndSet::Daylight(sunrise, _) => sunrise,
            _ => unimplemented!(),
        };

        debug_assert!(last_sunset < sunrise);
        debug_assert!(sunrise < sunset);
        debug_assert!(sunset < next_sunrise);

        debug_assert!(last_sunset.with_nanosecond(0).unwrap() <= now.with_nanosecond(0).unwrap());
        debug_assert!(now.with_nanosecond(0).unwrap() <= next_sunrise.with_nanosecond(0).unwrap());

        Ok(Self {
            last_sunset,
            sunrise,
            sunset,
            next_sunrise,
        })
    }

    /// Get the sunrise/sunset pairs depending on the time period.
    fn start_end(&self, time_period: &TimePeriod) -> (DateTime<Utc>, DateTime<Utc>) {
        match time_period {
            TimePeriod::BeforeSunrise => (self.last_sunset, self.sunrise),
            TimePeriod::DayTime => (self.sunrise, self.sunset),
            TimePeriod::AfterSunset => (self.sunset, self.next_sunrise),
        }
    }
}

impl fmt::Display for Sun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Sun:\n{}\n{:<13} {}\n{:<13} {}\n{:<13} {}\n{:<13} {}",
            "-".repeat(50),
            "last sunset:",
            self.last_sunset.with_timezone(&Local),
            "sunrise:",
            self.sunrise.with_timezone(&Local),
            "sunset:",
            self.sunset.with_timezone(&Local),
            "next sunrise:",
            self.next_sunrise.with_timezone(&Local)
        )
    }
}

/// Time of day according to the sun.
#[derive(Debug, PartialEq)]
enum TimePeriod {
    /// After the sun has set.
    AfterSunset,
    /// Before the sun has risen.
    BeforeSunrise,
    /// Between sunrise and sunset.
    DayTime,
}

impl TimePeriod {
    /// Determine the time period depending on the given time and the times for
    /// sunrise and sunset.
    fn new(now: &DateTime<Utc>, sunrise: &DateTime<Utc>, sunset: &DateTime<Utc>) -> Self {
        if *now > *sunset {
            TimePeriod::AfterSunset
        } else if *now >= *sunrise {
            TimePeriod::DayTime
        } else {
            TimePeriod::BeforeSunrise
        }
    }
}

impl fmt::Display for TimePeriod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TimePeriod::AfterSunset => write!(f, "\u{1f306} After Sunset"),
            TimePeriod::BeforeSunrise => write!(f, "\u{1f305} Before Sunrise"),
            TimePeriod::DayTime => write!(f, "\u{1f3d9} Daytime"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    lazy_static! {
        static ref SUN: Sun = Sun {
            last_sunset: Local.ymd(2018, 8, 5).and_hms(21, 3, 24).with_timezone(&Utc),
            sunrise: Local.ymd(2018, 8, 6).and_hms(6, 4, 25).with_timezone(&Utc),
            sunset: Local.ymd(2018, 8, 6).and_hms(21, 1, 44).with_timezone(&Utc),
            next_sunrise: Local.ymd(2018, 8, 6).and_hms(6, 5, 52).with_timezone(&Utc),
        };
    }

    const WALLPAPER: Wallpaper = Wallpaper {
        count: 16,
        daybreak: 2,
        nightfall: 13,
    };

    #[test]
    fn image_count_daytime() {
        let image_count = WALLPAPER.image_count(&TimePeriod::DayTime);
        assert_eq!(11, image_count);
    }

    #[test]
    fn image_count_before_sunrise() {
        let image_count = WALLPAPER.image_count(&TimePeriod::BeforeSunrise);
        assert_eq!(5, image_count);
    }

    #[test]
    fn image_count_after_sunset() {
        let image_count = WALLPAPER.image_count(&TimePeriod::AfterSunset);
        assert_eq!(5, image_count);
    }

    #[test]
    fn image_count_daytime_daybreak_greater_than_nightfall() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 13,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::DayTime);
        assert_eq!(5, image_count);
    }

    #[test]
    fn image_count_before_sunrise_daybreak_greater_than_nightfall() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 13,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::BeforeSunrise);
        assert_eq!(11, image_count);
    }

    #[test]
    fn image_count_after_sunset_daybreak_greater_than_nightfall() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 13,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::AfterSunset);
        assert_eq!(11, image_count);
    }

    #[test]
    fn image_count_daytime_daybreak_is_count() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 16,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::DayTime);
        assert_eq!(2, image_count);
    }

    #[test]
    fn image_count_before_sunrise_daybreak_is_count() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 16,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::BeforeSunrise);
        assert_eq!(14, image_count);
    }

    #[test]
    fn image_count_after_sunset_daybreak_is_count() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 16,
            nightfall: 2,
        };
        let image_count = wallpaper.image_count(&TimePeriod::AfterSunset);
        assert_eq!(14, image_count);
    }

    #[test]
    fn image_count_daytime_nightfall_is_count() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 3,
            nightfall: 16,
        };
        let image_count = wallpaper.image_count(&TimePeriod::DayTime);
        assert_eq!(13, image_count);
    }

    #[test]
    fn image_count_before_sunrise_nightfall_is_count() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 3,
            nightfall: 16,
        };
        let image_count = wallpaper.image_count(&TimePeriod::BeforeSunrise);
        assert_eq!(3, image_count);
    }

    #[test]
    fn image_count_after_sunset_nightfall_is_count() {
        let wallpaper = Wallpaper {
            count: 16,
            daybreak: 3,
            nightfall: 16,
        };
        let image_count = wallpaper.image_count(&TimePeriod::AfterSunset);
        assert_eq!(3, image_count);
    }

    #[test]
    fn time_period_noon() {
        let time_period = TimePeriod::new(
            &Local.ymd(2018, 8, 6).and_hms(12, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_last_midnight() {
        let time_period = TimePeriod::new(
            &Local.ymd(2018, 8, 6).and_hms(0, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::BeforeSunrise, time_period);
    }

    #[test]
    fn time_period_next_midnight() {
        let time_period = TimePeriod::new(
            &Local.ymd(2018, 8, 7).and_hms(0, 0, 0).with_timezone(&Utc),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::AfterSunset, time_period);
    }

    #[test]
    fn time_period_sunrise() {
        let time_period = TimePeriod::new(&SUN.sunrise, &SUN.sunrise, &SUN.sunset);
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_sunset() {
        let time_period = TimePeriod::new(&SUN.sunset, &SUN.sunrise, &SUN.sunset);
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_just_before_sunset() {
        let time_period = TimePeriod::new(
            &(SUN.sunset - Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn time_period_just_after_sunset() {
        let time_period = TimePeriod::new(
            &(SUN.sunset + Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::AfterSunset, time_period);
    }

    #[test]
    fn time_period_just_before_sunrise() {
        let time_period = TimePeriod::new(
            &(SUN.sunrise - Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::BeforeSunrise, time_period);
    }

    #[test]
    fn time_period_just_after_sunrise() {
        let time_period = TimePeriod::new(
            &(SUN.sunrise + Duration::nanoseconds(1)),
            &SUN.sunrise,
            &SUN.sunset,
        );
        assert_eq!(TimePeriod::DayTime, time_period);
    }

    #[test]
    fn get_image_sunrise() {
        let image = get_image(SUN.sunrise, &SUN, &TimePeriod::DayTime, &WALLPAPER);
        assert_eq!(WALLPAPER.daybreak, image);
    }

    #[test]
    fn get_image_sunset() {
        let image = get_image(SUN.sunset, &SUN, &TimePeriod::DayTime, &WALLPAPER);
        assert_eq!(WALLPAPER.nightfall, image);
    }

    #[test]
    fn get_image_just_past_sunrise() {
        let now = SUN.sunrise + Duration::nanoseconds(1);
        let image = get_image(now, &SUN, &TimePeriod::DayTime, &WALLPAPER);
        assert_eq!(2, image);
    }

    #[test]
    fn get_image_just_before_sunrise() {
        let now = SUN.sunrise - Duration::nanoseconds(1);
        debug_assert!(now < SUN.sunrise);
        let image = get_image(now, &SUN, &TimePeriod::BeforeSunrise, &WALLPAPER);
        assert_eq!(1, image);
    }

    #[test]
    fn get_image_just_before_sunset() {
        let now = SUN.sunset - Duration::nanoseconds(1);
        debug_assert!(now < SUN.sunset);
        let image = get_image(now, &SUN, &TimePeriod::DayTime, &WALLPAPER);
        assert_eq!(12, image);
    }

    #[test]
    fn get_image_just_past_sunset() {
        let now = SUN.sunset + Duration::nanoseconds(1);
        let image = get_image(now, &SUN, &TimePeriod::AfterSunset, &WALLPAPER);
        assert_eq!(13, image);
    }

    #[test]
    fn offset_daytime() {
        let wallpaper = Wallpaper {
            count: 10,
            daybreak: 2,
            nightfall: 7,
        };
        assert_eq!(wallpaper.daybreak, wallpaper.offset(&TimePeriod::DayTime));
    }

    #[test]
    fn offset_after_sunset() {
        let wallpaper = Wallpaper {
            count: 10,
            daybreak: 2,
            nightfall: 7,
        };
        assert_eq!(
            wallpaper.nightfall,
            wallpaper.offset(&TimePeriod::AfterSunset)
        );
    }

    #[test]
    fn offset_before_sunrise() {
        let wallpaper = Wallpaper {
            count: 10,
            daybreak: 2,
            nightfall: 7,
        };
        assert_eq!(
            wallpaper.nightfall,
            wallpaper.offset(&TimePeriod::BeforeSunrise)
        );
    }

}
