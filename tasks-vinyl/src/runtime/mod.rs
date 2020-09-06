#[cfg(feature = "tokio")]
mod tokio_rt;

#[cfg(feature = "tokio")]
pub use self::tokio_rt::*;

#[cfg(all(not(feature = "tokio")))]
pub fn spawn<T>(
    task: T,
) -> impl std::future::Future<Output = Result<T::Output, crate::error::Error>>
where
    T: std::future::Future + Send + 'static,
    T::Output: Send + 'static,
{
    use futures_util::FutureExt;
    task.map(|s| Ok(s))
}
