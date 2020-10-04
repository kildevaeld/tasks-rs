use crate::{Asset, AssetRequest, AssetResponse, Error, Node};
use bytes::Bytes;
use futures_util::{
    future::BoxFuture, stream::BoxStream, FutureExt, StreamExt, TryFutureExt, TryStreamExt,
};
use std::os::unix::fs::MetadataExt;
use tasks::{reject, task, Rejection, Task};
use tasks_vinyl::{
    mime_guess::from_path, runtime as rt, util, util::ByteStream, Content, Error as VinylError,
    File, Opener, Path,
};
// use std::fs:
pub fn dir(
    path: impl Into<Path>,
) -> impl Task<AssetRequest, Output = AssetResponse, Error = Error> + Clone + Sync + Send {
    let root = path.into();
    task!(move |req: AssetRequest| {
        let root = root.clone();
        let path = root.join(req.path());
        let valid = path.contains(&*root);
        async move {
            if !valid {
                reject!(req, Error::Custom("invalid path".to_owned()));
            }
            let metadata = match rt::metadata(&std::path::Path::new(&*path)).await {
                Ok(m) => m,
                Err(_) => return Err(Rejection::Reject(req, None)),
            };

            let node = if metadata.is_dir() {
                let readdir = match rt::read_dir(&std::path::Path::new(&*path)).await {
                    Ok(readir) => readir,
                    Err(err) => return Err(Rejection::Reject(req, Some(Error::Io(err)))),
                };

                let out = readdir
                    .and_then(move |next| {
                        let root = root.clone();
                        async move {
                            let full_path = next.path();
                            let path = full_path.to_str().unwrap().replace(&*root, "");
                            let meta = rt::metadata(&full_path).await?;
                            let node = if meta.is_dir() {
                                Node::Dir(Path::new(path))
                            } else {
                                let mime = from_path(&path).first_or_octet_stream();
                                Node::File(Path::new(path), mime, meta.size())
                            };

                            Ok(node)
                        }
                    })
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap();

                Asset::dir(path, out)
            } else {
                let content = Content::Ref(Box::new(PathOpener(
                    std::path::Path::new(&*path).to_path_buf(),
                    false,
                )));
                let mime = from_path(&*path).first_or_octet_stream();
                Asset::File(File::new(req.path(), content, mime, metadata.size()))
            };

            Ok(AssetResponse { request: req, node })
        }
    })
}

pub struct PathOpener(pub std::path::PathBuf, bool);

impl Opener for PathOpener {
    fn open(
        &self,
    ) -> BoxFuture<'static, Result<BoxStream<'static, Result<Bytes, VinylError>>, VinylError>> {
        let path = self.0.clone();
        let write = self.1;
        async move {
            let file = if write {
                rt::open_file(&path).await?
            } else {
                rt::create_file(&path).await?
            };
            Ok(ByteStream::new(file).map_err(VinylError::Io).boxed())
        }
        .boxed()
    }
}
