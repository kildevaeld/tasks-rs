use super::util;
use super::{Content, Error, File};
use futures_core::{future::BoxFuture, ready, Stream};
use futures_util::{stream::Buffered, StreamExt, TryStreamExt};
use pin_project::{pin_project, project};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tasks::{Rejection, Task};
use tokio::sync::Mutex;
use vfs_async::{Globber, OpenOptions, VFile, VPath, VFS};

pub async fn src<V>(
    vfs: V,
    glob: &str,
) -> Result<impl Stream<Item = impl Future<Output = Result<File, Error>> + Send> + Send, Error>
where
    V: VFS,
    V::Path: 'static + std::marker::Unpin,
    <V::Path as VPath>::ReadDir: Send,
    <V::Path as VPath>::Metadata: Send,
{
    let stream = vfs_async::glob(vfs.path("."), Globber::new(glob)).await?;
    let stream = stream.map_err(|err| err.into()).then(|ret| async move {
        async move {
            match ret {
                Ok(ret) => {
                    let content = ret.open(OpenOptions::new().read(true)).await?;
                    let stream = util::ByteStream::new(content).map_err(|e| e.into());
                    Ok(File {
                        path: ret.to_string().as_ref().to_owned(),
                        content: Content::Stream(Box::pin(stream)),
                    })
                }
                Err(err) => Err(err),
            }
        }
    });

    Ok(stream)
}
