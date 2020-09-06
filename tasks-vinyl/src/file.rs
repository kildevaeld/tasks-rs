use crate::Error;
use bytes::Bytes;
use futures_core::stream::{BoxStream, Stream};
use futures_io::AsyncRead;
use futures_util::stream::{self, StreamExt};
use mime::Mime;
use std::fmt;
use std::future::Future;
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::Mutex;

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
}

impl fmt::Debug for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Content")
    }
}

// #[derive(Debug, Clone, T)]
// pub enum FileType {
//     Dir,
//     File,
// }

// pub struct Metadata {
//     pub size: u64,
//     pub file_type: FileType,
// }

#[derive(Debug)]
pub struct File {
    pub path: String,
    pub content: Content,
    pub mime: Mime,
    pub size: u64,
}

pub trait IntoFile {
    type Future: Future<Output = File>;
    fn into_file(self) -> Self::Future;
}
