use super::Extract;
use crate::{One, Rejection, Task};
use futures_core::ready;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Untuple<T>(pub(crate) T);

impl<T, R, O> Task<R> for Untuple<T>
where
    T: Task<R, Output = (O,)>,
{
    type Output = O;
    type Error = T::Error;
    type Future = UntupleFuture<T, R>;
    fn run(&self, req: R) -> Self::Future {
        UntupleFuture {
            future: self.0.run(req),
        }
    }
}

#[pin_project]
pub struct UntupleFuture<T, R>
where
    T: Task<R>,
{
    #[pin]
    future: T::Future,
}

impl<T, R, O> Future for UntupleFuture<T, R>
where
    T: Task<R, Output = (O,)>,
{
    type Output = Result<O, Rejection<R, T::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        match ready!(this.future.poll(cx)) {
            Ok((ret,)) => Poll::Ready(Ok(ret)),
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
