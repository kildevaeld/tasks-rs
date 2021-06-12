use crate::{Rejection, Service};
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;

#[derive(Clone)]
pub struct ErrInto<T, E> {
    task: T,
    _e: PhantomData<E>,
}

impl<T, E> ErrInto<T, E> {
    pub fn new(task: T) -> ErrInto<T, E> {
        ErrInto {
            task,
            _e: PhantomData,
        }
    }
}

impl<T, E, R> Service<R> for ErrInto<T, E>
where
    T: Service<R>,
    E: From<T::Error>,
{
    type Output = T::Output;
    type Error = E;
    type Future = ErrIntoFuture<T, E, R>;
    fn call(&mut self, req: R) -> Self::Future {
        ErrIntoFuture {
            fut: self.task.call(req),
            _e: PhantomData,
        }
    }
}

#[pin_project]
pub struct ErrIntoFuture<T, E, R>
where
    T: Service<R>,
{
    #[pin]
    fut: T::Future,
    _e: PhantomData<E>,
}

impl<T, R, E> Future for ErrIntoFuture<T, E, R>
where
    T: Service<R>,
    E: From<T::Error>,
{
    type Output = Result<T::Output, Rejection<R, E>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match ready!(this.fut.poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(Rejection::Err(err)) => Poll::Ready(Err(Rejection::Err(err.into()))),
            Err(Rejection::Reject(req, Some(err))) => {
                Poll::Ready(Err(Rejection::Reject(req, Some(err.into()))))
            }
            Err(Rejection::Reject(req, None)) => Poll::Ready(Err(Rejection::Reject(req, None))),
        }
    }
}
