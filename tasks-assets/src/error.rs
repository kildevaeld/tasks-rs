use std::error::Error as StdError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Unknown,
    NotFound,
    Io(io::Error),
    Custom(String),
    Vinyl(tasks_vinyl::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl StdError for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}

impl From<tasks_vinyl::Error> for Error {
    fn from(error: tasks_vinyl::Error) -> Self {
        Error::Vinyl(error)
    }
}
