use std::future::Future;
use std::marker::PhantomData;

#[derive(Debug, PartialEq)]
pub enum Rejection<R, E> {
    Err(E),
    Reject(R),
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
