use crate::{Error, Request};
use futures::future;
use hyper::Method;
#[cfg(feature = "qs")]
use serde::de::DeserializeOwned;
#[cfg(feature = "qs")]
use serde_qs;
use tasks_core::{filter_fn_one, task, Rejection, Task};
use url::Url;

pub fn url() -> impl Task<Request, Output = (Request, (Url,)), Error = Error> + Copy {
    filter_fn_one(|req: &mut Request| future::ok(req.url().clone()))
}

#[cfg(feature = "qs")]
pub fn qs<S: DeserializeOwned + Send + 'static>(
) -> impl Task<Request, Output = (Request, (S,)), Error = Error> + Copy {
    filter_fn_one(|req: &mut Request| {
        let m = match serde_qs::from_str(req.url().query().unwrap_or("")) {
            Ok(s) => Ok(s),
            Err(e) => unimplemented!("qs fail: {}", e),
        };

        future::ready(m)
    })
}
