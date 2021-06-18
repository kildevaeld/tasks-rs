use super::{Rejection, Service};
use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;
#[cfg(feature = "std")]
use std::error::Error;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Either<T, U> {
    A(T),
    B(U),
}

impl<T, U> fmt::Display for Either<T, U>
where
    T: fmt::Display,
    U: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Either::A(a) => write!(f, "{}", a),
            Either::B(b) => write!(f, "{}", b),
        }
    }
}

#[cfg(feature = "std")]
impl<T, U> Error for Either<T, U>
where
    T: Error + 'static,
    U: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Either::A(a) => Some(a),
            Either::B(b) => Some(b),
        }
    }
}

impl<A, B, R> Service<R> for Either<A, B>
where
    A: Service<R>,
    B: Service<R, Error = <A as Service<R>>::Error>,
    R: Send,
{
    type Output = Either<A::Output, B::Output>;
    type Error = A::Error;
    type Future = EitherFuture<A, B, R>;

    fn call(&self, req: R) -> Self::Future {
        match self {
            Either::A(a) => EitherFuture {
                fut: EitherPromise::First(a.call(req)),
                _r: std::marker::PhantomData,
            },
            Either::B(b) => EitherFuture {
                fut: EitherPromise::Second(b.call(req)),
                _r: std::marker::PhantomData,
            },
        }
    }
}

#[pin_project(project = EitherPromiseProj)]
enum EitherPromise<A, B> {
    First(#[pin] A),
    Second(#[pin] B),
}

#[pin_project]
pub struct EitherFuture<A, B, R>
where
    A: Service<R>,
    B: Service<R, Error = <A as Service<R>>::Error>,
{
    #[pin]
    fut: EitherPromise<A::Future, B::Future>,
    _r: std::marker::PhantomData<R>,
}

impl<A, B, R> EitherFuture<A, B, R>
where
    A: Service<R>,
    B: Service<R, Error = <A as Service<R>>::Error>,
{
    pub fn a(fut: A::Future) -> EitherFuture<A, B, R> {
        EitherFuture {
            fut: EitherPromise::First(fut),
            _r: std::marker::PhantomData,
        }
    }

    pub fn b(fut: B::Future) -> EitherFuture<A, B, R> {
        EitherFuture {
            fut: EitherPromise::Second(fut),
            _r: std::marker::PhantomData,
        }
    }
}

impl<A, B, R> Future for EitherFuture<A, B, R>
where
    A: Service<R>,
    B: Service<R, Error = <A as Service<R>>::Error>,
{
    #[allow(clippy::type_complexity)]
    type Output = Result<Either<A::Output, B::Output>, Rejection<R, A::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        match this.fut.project() {
            EitherPromiseProj::First(fut) => match ready!(fut.poll(cx)) {
                Ok(o) => Poll::Ready(Ok(Either::A(o))),
                Err(e) => Poll::Ready(Err(e)),
            },
            EitherPromiseProj::Second(fut) => match ready!(fut.poll(cx)) {
                Ok(o) => Poll::Ready(Ok(Either::B(o))),
                Err(e) => Poll::Ready(Err(e)),
            },
        }
    }
}
