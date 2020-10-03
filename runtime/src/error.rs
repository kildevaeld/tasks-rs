use std::error::Error as StdError;
use std::fmt;
#[derive(Debug)]
pub struct SpawnError {
    pub(crate) inner: Box<dyn StdError>,
}

impl fmt::Display for SpawnError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl StdError for SpawnError {}
