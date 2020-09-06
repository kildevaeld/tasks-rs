use std::fmt;
use std::io;
use tasks_vinyl::Error as VinylError;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Vinyl(VinylError),
    NotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<VinylError> for Error {
    fn from(error: VinylError) -> Self {
        Self::Vinyl(error)
    }
}
