use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Rejection, Task};
use pin_project::pin_project;

pub fn pass<R: Send, E: Send>() -> impl Task<R, Output = (R, ()), Error = E> + Copy {
    Pass(PhantomData, PhantomData)
}

#[allow(missing_debug_implementations)]
struct Pass<R, E>(PhantomData<R>, PhantomData<E>);

impl<R, E> Copy for Pass<R, E> {}

impl<R, E> Clone for Pass<R, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<R, E> Task<R> for Pass<R, E>
where
    R: Send,
    E: Send,
{
    type Output = (R, ());
    type Error = E;
    type Future = PassFut<R, E>;

    #[inline]
    fn run(&self, req: R) -> Self::Future {
        PassFut(State::First(req), std::marker::PhantomData)
    }
}

enum State<R> {
    First(R),
    Done,
}

#[allow(missing_debug_implementations)]
#[pin_project]
struct PassFut<R, E>(State<R>, std::marker::PhantomData<E>);

impl<R, E> Future for PassFut<R, E> {
    type Output = Result<(R, ()), Rejection<R, E>>;

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
