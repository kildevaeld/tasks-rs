use super::{Either, Service};
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;

#[derive(Clone, Copy, Debug)]
pub struct Unify<F> {
    pub(super) filter: F,
}

impl<F, T, R> Service<R> for Unify<F>
where
    F: Service<R, Output = Either<T, T>>,
    R: Send,
{
    type Output = T;
    type Error = F::Error;
    type Future = UnifyFuture<F::Future, R>;
    #[inline]
    fn call(&self, req: R) -> Self::Future {
        UnifyFuture {
            inner: self.filter.call(req),
            _r: PhantomData,
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct UnifyFuture<F, R> {
    #[pin]
    inner: F,
    _r: PhantomData<R>,
}

impl<F, R, T> Future for UnifyFuture<F, R>
where
    F: TryFuture<Ok = Either<T, T>>,
{
    type Output = Result<T, F::Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let unified = match ready!(self.project().inner.try_poll(cx)) {
            Ok(Either::A(a)) => Ok(a),
            Ok(Either::B(b)) => Ok(b),
            Err(err) => Err(err),
        };
        Poll::Ready(unified)
    }
}
