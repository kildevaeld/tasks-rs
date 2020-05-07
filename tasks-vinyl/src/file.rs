use crate::Error;
use bytes::Bytes;
use futures_core::stream::BoxStream;
use futures_core::Stream;
use futures_io::AsyncRead;
use std::fmt;
use std::future::Future;
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;

pub enum Content {
    Stream(Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send>>),
    Reader(Box<dyn Read + Send>),
    Bytes(Bytes),
    None,
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
}

pub trait IntoFile {
    type Future: Future<Output = File>;
    fn into_file(self) -> Self::Future;
}
