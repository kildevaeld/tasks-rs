use super::{Rejection, Task};
use std::future::Future;
use std::marker::PhantomData;

pub struct TaskStateFn<F, S, I, O, E> {
    f: F,
    s: S,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
    _e: PhantomData<E>,
}

impl<F, S, I, O, E> Clone for TaskStateFn<F, S, I, O, E>
where
    F: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        TaskStateFn {
            f: self.f.clone(),
            s: self.s.clone(),
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, S, I, O, E> Copy for TaskStateFn<F, S, I, O, E>
where
    F: Copy,
    S: Copy,
{
}

impl<F, S, I, O, E, U> TaskStateFn<F, S, I, O, E>
where
    F: Fn(S, I) -> U,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send + 'static,
{
    pub fn new(state: S, task: F) -> TaskStateFn<F, S, I, O, E> {
        TaskStateFn {
            f: task,
            s: state,
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, S, I, O, E, U> Task<I> for TaskStateFn<F, S, I, O, E>
where
    F: Fn(S, I) -> U,
    S: Clone,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send,
{
    type Output = O;
    type Error = E;
    type Future = U;
    fn run(&self, input: I) -> Self::Future {
        (self.f)(self.s.clone(), input)
    }
}
