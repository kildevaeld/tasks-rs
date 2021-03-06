use super::Rejection;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;

pub trait Service<R> {
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send;
    fn call(&self, req: R) -> Self::Future;
}

#[derive(Debug)]
pub struct ServiceFn<F, I, O, E> {
    f: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
    _e: PhantomData<E>,
}

impl<F, I, O, E> Clone for ServiceFn<F, I, O, E>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        ServiceFn {
            f: self.f.clone(),
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E> Copy for ServiceFn<F, I, O, E> where F: Copy {}

unsafe impl<F, I, O, E> Sync for ServiceFn<F, I, O, E> where F: Sync {}

unsafe impl<F, I, O, E> Send for ServiceFn<F, I, O, E> where F: Send {}

impl<F, I, O, E, U> ServiceFn<F, I, O, E>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send + 'static,
{
    pub fn new(task: F) -> ServiceFn<F, I, O, E> {
        ServiceFn {
            f: task,
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E, U> Service<I> for ServiceFn<F, I, O, E>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send,
{
    type Output = O;
    type Error = E;
    type Future = U;
    fn call(&self, input: I) -> Self::Future {
        (self.f)(input)
    }
}

impl<F, I, O, E, U> Service<I> for F
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, E>> + Send,
{
    type Output = O;
    type Error = E;
    type Future = FnServiceFuture<U, I, O, E>;
    fn call(&self, input: I) -> Self::Future {
        FnServiceFuture((self)(input), PhantomData)
    }
}

#[pin_project]
pub struct FnServiceFuture<U, I, O, E>(#[pin] U, PhantomData<I>)
where
    U: Future<Output = Result<O, E>>;

impl<U, I, O, E> Future for FnServiceFuture<U, I, O, E>
where
    U: Future<Output = Result<O, E>>,
{
    type Output = Result<O, Rejection<I, E>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match ready!(this.0.poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(err) => Poll::Ready(Err(Rejection::Err(err))),
        }
    }
}

unsafe impl<U: Send, I, O, E> Send for FnServiceFuture<U, I, O, E> where
    U: Future<Output = Result<O, E>>
{
}
