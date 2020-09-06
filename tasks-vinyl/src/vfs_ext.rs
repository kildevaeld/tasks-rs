use super::error::Error;
use super::util::ByteStream;
use super::{Content, File, IntoVinylStreamDestination, VinylStreamDestination};
use futures_util::{
    future::BoxFuture, pin_mut, stream::BoxStream, AsyncReadExt, AsyncWriteExt, FutureExt,
    StreamExt, TryStreamExt,
};
use mime_guess;
use vfs_async::{Globber, OpenOptions, VMetadata, VPath, VFS};

pub trait VPathExt: VPath {
    fn vinyl(&self) -> BoxFuture<'static, Result<BoxStream<'static, Result<File, Error>>, Error>>
    where
        Self: 'static,
        Self::ReadDir: Send + 'static,
        Self::Metadata: Send,
        Self::File: 'static,
    {
        let read_dir = self.read_dir();
        async move {
            let stream = read_dir.await?;
            Ok(stream
                .map_err(|err| Error::Io(err))
                .try_filter_map(|path| async move {
                    let meta = path.metadata().await?;
                    if meta.is_dir() {
                        return Ok(None);
                    }

                    let content = path.open(OpenOptions::new().read(true)).await?;
                    let stream = ByteStream::new(content).map_err(|e| e.into());
                    Ok(Some(File {
                        path: path.to_string().as_ref().to_owned(),
                        content: Content::Stream(Box::pin(stream)),
                        size: meta.len(),
                        mime: mime_guess::from_path(path.file_name().unwrap_or(String::from("")))
                            .first_or_octet_stream(),
                    }))
                })
                .boxed())
        }
        .boxed()
    }

    fn vinyl_glob(
        &self,
        glob: impl Into<Globber>,
    ) -> BoxFuture<'static, Result<BoxStream<'static, Result<File, Error>>, Error>>
    where
        Self: 'static + std::marker::Unpin,
        Self::ReadDir: Send + 'static,
        Self::Metadata: Send,
        Self::File: 'static,
    {
        let read_dir = vfs_async::glob(self.clone(), glob.into());
        async move {
            let stream = read_dir.await?;
            Ok(stream
                .map_err(|err| Error::Io(err))
                .try_filter_map(|path| async move {
                    let meta = path.metadata().await?;
                    if meta.is_dir() {
                        return Ok(None);
                    }

                    let content = path.open(OpenOptions::new().read(true)).await?;
                    let stream = ByteStream::new(content).map_err(|e| e.into());
                    Ok(Some(File {
                        path: path.to_string().as_ref().to_owned(),
                        content: Content::Stream(Box::pin(stream)),
                        size: meta.len(),
                        mime: mime_guess::from_path(path.file_name().unwrap_or(String::from("")))
                            .first_or_octet_stream(),
                    }))
                })
                .boxed())
        }
        .boxed()
    }

    fn to_dest(self) -> VPathDest<Self> {
        VPathDest {
            path: self,
            overwrite: false,
        }
    }
}

impl<T> VPathExt for T where T: VPath {}

pub struct VPathDest<T> {
    path: T,
    overwrite: bool,
}

impl<T> VinylStreamDestination for VPathDest<T>
where
    T: VPath + 'static,
{
    type Future = BoxFuture<'static, Result<(), Error>>;
    fn write(&self, file: File) -> Self::Future {
        let path = self.path.clone();
        let overwrite = self.overwrite;
        async move {
            let path = path.resolve(&file.path);
            if let Some(parent) = path.parent() {
                if !parent.exists().await {
                    parent.mkdir().await?;
                }
            }
            if path.exists().await && !overwrite {
                return Err(Error::FileAlreadyExists);
            } else {
                let dest_file = path
                    .open(OpenOptions::new().create(true).write(true))
                    .await?;
                pin_mut!(dest_file);
                let mut content = file.content.into_stream();
                while let Some(next) = content.next().await {
                    let next = next?;
                    dest_file.write(&next).await?;
                }
            }
            Ok(())
        }
        .boxed()
    }
}

pub trait VFSExt: VFS {}
