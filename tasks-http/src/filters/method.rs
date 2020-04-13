use crate::Request;
use hyper::Method;
use tasks_core::filter::{filter_fn_one, Filter};

pub fn get() -> impl Filter<Request, Extract = (Body,), Error = Error> + Copy {
    method(Method::GET)
}

pub fn method(method: Method) -> impl Filter<Request, Extract = (Body,), Error = Error> + Copy {
    filter_fn_one(|req| {});
}
