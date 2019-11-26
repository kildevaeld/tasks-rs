use std::fmt;
use std::error::Error;
#[cfg(feature = "sync")]
use rayon::ThreadPoolBuildError;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TaskError {
    ReceiverClosed,
    NullFuture,
    InvalidRequest,
    #[cfg(feature = "sync")]
    ThreadPoolError,

}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TaskError({:?})", self)
    }
}

impl Error for TaskError {}
