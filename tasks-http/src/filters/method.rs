use crate::{Error, Request};
use futures::future;
use hyper::Method;
use tasks_core::{task, Rejection, Task};

pub fn get() -> impl Task<Request, Output = (Request, ()), Error = Error> + Copy {
    method_is(|| &Method::GET)
}

fn method_is<F>(func: F) -> impl Task<Request, Output = (Request, ()), Error = Error> + Copy
where
    F: Fn() -> &'static Method + Copy,
{
    task!(move |req: Request| {
        let method = func();
        log::trace!("method::{:?}?: {:?}", method, req.method());
        if req.method() == method {
            future::ok((req, ()))
        } else {
            future::err(Rejection::Reject(req))
        }
    })
}
