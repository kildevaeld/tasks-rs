use crate::{AssetRequest, AssetResponse, Dir, Error, Node};
use bytes::Bytes;
use futures_util::{
    future::BoxFuture, stream::BoxStream, FutureExt, StreamExt, TryFutureExt, TryStreamExt,
};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tasks::{task, Rejection, Task};
use tasks_vinyl::{
    mime_guess::from_path, runtime as rt, util::ByteStream, Content, Error as VinylError, File,
    Opener,
};
// use std::fs:
pub fn dir(
    path: impl AsRef<Path>,
) -> impl Task<AssetRequest, Output = AssetResponse, Error = Error> + Clone + Sync + Send {
    let path = path.as_ref().to_path_buf();
    task!(move |req: AssetRequest| {
        let path = path.join(req.path());
        async move {
            let metadata = match rt::metadata(&path).await {
                Ok(m) => m,
                Err(_) => return Err(Rejection::Reject(req, None)),
            };

            let node = if metadata.is_dir() {
                let readdir = match rt::read_dir(&path).await {
                    Ok(readir) => readir,
                    Err(err) => return Err(Rejection::Reject(req, Some(Error::Io(err)))),
                };

                let out = readdir
                    .and_then(|next| async move {
                        let path = next.path();
                        let meta = rt::metadata(&path).await?;

                        let node = if meta.is_dir() {
                            Node::Dir(Dir {
                                name: next.path().to_str().unwrap().to_string(),
                                children: Vec::default(),
                                path: next.path().to_str().unwrap().to_string(),
                            })
                        } else {
                            let content = Content::Ref(Box::new(PathOpener(path.clone(), false)));
                            let mime = from_path(&path).first_or_octet_stream();
                            Node::File(File::new(
                                path.as_path().to_str().unwrap(),
                                content,
                                mime,
                                meta.size(),
                            ))
                        };

                        Ok(node)
                    })
                    .try_collect::<Vec<_>>()
                    .await
                    .unwrap();
                Node::Dir(Dir {
                    name: path
                        .parent()
                        .map(|m| m.to_str().unwrap().to_owned())
                        .unwrap_or_else(|| String::from("/")),
                    path: path.to_str().unwrap().to_string(),
                    children: out,
                })
            } else {
                let content = Content::Ref(Box::new(PathOpener(path.clone(), false)));
                let mime = from_path(&path).first_or_octet_stream();
                Node::File(File::new(
                    path.as_path().to_str().unwrap(),
                    content,
                    mime,
                    metadata.size(),
                ))
            };

            Ok(AssetResponse { request: req, node })
        }
    })
}

pub struct PathOpener(pub PathBuf, bool);

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
