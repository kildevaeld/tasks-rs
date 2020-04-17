//! A filter that matches any route.
use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::filter::Filter;
use pin_project::pin_project;

pub fn any<R: Send>() -> impl Filter<R, Extract = (), Error = Infallible> + Copy {
    Any(PhantomData)
}

#[allow(missing_debug_implementations)]
struct Any<R>(PhantomData<R>);

impl<R> Copy for Any<R> {}

impl<R> Clone for Any<R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Filter<R> for Any<R>
where
    R: Send,
{
    type Extract = ();
    type Error = Infallible;
    type Future = AnyFut<R>;

    #[inline]
    fn filter(&self, req: R) -> Self::Future {
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
    type Output = Result<(R, ()), Infallible>;

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
