use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    NotFound,
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
