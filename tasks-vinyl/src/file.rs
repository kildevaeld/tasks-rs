use crate::Error;
use bytes::{Bytes, BytesMut};
use futures_core::{
    future::BoxFuture,
    stream::{BoxStream, Stream},
};
use futures_util::stream::{self, StreamExt, TryStreamExt};
use mime::Mime;
use std::fmt;

pub trait Opener: Send + Sync {
    fn open(&self) -> BoxFuture<'static, Result<BoxStream<'static, Result<Bytes, Error>>, Error>>;
}

pub enum Content {
    Stream(BoxStream<'static, Result<Bytes, Error>>),
    Bytes(Bytes),
    Ref(Box<dyn Opener>),
    None,
}

impl Content {
    pub async fn into_stream(self) -> Result<BoxStream<'static, Result<Bytes, Error>>, Error> {
        match self {
            Content::Stream(s) => Ok(s),
            Content::Bytes(b) => Ok(stream::iter(vec![Ok(b)]).boxed()),
            Content::Ref(o) => Ok(o.open().await?),
            Content::None => Ok(stream::empty().boxed()),
        }
    }

    pub async fn read(self) -> Result<Bytes, Error> {
        let stream = self.into_stream().await?;
        let data = stream
            .try_fold(BytesMut::new(), |mut prev, cur| async move {
                prev.extend(cur.to_vec());
                Ok(prev)
            })
            .await?;

        Ok(Bytes::from(data))
    }

    pub fn from_stream<S>(stream: S) -> Content
    where
        S: Stream<Item = Result<Bytes, Error>> + Send + 'static,
    {
        Content::Stream(Box::pin(stream))
    }
}

impl From<Bytes> for Content {
    fn from(bytes: Bytes) -> Self {
        Content::Bytes(bytes)
    }
}

impl From<String> for Content {
    fn from(bytes: String) -> Self {
        Content::Bytes(Bytes::from(bytes))
    }
}

impl From<&'static str> for Content {
    fn from(bytes: &'static str) -> Self {
        Content::Bytes(Bytes::from(bytes))
    }
}

impl From<()> for Content {
    fn from(_: ()) -> Self {
        Content::None
    }
}

impl fmt::Debug for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Content")
    }
}

#[derive(Debug)]
pub struct File {
    pub path: String,
    pub content: Content,
    pub mime: Mime,
    pub size: u64,
}

impl File {
    pub fn new(
        path: impl ToString,
        content: impl Into<Content>,
        mime: impl Into<Mime>,
        size: u64,
    ) -> File {
        File {
            path: path.to_string(),
            content: content.into(),
            mime: mime.into(),
            size,
        }
    }
}
