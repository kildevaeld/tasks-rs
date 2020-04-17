use super::Extract;
use crate::{Rejection, Task};
use futures_core::ready;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct Unroll<T>(pub(crate) T);

impl<T, R> Task<R> for Unroll<T>
where
    T: Task<R>,
    T::Output: Extract<R>,
{
    type Output = <T::Output as Extract<R>>::Extract;
    type Error = T::Error;
    type Future = UnrollFuture<T, R>;
    fn run(&self, req: R) -> Self::Future {
        UnrollFuture {
            future: self.0.run(req),
        }
    }
}

#[pin_project]
pub struct UnrollFuture<T, R>
where
    T: Task<R>,
{
    #[pin]
    future: T::Future,
}

impl<T, R> Future for UnrollFuture<T, R>
where
    T: Task<R>,
    T::Output: Extract<R>,
{
    type Output = Result<<T::Output as Extract<R>>::Extract, Rejection<R, T::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        match ready!(this.future.poll(cx)) {
            Ok(ret) => {
                let (_, out) = ret.unpack();
                Poll::Ready(Ok(out))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
