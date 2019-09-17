use std::fmt;
use std::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum TaskError {
    ReceiverClosed,
    NullFuture,
    InvalidRequest,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TaskError({:?})", self)
    }
}

impl Error for TaskError {}
