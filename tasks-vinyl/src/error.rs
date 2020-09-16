use std::error::Error as StdError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    FileAlreadyExists,
    Other(Box<dyn StdError + Send>),
    InvalidMimeType {
        expected: mime::Mime,
    },
    #[cfg(feature = "tokio")]
    Join(tokio::task::JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl StdError for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(feature = "tokio")]
impl From<tokio::task::JoinError> for Error {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::Join(error)
    }
}
