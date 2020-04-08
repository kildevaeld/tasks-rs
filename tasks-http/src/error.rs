use super::{Request, Response};
use modifier::Modifier;
use std::error::Error as StdError;
use std::fmt;
use tasks_core::Rejection;

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
