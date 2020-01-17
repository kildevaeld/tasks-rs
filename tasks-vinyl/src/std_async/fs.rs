use crate::error::Error;
use crate::file::{Content, File, FileType, IntoFile, Metadata};
use async_std::fs::{DirEntry, File as AFile, ReadDir, read_dir};
use async_std::prelude::*;
use futures_core::Stream;
use futures_util::future;
use futures_util::{FutureExt, StreamExt};
use futures_util::io::AsyncReadExt;
use pin_project::pin_project;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tasks::{
    task_fn,
    utils::{OneOf2Future, Promise},
    Producer, ProducerExt , Task,
};
use bytes::{Bytes, BytesMut};


#[pin_project]
pub struct DirectoryProducer {
    #[pin]
    inner: ReadDir,
    path: PathBuf,
}



impl DirectoryProducer {

    pub async fn new<S: AsRef<str>>(path: S) -> Result<DirectoryProducer, Error> {
        let reader = read_dir(path.as_ref()).await?;
        let path = PathBuf::from(path.as_ref());
        Ok(DirectoryProducer{
            inner: reader,
            path,
        })
    }

    pub fn into_vinyl(
        self,
    ) -> impl Producer<
        Item = File,
        Error = Error,
        Future = impl Future<Output = Result<File, Error>> + Send, //Pin<Box<dyn Future<Output = Result<File, Error>> + Send>>,
    >{
        let stream = self
            .into_stream()
            .filter_map(|item| {
                async {
                    let item = match item.await {
                        Ok(i) => i,
                        Err(e) => return Some(Err(e.into())),
                    };
                    let meta = match item.metadata().await {
                        Ok(i) => i,
                        Err(e) => return Some(Err(e.into())),
                    };
                    let path = item.path();

                    if meta.is_file() {
                        Some(Ok::<_, Error>((meta, path)))
                    } else {
                        None
                    }
                }
            })
            .then(|item| {
                async move {
                    async move {
                        let item = item?;
                        let mut file = AFile::open(&item.1).await?;

                        let mut b = Vec::default();

                        <AFile as AsyncReadExt>::read_to_end(&mut file, &mut b).await?;

                        let out = File {
                            path: PathBuf::from(item.1.into_os_string()),
                            content: Content::Bytes(Bytes::from(b)),
                            metadata: Metadata {
                                size: item.0.len(),
                                file_type: FileType::File,
                            },
                        };

                        Ok::<_, Error>(out)
                    }
                }
            })
            .boxed();
        
        stream
    }
}

impl Producer for DirectoryProducer {
    type Item = DirEntry;
    type Error = Error;
    type Future = future::Ready<Result<DirEntry, Error>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Future>> {
        match self.project().inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(next))) => {
                // let future = async move {
                //     let m = next.metadata().await?;
                //     let t = next.file_type().await?;

                //     let file = File {
                //         path: PathBuf::from(next.path().into_os_string()),
                //         content: Content::None,
                //         metadata: Metadata {
                //             size: m.len(),
                //             file_type: if m.is_dir() {
                //                 FileType::Dir
                //             } else {
                //                 FileType::File
                //             },
                //         },
                //     };

                //     Ok::<_, Error>(file)
                // }.boxed();

                // let meta = match next.metadata() {
                //     Ok(m) => m,
                //     Err(e) => return Poll::Ready(Some(future::err(Error::IoError(e))))
                // };

                // let file = File {
                //     path: next.path().to_owned(),
                //     content: Content::None,
                //     metadata: Metadata {}
                // };

                Poll::Ready(Some(future::ok(next)))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(future::err(Error::IoError(err)))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}

#[pin_project]
pub struct IntoVinyl<S: Stream<Item = Result<(AFile, PathBuf), Error>>>(#[pin] S);

impl<S: Stream<Item = Result<(AFile, PathBuf), Error>>> Producer for IntoVinyl<S> {
    type Item = File;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Item, Self::Error>> + Send>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Future>> {
        let this = self.project();
        match this.0.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok((file, path)))) => {
                let future = async move {
                    let meta = file.metadata().await?;
                    let out = File {
                        path,
                        content: Content::Stream(Box::pin(file)),
                        metadata: Metadata {
                            size: meta.len(),
                            file_type: FileType::File,
                        },
                    };

                    Ok::<_, Error>(out)
                }
                .boxed();

                Poll::Ready(Some(future))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(future::err(e).boxed())),
        }
    }
}


