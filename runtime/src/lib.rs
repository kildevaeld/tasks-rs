mod error;

pub use self::error::*;

#[cfg(feature = "tokio")]
mod tokio_impl {
    use crate::SpawnError;
    use std::future::Future;

    pub async fn spawn<T>(future: T) -> Result<T::Output, SpawnError>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        tokio::spawn(future).await.map_err(|err| SpawnError {
            inner: Box::new(err),
        })
    }

    pub async fn spawn_blocking<F, R>(task: F) -> Result<R, SpawnError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        tokio::task::spawn_blocking(task)
            .await
            .map_err(|err| SpawnError {
                inner: Box::new(err),
            })
    }

    #[cfg(feature = "time")]
    pub async fn interval(duration: std::time::Duration) -> impl futures_core::Stream<Item = ()> {
        use futures_util::StreamExt;
        tokio::time::interval(duration).map(|_| ())
    }
}

#[cfg(feature = "smol")]
mod smol_impl {
    use crate::SpawnError;
    use std::future::Future;

    pub async fn spawn<T>(future: T) -> Result<T::Output, SpawnError>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        Ok(smol::spawn(future).await)
    }

    pub async fn spawn_blocking<F, R>(task: F) -> Result<R, SpawnError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        Ok(smol::unblock(task).await)
    }
}

#[cfg(feature = "async-std")]
mod async_impl {
    use crate::SpawnError;
    use std::future::Future;

    pub async fn spawn<T>(future: T) -> Result<T::Output, SpawnError>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        async_std::task::spawn(future).await
    }

    pub async fn spawn_blocking<F, R>(task: F) -> Result<R, SpawnError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        async_std::task::spawn_blocking(task).await
    }

    #[cfg(feature = "time")]
    pub async fn interval(
        duration: std::time::Duration,
    ) -> impl futures_core::Stream<Item = std::time::Instant> {
        async_std::stream::interval(duration)
    }
}

#[cfg(feature = "tokio")]
pub use tokio_impl::*;

#[cfg(feature = "smol")]
pub use smol_impl::*;

#[cfg(feature = "async")]
pub use async_impl::*;

#[cfg(all(
    not(feature = "tokio"),
    not(feature = "smol"),
    not(feature = "async-std")
))]
mod default {
    use crate::SpawnError;
    use std::future::Future;
    #[allow(unused)]
    pub async fn spawn<T>(future: T) -> Result<T::Output, SpawnError>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        panic!("no runtime specified. enable one of features: tokio, smol, async");
    }

    #[allow(unused)]
    pub async fn spawn_blocking<F, R>(task: F) -> Result<R, SpawnError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        panic!("no runtime specified. enable one of features: tokio, smol, async")
    }
}

#[cfg(all(
    not(feature = "tokio"),
    not(feature = "smol"),
    not(feature = "async-std")
))]
pub use default::*;
