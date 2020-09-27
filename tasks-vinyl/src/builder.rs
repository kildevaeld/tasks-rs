use crate::runtime;
use crate::{Discard, Error, File, Reply, Vector, VinylStream};
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
        R: Reply<Error = Error> + Send,
        R::Future: Send + 'static,
    {
        self.streams.push(
            stream
                .map(|file| file.and_then(|file| file.into_file()).boxed())
                .boxed(),
        );
        self
    }

    pub async fn run(self) -> Result<Vec<Result<File, Error>>, Error> {
        self.into_stream().write_to(Vector::default()).await
    }

    pub fn into_stream(
        self,
    ) -> impl Stream<Item = impl Future<Output = Result<File, Error>> + Send> + Send {
        futures_util::stream::select_all(self.streams)
    }
}
