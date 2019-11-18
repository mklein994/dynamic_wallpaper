use std::fmt;

#[derive(Debug)]
pub enum Error {
    Config(&'static str),
    Io(std::io::Error),
    Spa(spa::SpaError),
    Toml(toml::de::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Config(msg) => msg.fmt(f),
            Self::Io(err) => err.fmt(f),
            Self::Spa(err) => err.fmt(f),
            Self::Toml(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<spa::SpaError> for Error {
    fn from(err: spa::SpaError) -> Self {
        Self::Spa(err)
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Self::Toml(err)
    }
}
