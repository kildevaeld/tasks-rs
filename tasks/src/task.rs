use futures_core::ready;
use pin_project::pin_project;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, PartialEq)]
pub enum Rejection<R, E> {
    Err(E),
    Reject(R, Option<E>),
}

pub trait Task<R> {
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send;

    fn run(&self, req: R) -> Self::Future;
}

pub struct TaskFn<F, I, O, E> {
    f: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
    _e: PhantomData<E>,
}

impl<F, I, O, E> Clone for TaskFn<F, I, O, E>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        TaskFn {
            f: self.f.clone(),
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E> Copy for TaskFn<F, I, O, E> where F: Copy {}

impl<F, I, O, E, U> TaskFn<F, I, O, E>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send + 'static,
{
    pub fn new(task: F) -> TaskFn<F, I, O, E> {
        TaskFn {
            f: task,
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E, U> Task<I> for TaskFn<F, I, O, E>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send,
{
    type Output = O;
    type Error = E;
    type Future = U;
    fn run(&self, input: I) -> Self::Future {
        (self.f)(input)
    }
}

pub struct Reject<T>(T);

impl<T> Reject<T> {
    pub fn new(task: T) -> Reject<T> {
        Reject(task)
    }
}

impl<T, R> Task<R> for Reject<T>
where
    T: Task<R>,
{
    type Output = T::Output;
    type Error = T::Error;
    type Future = RejectFuture<T, R>;
    fn run(&self, req: R) -> Self::Future {
        RejectFuture(self.0.run(req))
    }
}

#[pin_project]
pub struct RejectFuture<T: Task<R>, R>(#[pin] T::Future);

impl<T, R> Future for RejectFuture<T, R>
where
    T: Task<R>,
{
    type Output = Result<T::Output, Rejection<R, T::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        match ready!(this.0.poll(cx)) {
            Ok(s) => Poll::Ready(Ok(s)),
            Err(Rejection::Err(e)) => Poll::Ready(Err(Rejection::Err(e))),
            Err(Rejection::Reject(_, Some(e))) => Poll::Ready(Err(Rejection::Err(e))),
            Err(Rejection::Reject(r, None)) => Poll::Ready(Err(Rejection::Reject(r, None))),
        }
    }
}
