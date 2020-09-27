//! A filter that matches any route.
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Error, Request};
use pin_project::pin_project;
use tasks::{Rejection, Task, TaskExt};

pub fn any() -> impl Task<Request, Output = (Request, ()), Error = Error> + Copy {
    Any
}

pub fn state<S: Send + Clone + 'static>(
    state: S,
) -> impl Task<Request, Output = (Request, (S,)), Error = Error> + Clone {
    any().map(move || state.clone())
}

#[allow(missing_debug_implementations)]
#[derive(Clone, Copy)]
struct Any;

impl Task<Request> for Any {
    type Output = (Request, ());
    type Error = Error;
    type Future = AnyFut<Request>;

    #[inline]
    fn run(&self, req: Request) -> Self::Future {
        AnyFut(State::First(req))
    }
}

enum State<R> {
    First(R),
    Done,
}

#[allow(missing_debug_implementations)]
#[pin_project]
struct AnyFut<R>(State<R>);

impl<R> Future for AnyFut<R> {
    type Output = Result<(R, ()), Rejection<Request, Error>>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        let state = std::mem::replace(this.0, State::Done);
        match state {
            State::First(req) => Poll::Ready(Ok((req, ()))),
            State::Done => panic!("poll after done"),
        }
    }
}
