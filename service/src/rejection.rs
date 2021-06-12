#[cfg(feature = "alloc")]
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum Rejection<R, E> {
    Reject(R, Option<E>),
    Err(E),
}

impl<R, E> fmt::Display for Rejection<R, E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Rejection::Err(err) => write!(f, "{}", err),
            Rejection::Reject(_, err) => match err {
                Some(err) => write!(f, "rejected with error: {}", err),
                None => write!(f, "rejected"),
            },
        }
    }
}

#[cfg(feature = "alloc")]
impl<R, E> Error for Rejection<R, E>
where
    R: fmt::Debug,
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Rejection::Err(err) => Some(err),
            Rejection::Reject(_, Some(err)) => Some(err),
            _ => None,
        }
    }
}

#[cfg(feature = "alloc")]
impl<R, E> From<E> for Rejection<R, E>
where
    E: Error,
{
    fn from(error: E) -> Rejection<R, E> {
        Rejection::Err(error)
    }
}
