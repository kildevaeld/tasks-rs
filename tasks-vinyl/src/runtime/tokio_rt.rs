use futures_util::TryFutureExt;
use std::future::Future;
use std::io;
use std::path::Path;
use tokio::task::{JoinError, JoinHandle};

pub use tokio::sync::Mutex;

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

pub async fn mkdir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    tokio::fs::create_dir(path).await
}

pub async fn mkdir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    tokio::fs::create_dir_all(path).await
}

pub async fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    tokio::fs::remove_file(path).await
}
