use futures_util::future::Ready;
use std::future::Future;
use std::marker::PhantomData;

pub trait Task<Input> {
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Self::Error>> + Send + 'static;

    fn exec(&self, input: Input) -> Self::Future;
    fn can_exec(&self, input: &Input) -> bool;
}

pub trait IntoTask<I> {
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Self::Error>> + Send + 'static;
    type Task: Task<I, Output = Self::Output, Error = Self::Error, Future = Self::Future>;

    fn into_task(self) -> Self::Task;
}

impl<T: Task<I>, I> IntoTask<I> for T {
    type Output = T::Output;
    type Error = T::Error;
    type Future = T::Future;
    type Task = T;
    fn into_task(self) -> Self::Task {
        self
    }
}

#[derive(Clone)]
pub struct TaskFn<F, I, O, E, C> {
    inner: F,
    _i: std::marker::PhantomData<I>,
    _o: std::marker::PhantomData<O>,
    _e: std::marker::PhantomData<E>,
    check: C,
}

impl<F, I, O, E, C, U> TaskFn<F, I, O, E, C>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, E>> + Send + 'static,
    C: Fn(&I) -> bool,
{
    pub fn new(service: F, check: C) -> TaskFn<F, I, O, E, C> {
        TaskFn {
            inner: service,
            _i: std::marker::PhantomData,
            _o: std::marker::PhantomData,
            _e: std::marker::PhantomData,
            check,
        }
    }
}

impl<F, I, O, E, C, U> Task<I> for TaskFn<F, I, O, E, C>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, E>> + Send + 'static,
    C: Fn(&I) -> bool,
{
    type Output = O;
    type Error = E;
    type Future = U;

    fn exec(&self, input: I) -> Self::Future {
        (self.inner)(input)
    }

    fn can_exec(&self, input: &I) -> bool {
        (self.check)(input)
    }
}

pub struct Transform<FROM, TO, ERROR, C>(C, PhantomData<FROM>, PhantomData<TO>, PhantomData<ERROR>);

impl<FROM, TO, ERROR, C> Transform<FROM, TO, ERROR, C> {
    pub fn new(trans: C) -> Transform<FROM, TO, ERROR, C> {
        Transform(trans, PhantomData, PhantomData, PhantomData)
    }
}

impl<FROM: Send + Sync + 'static, TO: Send + 'static, ERROR: Send + 'static, C> Task<FROM>
    for Transform<FROM, TO, ERROR, C>
where
    C: Fn(FROM) -> TO,
{
    type Output = TO;
    type Error = ERROR;
    type Future = Ready<Result<TO, ERROR>>;

    #[inline]
    fn exec(&self, input: FROM) -> Self::Future {
        let out = (self.0)(input);
        futures_util::future::ok(out)
    }

    #[inline]
    fn can_exec(&self, _input: &FROM) -> bool {
        true
    }
}


pub enum Rejection<R, E> {
    Error(E),
    Reject(R)
}