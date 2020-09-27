use futures_io::{AsyncRead, AsyncSeek, AsyncWrite, IoSlice, IoSliceMut};
use futures_util::TryFutureExt;
use pin_project::pin_project;
use std::fs::Metadata;
use std::fs::OpenOptions;
use std::future::Future;
use std::io::{self, SeekFrom};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::{File as TokioFile, ReadDir};
use tokio::io::{AsyncRead as TAsyncRead, AsyncWrite as TAsyncWrite};
pub use tokio::sync::Mutex;
use tokio::task::{JoinError, JoinHandle};

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

pub async fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    tokio::fs::metadata(path).await
}

pub async fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    tokio::fs::read_dir(path).await
}

pub async fn create_file<P: AsRef<Path>>(path: P) -> io::Result<File> {
    TokioFile::create(path).await.map(File)
}

pub async fn open_file<P: AsRef<Path>>(path: P) -> io::Result<File> {
    TokioFile::open(path).await.map(File)
}

#[pin_project]
pub struct File(#[pin] TokioFile);

impl AsyncRead for File {
    #[cfg(feature = "read-initializer")]
    unsafe fn initializer(&self) -> Initializer {
        self.0.initializer()
    }

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.project().0.poll_read(cx, buf)
    }
}

impl AsyncWrite for File {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.project().0.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().0.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().0.poll_shutdown(cx)
    }
}
