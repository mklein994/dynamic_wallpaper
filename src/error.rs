use std::fmt;

#[derive(Debug)]
pub enum Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }
}

impl std::error::Error for Error {}
