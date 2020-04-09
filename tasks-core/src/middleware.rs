use super::Rejection;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

pub trait Next<Req>: Send + Sync {
    type Output;
    type Error;
    fn run(
        &self,
        req: Req,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Rejection<Req, Self::Error>>> + Send>>;
}

pub trait Middleware<R> {
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send;
    fn run<N: Clone + 'static + Next<R, Output = Self::Output, Error = Self::Error>>(
        &self,
        req: R,
        next: N,
    ) -> Self::Future;
}

pub struct MiddlewareFn<F, I, O, E> {
    inner: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
    _e: PhantomData<E>,
}

impl<F, I, O, E> Clone for MiddlewareFn<F, I, O, E>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        MiddlewareFn {
            inner: self.inner.clone(),
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E, U> MiddlewareFn<F, I, O, E>
where
    F: (Fn(I, NextFn<I, O, E>) -> U) + Send + std::marker::Unpin,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send + 'static,
{
    pub fn new(middleware: F) -> MiddlewareFn<F, I, O, E> {
        MiddlewareFn {
            inner: middleware,
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E, U> Middleware<I> for MiddlewareFn<F, I, O, E>
where
    F: (Fn(I, NextFn<I, O, E>) -> U) + Send + std::marker::Unpin,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send + 'static,
{
    type Output = O;
    type Error = E;
    type Future = U;

    fn run<N: Clone + 'static + Next<I, Output = Self::Output, Error = Self::Error>>(
        &self,
        req: I,
        next: N,
    ) -> Self::Future {
        (self.inner)(
            req,
            NextFn {
                inner: Box::new(next),
            },
        )
    }
}

pub struct NextFn<R, O, E> {
    inner: Box<dyn Next<R, Output = O, Error = E> + Send + Sync>,
}

impl<R, O, E> Next<R> for NextFn<R, O, E> {
    type Output = O;
    type Error = E;
    fn run(
        &self,
        req: R,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send>> {
        self.inner.run(req)
    }
}
