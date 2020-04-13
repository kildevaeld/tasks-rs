#[cfg(feature = "compress")]
mod compress;
mod error;
mod modifiers;
mod mount;
pub mod reply;
mod request;
mod response;
mod server;
mod service;
#[cfg(feature = "tls")]
mod tls;
mod transport;
#[cfg(feature = "ws")]
mod ws;

pub mod filters;

// Re-export;
pub use http;
pub use hyper;
pub use modifier;
pub use tasks_core as tasks;
pub use url;

pub use self::{error::*, modifiers::*, mount::*, request::*, response::*, server::*, service::*};

#[cfg(feature = "compress")]
pub use compress::*;

#[cfg(feature = "ws")]
pub use ws::*;

pub mod prelude {
    pub use super::{BoxError, Error, Request, Response};
    pub use hyper::{HeaderMap, StatusCode};
    pub use modifier::Set;
    pub use tasks_core::{middleware, task, Middleware, MiddlewareExt, Next, Task, TaskExt};
}
