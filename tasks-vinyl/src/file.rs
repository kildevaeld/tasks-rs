use futures_core::stream::BoxStream;
use bytes::Bytes;
use std::io::Read;
use std::path::PathBuf;
use std::future::Future;
use futures_io::AsyncRead;
use std::pin::Pin;

pub enum Content {
    Stream(Pin<Box<dyn AsyncRead + Send + Unpin>>),
    Reader(Box<dyn Read + Send>),
    Bytes(Bytes),
    None
}

pub enum FileType {
    Dir,
    File
}

pub struct Metadata {
    pub size: u64,
    pub file_type: FileType   
}


pub struct File {
    pub path: PathBuf,
    pub metadata: Metadata,
    pub content: Content
}


pub trait IntoFile {
    type Future: Future<Output = File>;
    fn into_file(self) -> Self::Future;
} 