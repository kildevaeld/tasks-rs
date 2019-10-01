use std::future::Future;

pub trait Task {
    type Input;
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Self::Error>> + Send + 'static;

    fn exec(&self, input: Self::Input) -> Self::Future;
}

pub trait ConditionalTask: Task {
    fn can_exec(&self, input: &Self::Input) -> bool;
}

pub trait IntoTask {
    type Input;
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Self::Error>> + Send + 'static;
    type Task: Task<
        Input = Self::Input,
        Output = Self::Output,
        Error = Self::Error,
        Future = Self::Future,
    >;

    fn into_task(self) -> Self::Task;
}

impl<T> IntoTask for T
where
    T: Task,
{
    type Input = T::Input;
    type Output = T::Output;
    type Error = T::Error;
    type Future = T::Future;
    type Task = T;
    fn into_task(self) -> Self::Task {
        self
    }
}

pub trait IntoConditionalTask {
    type Input;
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Self::Error>> + Send + 'static;
    type Task: ConditionalTask<
        Input = Self::Input,
        Output = Self::Output,
        Error = Self::Error,
        Future = Self::Future,
    >;

    fn into_task(self) -> Self::Task;
}

impl<T> IntoConditionalTask for T
where
    T: ConditionalTask,
{
    type Input = T::Input;
    type Output = T::Output;
    type Error = T::Error;
    type Future = T::Future;
    type Task = T;
    fn into_task(self) -> Self::Task {
        self
    }
}

pub struct TaskFn<F, I, O, E, C> {
    inner: F,
    _i: std::marker::PhantomData<I>,
    _o: std::marker::PhantomData<O>,
    _e: std::marker::PhantomData<E>,
    check: C,
}

impl<F, I, O, E, U> TaskFn<F, I, O, E, ()>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, E>> + Send + 'static,
{
    pub fn new(service: F) -> TaskFn<F, I, O, E, ()> {
        TaskFn {
            inner: service,
            _i: std::marker::PhantomData,
            _o: std::marker::PhantomData,
            _e: std::marker::PhantomData,
            check: (),
        }
    }
}

impl<F, I, O, E, C, U> TaskFn<F, I, O, E, C>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, E>> + Send + 'static,
    C: Fn(&I) -> bool,
{
    pub fn with_check(service: F, check: C) -> TaskFn<F, I, O, E, C> {
        TaskFn {
            inner: service,
            _i: std::marker::PhantomData,
            _o: std::marker::PhantomData,
            _e: std::marker::PhantomData,
            check,
        }
    }
}


impl<F, I, O, E, C, U> Task for TaskFn<F, I, O, E, C>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, E>> + Send + 'static,
{
    type Input = I;
    type Output = O;
    type Error = E;
    type Future = U;

    fn exec(&self, input: I) -> Self::Future {
        (self.inner)(input)
    }
}

impl<F, I, O, E, C, U> ConditionalTask for TaskFn<F, I, O, E, C>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, E>> + Send + 'static,
    C: Fn(&I) -> bool,
{
    fn can_exec(&self, input: &I) -> bool {
        (self.check)(input)
    }
}

