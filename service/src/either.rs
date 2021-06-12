use core::fmt;
#[cfg(feature = "std")]
use std::error::Error;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Either<T, U> {
    A(T),
    B(U),
}

impl<T, U> fmt::Display for Either<T, U>
where
    T: fmt::Display,
    U: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Either::A(a) => write!(f, "{}", a),
            Either::B(b) => write!(f, "{}", b),
        }
    }
}

#[cfg(feature = "std")]
impl<T, U> Error for Either<T, U>
where
    T: Error + 'static,
    U: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Either::A(a) => Some(a),
            Either::B(b) => Some(b),
        }
    }
}
