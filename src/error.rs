use spa;
use std;
use std::fmt;

#[derive(Debug)]
pub enum DwError {
    SpaError(spa::SpaError),
    ParseFloatError(std::num::ParseFloatError),
    VarError(std::env::VarError),
}

impl fmt::Display for DwError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DwError::SpaError(ref err) => write!(f, "{:?}", err),
            DwError::ParseFloatError(ref err) => err.fmt(f),
            DwError::VarError(ref err) => err.fmt(f),
        }
    }
}

impl std::error::Error for DwError {
    fn description(&self) -> &str {
        match *self {
            DwError::SpaError(ref _err) => "spa error",
            DwError::ParseFloatError(ref err) => err.description(),
            DwError::VarError(ref err) => err.description(),
        }
    }
}

impl From<spa::SpaError> for DwError {
    fn from(err: spa::SpaError) -> Self {
        DwError::SpaError(err)
    }
}

impl From<std::env::VarError> for DwError {
    fn from(err: std::env::VarError) -> Self {
        DwError::VarError(err)
    }
}

impl From<std::num::ParseFloatError> for DwError {
    fn from(err: std::num::ParseFloatError) -> Self {
        DwError::ParseFloatError(err)
    }
}
