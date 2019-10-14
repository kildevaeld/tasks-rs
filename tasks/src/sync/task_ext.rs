use super::task::{SyncTask, ConditionalSyncTask, IntoConditionalSyncTask, IntoSyncTask};
use threadpool::ThreadPool;
use super::pool::Pool;
use num_cpus;
use super::pipe::SyncPipe;
use super::either::Either;
use crate::error::TaskError;

pub trait SyncTaskExt: SyncTask + Sized {
    fn into_async(self, threadpool: Option<ThreadPool>) ->  Pool<Self>;
    fn pipe<T: IntoSyncTask>(self, next: T) -> SyncPipe<Self, T::Task>;
}


impl<T> SyncTaskExt for T 
where 
    T: SyncTask + Clone + Send + 'static,
    <T as SyncTask>::Input: Send,
    <T as SyncTask>::Output: Send,
    <T as SyncTask>::Error: From<TaskError> + Send
    
     {
    fn into_async(self, threadpool: Option<ThreadPool>) -> Pool<Self> {
        match threadpool {
            Some(tp) => Pool::with_pool(tp, self),
            None => Pool::new(num_cpus::get(), self)
        }
    }

    fn pipe<P: IntoSyncTask>(self, next: P) -> SyncPipe<Self, P::Task> {
        SyncPipe{
            s1: self,
            s2: next.into_task()
        }
    }
}

pub trait ConditionalSyncTaskExt: ConditionalSyncTask + Sized {
    fn or<
        S: IntoConditionalSyncTask<Input = Self::Input, Output = Self::Output, Error = Self::Error>,
    >(
        self,
        service: S,
    ) -> Either<Self, S::Task> {
        Either::new(self, service.into_task())
    }
}

impl<T> ConditionalSyncTaskExt for T where T: ConditionalSyncTask {}
