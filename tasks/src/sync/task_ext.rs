use super::either::EitherSync;
use super::pipe::SyncPipe;
use super::pool::Pool;
use super::task::{IntoSyncTask, SyncTask};
use crate::error::TaskError;
use num_cpus;
use threadpool::ThreadPool;

pub trait SyncTaskExt: SyncTask + Sized {
    fn into_async(self, threadpool: Option<ThreadPool>) -> Pool<Self>;
    fn pipe<T: IntoSyncTask<Input = Self::Output, Error = Self::Error>>(
        self,
        next: T,
    ) -> SyncPipe<Self, T::Task> {
        SyncPipe {
            s1: self,
            s2: next.into_task(),
        }
    }

    fn or<
        S: IntoSyncTask<Input = Self::Input, Output = Self::Output, Error = Self::Error>,
    >(
        self,
        service: S,
    ) -> EitherSync<Self, S::Task> {
        EitherSync::new(self, service.into_task())
    }
}

impl<T> SyncTaskExt for T
where
    T: SyncTask + Sync + Send + 'static,
    <T as SyncTask>::Input: Send,
    <T as SyncTask>::Output: Send,
    <T as SyncTask>::Error: From<TaskError> + Send,
{
    fn into_async(self, threadpool: Option<ThreadPool>) -> Pool<Self> {
        match threadpool {
            Some(tp) => Pool::with_pool(tp, self),
            None => Pool::new(num_cpus::get(), self),
        }
    }
}



#[cfg(test)]
mod tests {

    use super::super::SyncTask;
    use super::SyncTaskExt;
    use crate::{TaskError, Task};

    #[test]
    fn test_sync_pipe() {
        let task = sync_task_fn!(|s: String| Result::<_, TaskError>::Ok(s + " Total"));

        let task = task.pipe(sync_task_fn!(|s: String| Ok(s + " Control")));

        let out = task.exec("Hello, World".into());
        assert_eq!(out, Ok(String::from("Hello, World Total Control")));
    }


     #[test]
    fn test_sync_pool() {
        let task = sync_task_fn!(|s: String| Result::<_, TaskError>::Ok(s + " Total"));

        let task = task.pipe(sync_task_fn!(|s: String| Ok(s + " Control")));
        let pool = task.into_async(None);

        let result = futures_executor::block_on(pool.exec("Hello, World".to_string()));

        assert_eq!(result, Ok(String::from("Hello, World Total Control")));
    }
}
