use crate::{Rejection, Service};
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;

#[derive(Clone)]
pub struct MapErr<T, F, E> {
    task: T,
    cb: F,
    _e: PhantomData<E>,
}

impl<T, F, E> MapErr<T, F, E> {
    pub fn new(task: T, cb: F) -> MapErr<T, F, E> {
        MapErr {
            task,
            cb,
            _e: PhantomData,
        }
    }
}

impl<T, F, E, R> Service<R> for MapErr<T, F, E>
where
    T: Service<R>,
    F: Send + Clone + Fn(T::Error) -> E,
{
    type Output = T::Output;
    type Error = E;
    type Future = MapErrFuture<T, F, R>;
    fn call(& self, req: R) -> Self::Future {
        MapErrFuture {
            fut: self.task.call(req),
            cb: self.cb.clone(),
        }
    }
}

#[pin_project]
pub struct MapErrFuture<T, F, R>
where
    T: Service<R>,
{
    #[pin]
    fut: T::Future,
    cb: F,
}

impl<T, F, R, E> Future for MapErrFuture<T, F, R>
where
    T: Service<R>,
    F: Fn(T::Error) -> E,
{
    type Output = Result<T::Output, Rejection<R, E>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match ready!(this.fut.poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(Rejection::Err(err)) => Poll::Ready(Err(Rejection::Err((this.cb)(err)))),
            Err(Rejection::Reject(req, Some(err))) => {
                Poll::Ready(Err(Rejection::Reject(req, Some((this.cb)(err)))))
            }
            Err(Rejection::Reject(req, None)) => Poll::Ready(Err(Rejection::Reject(req, None))),
        }
    }
}
