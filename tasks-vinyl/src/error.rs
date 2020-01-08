use failure::Fail;
use std::io;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "Io error: {}", _0)]
    IoError(io::Error)
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::IoError(error)
    }
}