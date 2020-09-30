use super::error::Error;
use super::util::ByteStream;
use super::{Content, File, IntoVinylStreamDestination, Opener, VinylStreamDestination};
use bytes::Bytes;
use futures_util::{
    future::BoxFuture, pin_mut, stream::BoxStream, AsyncReadExt, AsyncWriteExt, FutureExt,
    StreamExt, TryFutureExt, TryStreamExt,
};
use mime_guess;
use tasks::{Rejection, Task};
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
                    Ok(Some(File::new(
                        path.to_string().as_ref().to_owned(),
                        Content::Stream(Box::pin(stream)),
                        mime_guess::from_path(path.file_name().unwrap_or(String::from("")))
                            .first_or_octet_stream(),
                        meta.len(),
                    )))
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

                    Ok(Some(File::new(
                        path.to_string().as_ref().to_owned(),
                        Content::Stream(Box::pin(stream)),
                        mime_guess::from_path(path.file_name().unwrap_or(String::from("")))
                            .first_or_octet_stream(),
                        meta.len(),
                    )))
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
    type Output = ();
    fn write(&mut self, file: File) -> BoxFuture<'static, Result<(), Error>> {
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
                let mut content = file.content.into_stream().await?;
                while let Some(next) = content.next().await {
                    let next = next?;
                    dest_file.write(&next).await?;
                }
            }
            Ok(())
        }
        .boxed()
    }

    fn finish(self) -> BoxFuture<'static, Result<(), Error>> {
        futures_util::future::ok(()).boxed()
    }
}

pub trait VFSExt: VFS {}

#[derive(Clone)]
pub struct PathTask<V> {
    path: V,
    overwrite: bool,
}

impl<V> PathTask<V>
where
    V: VPath,
{
    pub fn new(path: V) -> PathTask<V> {
        PathTask {
            path,
            overwrite: false,
        }
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }
}

impl<V> Task<File> for PathTask<V>
where
    V: VPath + 'static + Send,
    V::File: 'static + Send,
{
    type Output = File;
    type Error = Error;
    type Future = BoxFuture<'static, Result<File, Rejection<File, Error>>>;

    fn run(&self, mut file: File) -> Self::Future {
        let path = self.path.clone();
        let overwrite = self.overwrite;
        async move {
            let path = path.resolve(&file.path);
            if let Some(parent) = path.parent() {
                if !parent.exists().await {
                    parent
                        .mkdir()
                        .await
                        .map_err(|err| Rejection::Err(err.into()))?;
                }
            }
            if path.exists().await && !overwrite {
                return Err(Rejection::Reject(file, Some(Error::FileAlreadyExists)));
            } else {
                let dest_file = path
                    .open(OpenOptions::new().create(true).write(true))
                    .await
                    .map_err(|err| Rejection::Err(err.into()))?;
                pin_mut!(dest_file);
                let mut content = file.content.into_stream().await.map_err(Rejection::Err)?;
                while let Some(next) = content.next().await {
                    let next = next.map_err(|err| Rejection::Err(err.into()))?;
                    dest_file
                        .write(&next)
                        .await
                        .map_err(|err| Rejection::Err(err.into()))?;
                }
            }

            let body = path
                .open(OpenOptions::new().read(true))
                .await
                .map_err(|err| Rejection::Err(err.into()))?;

            file.content = Content::from_stream(ByteStream::new(body).map_err(Error::Io));

            Ok(file)
        }
        .boxed()
    }
}

pub struct PathOpener<V>(pub V);

impl<V> Opener for PathOpener<V>
where
    V: VPath,
    V::File: 'static,
{
    fn open(&self) -> BoxFuture<'static, Result<BoxStream<'static, Result<Bytes, Error>>, Error>> {
        self.0
            .open(OpenOptions::new().read(true))
            .map_ok(|file| ByteStream::new(file).map_err(Error::Io).boxed())
            .map_err(Error::Io)
            .boxed()
    }
}
