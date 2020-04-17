use crate::{Error, KnownError, Request, Response};
use futures::future;
use headers::{Header, HeaderMapExt};
use tasks_core::{filter_fn_one, task, Task};

pub fn header<H: Header + Send + 'static>(
) -> impl Task<Request, Output = (Request, (H,)), Error = Error> + Copy {
    filter_fn_one(move |req: &mut Request| {
        log::trace!("header2({:?})", H::name());
        let route = req
            .headers()
            .typed_get()
            .ok_or_else(|| KnownError::InvalidHeader(H::name().as_str().to_owned()).into());
        future::ready(route)
    })
}

pub fn optional<H: Header + Send + 'static>(
) -> impl Task<Request, Output = (Request, (Option<H>,)), Error = Error> + Copy + Copy {
    filter_fn_one(move |req: &mut Request| future::ready(Ok(req.headers().typed_get())))
}
