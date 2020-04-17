use crate::{Error, Request};
use bytes::{buf::BufExt, Buf, Bytes};
use futures::{future, FutureExt, TryFutureExt};
use hyper::Body;
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;
use std::fmt;
use tasks_core::{filter_fn_one, Task, TaskExt};

#[derive(Debug)]
pub struct BodyReadError(hyper::Error);

impl fmt::Display for BodyReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "body read error: {}", self.0)
    }
}

impl std::error::Error for BodyReadError {}

pub fn body() -> impl Task<Request, Output = (Request, (Body,)), Error = Error> + Copy {
    filter_fn_one(|req: &mut Request| future::ready(Ok(req.take_body().unwrap())))
}

pub fn aggregate() -> impl Task<Request, Output = (Request, (impl Buf,)), Error = Error> + Copy {
    body().and_then(|body: Body| hyper::body::aggregate(body).map_err(|err| Error::new(err)))
}

pub fn bytes() -> impl Task<Request, Output = (Request, (impl Buf,)), Error = Error> + Copy {
    body().and_then(|body: Body| {
        hyper::body::to_bytes(body).map_err(|err| {
            log::debug!("to_bytes error: {}", err);
            Error::new(BodyReadError(err))
        })
    })
}

#[cfg(feature = "json")]
pub fn json<S: DeserializeOwned + Send>(
) -> impl Task<Request, Output = (Request, (S,)), Error = Error> + Copy {
    aggregate().and_then(|buf| async move { do_json(buf) })
}

#[cfg(feature = "json")]
fn do_json<B: Buf, S: DeserializeOwned + Send>(buf: B) -> Result<S, Error> {
    Ok(serde_json::from_reader(buf.reader()).map_err(|err| Error::new(err))?)
}
