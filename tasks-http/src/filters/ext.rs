use crate::{Error, Request};
use futures::future;
use tasks::{filter_fn_one, Task};

pub fn get_ext<S: Clone + Send + Sync + 'static>(
) -> impl Task<Request, Output = (Request, (Option<S>,)), Error = Error> + Copy {
    filter_fn_one(|req: &mut Request| future::ok(req.extensions().get().map(|m: &S| m.clone())))
}
