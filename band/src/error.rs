use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    TaskNotFound(String),
    InvalidDepency(String),
    External(Box<dyn StdError + Send>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl StdError for Error {}

impl From<Box<dyn StdError + Send>> for Error {
    fn from(error: Box<dyn StdError + Send>) -> Self {
        Error::External(error)
    }
}
