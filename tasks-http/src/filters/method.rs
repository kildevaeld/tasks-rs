use crate::{Error, Request};
use futures::future;
use hyper::Method;
use tasks_core::{filter_fn_one, task, Rejection, Task};

pub fn get() -> impl Task<Request, Output = (Request, ()), Error = Error> + Copy {
    method_is(|| &Method::GET)
}

pub fn post() -> impl Task<Request, Output = (Request, ()), Error = Error> + Copy {
    method_is(|| &Method::POST)
}

pub fn method() -> impl Task<Request, Output = (Request, (Method,)), Error = Error> + Copy {
    filter_fn_one(|req: &mut Request| future::ok(req.method().clone()))
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
