use super::Extract;
use crate::{Rejection, Service};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;

#[derive(Clone)]
pub struct Unpack<T>(pub(crate) T);

impl<T, R> Service<R> for Unpack<T>
where
    T: Service<R>,
    T::Output: Extract<R>,
{
    type Output = <T::Output as Extract<R>>::Extract;
    type Error = T::Error;
    type Future = UnpackFuture<T, R>;
    fn call(&self, req: R) -> Self::Future {
        UnpackFuture {
            future: self.0.call(req),
        }
    }
}

#[pin_project]
pub struct UnpackFuture<T, R>
where
    T: Service<R>,
{
    #[pin]
    future: T::Future,
}

impl<T, R> Future for UnpackFuture<T, R>
where
    T: Service<R>,
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
