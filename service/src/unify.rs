use super::{Either, Rejection, Service};
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

impl<F, T, E, R> Service<R> for Unify<F>
where
    F: Service<R, Output = Either<T, T>, Error = Either<E, E>>,
    R: Send,
    E: Send,
{
    type Output = T;
    type Error = E;
    type Future = UnifyFuture<F::Future, R, E>;
    #[inline]
    fn call(&self, req: R) -> Self::Future {
        UnifyFuture {
            inner: self.filter.call(req),
            _r: PhantomData,
            _e: PhantomData,
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct UnifyFuture<F, R, E> {
    #[pin]
    inner: F,
    _r: PhantomData<R>,
    _e: PhantomData<E>,
}

impl<F, R, E, T> Future for UnifyFuture<F, R, E>
where
    F: TryFuture<Ok = Either<T, T>, Error = Rejection<R, Either<E, E>>>,
{
    type Output = Result<T, Rejection<R, E>>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let unified = match ready!(self.project().inner.try_poll(cx)) {
            Ok(Either::A(a)) => Ok(a),
            Ok(Either::B(b)) => Ok(b),
            Err(Rejection::Err(Either::A(a))) => Err(Rejection::Err(a)),
            Err(Rejection::Err(Either::B(b))) => Err(Rejection::Err(b)),
            Err(Rejection::Reject(req, Some(Either::A(a)))) => Err(Rejection::Reject(req, Some(a))),
            Err(Rejection::Reject(req, Some(Either::B(b)))) => Err(Rejection::Reject(req, Some(b))),
            Err(Rejection::Reject(req, None)) => Err(Rejection::Reject(req, None)),
        };
        Poll::Ready(unified)
    }
}
