use crate::{Rejection, Task};
use futures_core::ready;
use pin_project::pin_project;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct MapRejection<T, F, R1, R2> {
    task: T,
    cb: F,
    _r1: PhantomData<R1>,
    _r2: PhantomData<R2>,
}

impl<T, F, R1, R2> Task<R1> for MapRejection<T, F, R1, R2>
where
    T: Task<R1>,
    F: Send + Clone + Fn(R1) -> R2,
{
    type Output = T::Output;
    type Error = T::Error;
    type Future = MapRejectionFuture<T, F, R1>;
    fn run(&self, req: R1) -> Self::Future {
        MapRejectionFuture {
            fut: self.task.run(req),
            cb: self.cb.clone(),
        }
    }
}

#[pin_project]
pub struct MapRejectionFuture<T, F, Req>
where
    T: Task<Req>,
{
    #[pin]
    fut: T::Future,
    cb: F,
}

impl<T, F, Req, R> Future for MapRejectionFuture<T, F, Req>
where
    T: Task<Req>,
    F: Fn(Req) -> R,
{
    type Output = Result<T::Output, Rejection<R, T::Error>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match ready!(this.fut.poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(Rejection::Err(err)) => Poll::Ready(Err(Rejection::Err(err))),
            Err(Rejection::Reject(req, err)) => {
                Poll::Ready(Err(Rejection::Reject((this.cb)(req), err)))
            }
        }
    }
}
