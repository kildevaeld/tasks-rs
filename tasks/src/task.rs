use std::future::Future;

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
    type Task: Task<
        I,
        Output = Self::Output,
        Error = Self::Error,
        Future = Self::Future,
    >;

    fn into_task(self) -> Self::Task;
}

impl<T, I> IntoTask<I> for T
where
    T: Task<I>,
{

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
