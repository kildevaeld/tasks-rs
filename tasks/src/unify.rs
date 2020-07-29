use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::{Either, Task, Tuple};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub struct Unify<F> {
    pub(super) filter: F,
}

impl<F, T, R> Task<R> for Unify<F>
where
    F: Task<R, Output = Either<(R, (T,)), (R, (T,))>>,
    T: Tuple,
    R: Send,
{
    type Output = (R, (T,));
    type Error = F::Error;
    type Future = UnifyFuture<F::Future, R>;
    #[inline]
    fn run(&self, req: R) -> Self::Future {
        UnifyFuture {
            inner: self.filter.run(req),
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
    F: TryFuture<Ok = Either<(R, (T,)), (R, (T,))>>,
{
    type Output = Result<(R, (T,)), F::Error>;

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
