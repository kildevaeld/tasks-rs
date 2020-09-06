use futures_util::TryFutureExt;
use std::future::Future;
use tokio::task::JoinError;

pub fn spawn<T>(task: T) -> impl Future<Output = Result<T::Output, JoinError>>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    tokio::spawn(task)
}

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(f)
}
