use super::error::Error;
use async_trait::async_trait;
use futures_core::Stream;
use futures_io::AsyncRead;
use futures_util::{future::BoxFuture, stream::BoxStream, FutureExt, StreamExt, TryStreamExt};
use mime::Mime;
use std::path::Path;
use tasks_vinyl::{util::ByteStream, Content, Error as VinylError, File, IntoFile};
use vfs_async::{OpenOptions, VFile, VMetadata, VPath, VFS};
#[async_trait]
pub trait Resource {
    type Read: AsyncRead;
    fn name(&self) -> &str;
    fn size(&self) -> u64;
    fn mime(&self) -> &Mime;

    async fn body(&self) -> Self::Read;
}

#[async_trait]
pub trait ResourceExt: Resource {
    async fn vinyl(&self) -> BoxStream<'static, Result<File, VinylError>>
    where
        Self::Read: Send + 'static,
    {
        let file = self.body().await;
        futures_util::stream::iter(vec![Ok(File {
            content: Content::Stream(Box::pin(
                ByteStream::new(file).map_err(|err| VinylError::Io(err)),
            )),
            path: self.name().to_string(),
            size: self.size(),
            mime: self.mime().clone(),
        })])
        .boxed()
    }
}

impl<T> ResourceExt for T where T: Resource {}

#[async_trait]
pub trait Source {
    type Resource: Resource;
    type Stream: Stream<Item = Self::Resource>;

    async fn list(&self) -> Result<Self::Stream, Error>;
    async fn resource(&self, name: &str) -> Result<Self::Resource, Error>;
    async fn contains(&self, name: &str) -> bool;
}

#[async_trait]
pub trait SourceExt: Source {
    async fn vinyl(
        &self,
    ) -> Result<BoxStream<'static, BoxFuture<'static, Result<File, VinylError>>>, Error>
    where
        Self: Send + 'static,
        Self::Stream: Send,
        Self::Resource: Sync + Send + 'static,
        <Self::Resource as Resource>::Read: Send,
    {
        Ok(self
            .list()
            .await?
            .map(|m| {
                async move {
                    let file = m.body().await;
                    Result::<_, VinylError>::Ok(File {
                        content: Content::Stream(Box::pin(
                            ByteStream::new(file).map_err(|err| VinylError::Io(err)),
                        )),
                        path: m.name().to_string(),
                        size: m.size(),
                        mime: m.mime().clone(),
                    })
                }
                .boxed()
            })
            .boxed())
    }
}

impl<T> SourceExt for T where T: Source {}

pub trait ResourceStream: Stream + Sized {
    fn vinyl(self) -> BoxStream<'static, Result<File, VinylError>>
    where
        Self: Send + 'static,
        Self::Item: Resource + Send + Sync + 'static,
        <Self::Item as Resource>::Read: Send,
    {
        self.then(|m| async move {
            let file = m.body().await;
            Result::<_, VinylError>::Ok(File {
                content: Content::Stream(Box::pin(
                    ByteStream::new(file).map_err(|err| VinylError::Io(err)),
                )),
                path: m.name().to_string(),
                size: m.size(),
                mime: m.mime().clone(),
            })
        })
        .boxed()
    }
}

pub struct VfsResource<P> {
    path: P,
    name: String,
    size: u64,
    mime: Mime,
}

#[async_trait]
impl<P> Resource for VfsResource<P>
where
    P: VPath,
{
    type Read = P::File;
    fn name(&self) -> &str {
        &self.name
    }
    fn size(&self) -> u64 {
        self.size
    }
    fn mime(&self) -> &Mime {
        &self.mime
    }

    async fn body(&self) -> Self::Read {
        self.path.open(OpenOptions::new().read(true)).await.unwrap()
    }
}

pub struct VfsSource<V>(V);

impl<V> VfsSource<V> {
    pub fn new(vfs: V) -> VfsSource<V> {
        VfsSource(vfs)
    }
}

#[async_trait]
impl<V> Source for VfsSource<V>
where
    V: VFS,
    <V::Path as VPath>::Metadata: Send,
    <V::Path as VPath>::ReadDir: Send,
    V::Path: std::marker::Unpin + 'static,
{
    type Resource = VfsResource<V::Path>;
    type Stream = BoxStream<'static, Self::Resource>;

    async fn list(&self) -> Result<Self::Stream, Error> {
        Ok(vfs_async::walkdir(self.0.path("."))
            .await?
            .and_then(|file| async move {
                let meta = file.metadata().await?;
                let name = file.to_string();
                let mime = mime_guess::from_path(name.as_ref()).first_or_octet_stream();
                Ok(VfsResource {
                    name: name.to_string(),
                    path: file,
                    size: meta.len(),
                    mime,
                })
            })
            .filter_map(|m| async move {
                match m {
                    Ok(s) => Some(s),
                    Err(_) => None,
                }
            })
            .boxed())
    }
    async fn resource(&self, name: &str) -> Result<Self::Resource, Error> {
        let path = self.0.path(name);
        let meta = path.metadata().await?;
        if meta.is_dir() {
            Err(Error::NotFound)
        } else {
            let name = path.to_string();
            let mime = mime_guess::from_path(name.as_ref()).first_or_octet_stream();
            Ok(VfsResource {
                name: name.to_string(),
                path: path,
                size: meta.len(),
                mime,
            })
        }
    }
    async fn contains(&self, name: &str) -> bool {
        self.0.path(name).exists().await
    }
}
