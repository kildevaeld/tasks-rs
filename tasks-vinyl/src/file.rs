use crate::Error;
use bytes::Bytes;
use futures_core::stream::{BoxStream, Stream};
use futures_io::AsyncRead;
use futures_util::stream::{self, StreamExt};
use mime::Mime;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

pub enum Content {
    Stream(Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>),
    Bytes(Bytes),
    None,
}

impl Content {
    pub fn into_stream(self) -> BoxStream<'static, Result<Bytes, Error>> {
        match self {
            Content::Stream(s) => s,
            Content::Bytes(b) => stream::iter(vec![Ok(b)]).boxed(),
            Content::None => stream::empty().boxed(),
        }
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
    fn from(bytes: ()) -> Self {
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

// pub trait IntoFile {
//     type Future: Future<Output = File>;
//     fn into_file(self) -> Self::Future;
// }
