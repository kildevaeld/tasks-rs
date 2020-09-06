use crate::runtime;
use crate::{Error, File, Reply, VinylStream};
use futures_core::Stream;
use futures_util::{
    future::BoxFuture, stream::BoxStream, FutureExt, StreamExt, TryFutureExt, TryStreamExt,
};
use std::future::Future;

pub struct Builder {
    streams: Vec<BoxStream<'static, BoxFuture<'static, Result<File, Error>>>>,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            streams: Vec::new(),
        }
    }

    pub fn push<S, R>(mut self, stream: S) -> Self
    where
        S: Send + 'static + Stream,
        S::Item: Send + Future<Output = Result<R, Error>>,
        R: Reply + Send,
        R::Future: Send,
    {
        self.streams.push(
            stream
                .map(|file| {
                    async move {
                        let file = file.await?;
                        Ok(file.into_file().await?)
                    }
                    .boxed()
                })
                .boxed(),
        );
        self
    }

    pub async fn run(self) -> Vec<Result<File, Error>> {
        self.into_stream().buffer_unordered(5).collect().await
    }

    pub fn into_stream(
        self,
    ) -> impl Stream<Item = impl Future<Output = Result<File, Error>> + Send> + Send {
        futures_util::stream::select_all(self.streams)
            .map(|next| async move { runtime::spawn(async move { next.await }).await })
            .map(|ret| async move {
                match ret.await {
                    Ok(s) => s,
                    Err(e) => Err(e),
                }
            })
    }
}
