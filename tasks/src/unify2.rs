use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::{Either, Task};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub struct Unify2<F> {
    pub(super) filter: F,
}

impl<F, T, R> Task<R> for Unify2<F>
where
    F: Task<R, Output = Either<T, T>>,
    R: Send,
{
    type Output = T;
    type Error = F::Error;
    type Future = Unify2Future<F::Future, R>;
    #[inline]
    fn run(&self, req: R) -> Self::Future {
        Unify2Future {
            inner: self.filter.run(req),
            _r: PhantomData,
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct Unify2Future<F, R> {
    #[pin]
    inner: F,
    _r: PhantomData<R>,
}

impl<F, R, T> Future for Unify2Future<F, R>
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
