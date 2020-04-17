#[cfg(feature = "statics")]
mod dir;
#[cfg(feature = "statics")]
mod file;

#[cfg(feature = "statics")]
pub use self::{dir::*, file::*};
