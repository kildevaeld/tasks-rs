use super::error::Error;
use async_trait::async_trait;
use futures_core::Stream;
use futures_io::AsyncRead;
use futures_util::{
    future::BoxFuture, stream::BoxStream, FutureExt, StreamExt, TryFutureExt, TryStreamExt,
};
use mime::Mime;
use std::future::Future;
use std::path::Path;
use tasks_vinyl::{
    util::ByteStream, Content, Error as VinylError, File, VinylStream, VinylStreamDestination,
};
use vfs_async::{OpenOptions, VFile, VMetadata, VPath, VFS};

pub trait Resource {
    type Read: AsyncRead;
    type Future: Future<Output = Result<Self::Read, Error>>;
    fn name(&self) -> &str;
    fn size(&self) -> u64;
    fn mime(&self) -> &Mime;

    fn body(&self) -> Self::Future;
}

#[async_trait]
pub trait ResourceExt: Resource {
    fn vinyl(&self) -> BoxStream<'static, BoxFuture<'static, Result<File, VinylError>>>
    where
        Self: Send,
        Self::Read: Send + 'static,
        Self::Future: Send + 'static,
    {
        let body = self.body();

        let name = self.name().to_string();
        let size = self.size();
        let mime = self.mime().clone();

        futures_util::stream::iter(vec![body
            .map_err(|err| VinylError::Other(Box::new(err)))
            .map_ok(move |body| File {
                content: Content::Stream(Box::pin(
                    ByteStream::new(body).map_err(|err| VinylError::Io(err)),
                )),
                path: name,
                size,
                mime,
            })
            .boxed()])
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
        <Self::Resource as Resource>::Future: Send,
    {
        Ok(self
            .list()
            .await?
            .map(|m| {
                async move {
                    let file = m
                        .body()
                        .await
                        .map_err(|err| VinylError::Other(Box::new(err)))?;
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

    // async fn write_to<D>(&self, dest: D) -> Result<usize, Error>
    // where
    //     D: VinylStreamDestination + Send + Sync + 'static,
    //     D::Future: Send,
    //     Self: Send + 'static,
    //     Self::Stream: Send,
    //     Self::Resource: Sync + Send + 'static,
    //     <Self::Resource as Resource>::Read: Send,
    //     <Self::Resource as Resource>::Future: Send,
    // {
    //     self.vinyl()
    //         .await?
    //         .write_to(dest)
    //         .await
    //         .map_err(Error::from)
    // }
}

impl<T> SourceExt for T where T: Source {}

// pub trait ResourceStream: Stream + Sized {
//     fn vinyl(self) -> BoxStream<'static, Result<File, VinylError>>
//     where
//         Self: Send + 'static,
//         Self::Item: Resource + Send + Sync + 'static,
//         <Self::Item as Resource>::Read: Send,
//         <Self::Item as Resource>::Future: Send,
//     {
//         self.then(|m| async move {
//             let file = m
//                 .body()
//                 .await
//                 .map_err(|err| VinylError::Other(Box::new(err)))?;
//             Result::<_, VinylError>::Ok(File {
//                 content: Content::Stream(Box::pin(
//                     ByteStream::new(file).map_err(|err| VinylError::Io(err)),
//                 )),
//                 path: m.name().to_string(),
//                 size: m.size(),
//                 mime: m.mime().clone(),
//             })
//         })
//         .boxed()
//     }
// }

pub struct VfsResource<P> {
    path: P,
    name: String,
    size: u64,
    mime: Mime,
}

impl<P> Resource for VfsResource<P>
where
    P: VPath,
    P::File: 'static,
{
    type Read = P::File;
    type Future = BoxFuture<'static, Result<Self::Read, Error>>;
    fn name(&self) -> &str {
        &self.name
    }
    fn size(&self) -> u64 {
        self.size
    }
    fn mime(&self) -> &Mime {
        &self.mime
    }

    fn body(&self) -> Self::Future {
        self.path
            .open(OpenOptions::new().read(true))
            .map_err(|err| Error::Io(err))
            .boxed()
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
