use std::fmt;

#[derive(Debug)]
pub enum Error {
    Config(&'static str),
    Io(std::io::Error),
    SpaError(spa::SpaError),
    Toml(toml::de::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Config(msg) => msg.fmt(f),
            Error::Io(err) => err.fmt(f),
            // NOTE: This is only a wrapper around `spa::SpaError`, since that doesn't implement
            // std::error::Error itself.
            Error::SpaError(_) => write!(
                f,
                "latitude must be between -90° and 90°, and \
                 longitude must be between -180° and 180°"
            ),
            Error::Toml(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<spa::SpaError> for Error {
    fn from(err: spa::SpaError) -> Self {
        Error::SpaError(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Toml(err)
    }
}
