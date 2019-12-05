pub trait SyncTask<INPUT> {
    type Output;
    type Error;
    fn exec(&self, input: INPUT) -> Result<Self::Output, Self::Error>;
    fn can_exec(&self, input: &INPUT) -> bool {
        true
    }
}

// pub trait ConditionalSyncTask: SyncTask {
//     fn can_exec(&self, input: &Self::Input) -> bool;
// }

pub trait IntoSyncTask<INPUT> {
    type Output;
    type Error;
    type Task: SyncTask<INPUT, Output = Self::Output, Error = Self::Error>;

    fn into_task(self) -> Self::Task;
}

impl<T, I> IntoSyncTask<I> for T
where
    T: SyncTask<I>,
{
    type Output = T::Output;
    type Error = T::Error;
    type Task = T;
    fn into_task(self) -> Self::Task {
        self
    }
}

// pub trait IntoConditionalSyncTask {
//     type Input;
//     type Output;
//     type Error;
//     type Task: ConditionalSyncTask<
//         Input = Self::Input,
//         Output = Self::Output,
//         Error = Self::Error,
//     >;

//     fn into_task(self) -> Self::Task;
// }

// impl<T> IntoConditionalSyncTask for T
// where
//     T: ConditionalSyncTask,
// {
//     type Input = T::Input;
//     type Output = T::Output;
//     type Error = T::Error;
//     type Task = T;
//     fn into_task(self) -> Self::Task {
//         self
//     }
// }

#[derive(Clone)]
pub struct SyncTaskFn<F, I, O, E, C> {
    inner: F,
    _i: std::marker::PhantomData<I>,
    _o: std::marker::PhantomData<O>,
    _e: std::marker::PhantomData<E>,
    check: C,
}

// impl<F, I, O, E> SyncTaskFn<F, I, O, E, ()>
// where
//     F: Fn(I) -> Result<O, E>,
// {
//     pub fn new(service: F) -> SyncTaskFn<F, I, O, E, ()> {
//         SyncTaskFn {
//             inner: service,
//             _i: std::marker::PhantomData,
//             _o: std::marker::PhantomData,
//             _e: std::marker::PhantomData,
//             check: (),
//         }
//     }
// }

impl<F, I, O, E, C> SyncTaskFn<F, I, O, E, C>
where
    F: Fn(I) -> Result<O, E>,
    C: Fn(&I) -> bool,
{
    pub fn new(service: F, check: C) -> SyncTaskFn<F, I, O, E, C> {
        SyncTaskFn {
            inner: service,
            _i: std::marker::PhantomData,
            _o: std::marker::PhantomData,
            _e: std::marker::PhantomData,
            check,
        }
    }
}

impl<F, I, O, E, C> SyncTask<I> for SyncTaskFn<F, I, O, E, C>
where
    F: Fn(I) -> Result<O, E>,
    C: Fn(&I) -> bool,
{
    type Output = O;
    type Error = E;

    fn exec(&self, input: I) -> Result<Self::Output, Self::Error> {
        (self.inner)(input)
    }

    fn can_exec(&self, input: &I) -> bool {
        (self.check)(input)
    }
}

// impl<F, I, O, E, C> ConditionalSyncTask for SyncTaskFn<F, I, O, E, C>
// where
//     F: Fn(I) -> Result<O, E>,
//     C: Fn(&I) -> bool,
// {
//     fn can_exec(&self, input: &I) -> bool {
//         (self.check)(input)
//     }
// }
