use super::{Request, Response};
use modifier::Modifier;
use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;
use tasks::Rejection;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct Error {
    pub(crate) inner: BoxError,
    pub(crate) response: Response,
}

impl Error {
    pub(crate) fn new<I: Into<BoxError>>(error: I) -> Error {
        Error {
            inner: error.into(),
            response: Response::new(),
        }
    }

    #[inline(always)]
    pub fn set<M: Modifier<Response>>(mut self, modifier: M) -> Self
    where
        Self: Sized,
    {
        modifier.modify(&mut self.response);
        self
    }

    /// Modify self through a mutable reference with the provided modifier.
    #[inline(always)]
    pub fn set_mut<M: Modifier<Response>>(&mut self, modifier: M) -> &mut Self {
        modifier.modify(&mut self.response);
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl StdError for Error {}

impl From<Error> for Rejection<Request, Error> {
    fn from(error: Error) -> Rejection<Request, Error> {
        Rejection::Err(error)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KnownError {
    InvalidHeader(String),
    FilePermission(Option<PathBuf>),
    FileOpen(Option<PathBuf>),
    NotFound,
}

impl fmt::Display for KnownError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KnownError::InvalidHeader(name) => write!(f, "invalid header: {}", name),
            KnownError::FilePermission(path) => write!(f, "file permission: {:?}", path),
            KnownError::FileOpen(path) => write!(f, "file open {:?}", path),
            KnownError::NotFound => write!(f, "not found"),
        }
    }
}

impl StdError for KnownError {}

impl From<KnownError> for Error {
    fn from(error: KnownError) -> Error {
        Error::new(error)
    }
}

impl From<KnownError> for Rejection<Request, Error> {
    fn from(error: KnownError) -> Self {
        Error::new(error).into()
    }
}
