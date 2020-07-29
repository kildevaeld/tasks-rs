use super::{Rejection, Task};
use std::future::Future;
use std::marker::PhantomData;

pub trait Middleware<R, T: Task<R>> {
    type Task: Task<R>;

    fn wrap(&self, task: T) -> Self::Task;
}

pub struct MiddlewareFn<R, F, T> {
    cb: F,
    _r: PhantomData<R>,
    _t: PhantomData<T>,
}

impl<R, F: Clone, T> Clone for MiddlewareFn<R, F, T> {
    fn clone(&self) -> Self {
        MiddlewareFn {
            cb: self.cb.clone(),
            _r: PhantomData,
            _t: PhantomData,
        }
    }
}

impl<R, F: Copy, T> Copy for MiddlewareFn<R, F, T> {}

impl<R, F, T> MiddlewareFn<R, F, T> {
    pub fn new(cb: F) -> MiddlewareFn<R, F, T> {
        MiddlewareFn {
            cb,
            _r: PhantomData,
            _t: PhantomData,
        }
    }
}

impl<R, F, T, U, O, E> Middleware<R, T> for MiddlewareFn<R, F, T>
where
    T: Task<R> + Clone,
    F: Clone + Fn(T, R) -> U,
    U: Send + Future<Output = Result<O, Rejection<R, E>>>,
{
    type Task = MiddlewareFnTask<R, F, T>;
    fn wrap(&self, task: T) -> Self::Task {
        MiddlewareFnTask {
            task,
            cb: self.cb.clone(),
            _a: PhantomData,
        }
    }
}

pub struct MiddlewareFnTask<R, F, T> {
    task: T,
    cb: F,
    _a: std::marker::PhantomData<R>,
}

impl<R, F, T, U, O, E> Task<R> for MiddlewareFnTask<R, F, T>
where
    T: Task<R> + Clone,
    F: Fn(T, R) -> U,
    U: Send + Future<Output = Result<O, Rejection<R, E>>>,
{
    type Output = O;
    type Error = E;
    type Future = U;

    fn run(&self, req: R) -> Self::Future {
        (self.cb)(self.task.clone(), req)
    }
}

impl<R, F: Clone, T: Clone> Clone for MiddlewareFnTask<R, F, T> {
    fn clone(&self) -> Self {
        MiddlewareFnTask {
            cb: self.cb.clone(),
            task: self.task.clone(),
            _a: PhantomData,
        }
    }
}

impl<R, F: Copy, T: Copy> Copy for MiddlewareFnTask<R, F, T> {}
