use super::vfs_ext::PathOpener;
use super::{Content, Error, File};
use futures_core::Stream;
use futures_util::{StreamExt, TryStreamExt};
use mime_guess;
use std::future::Future;
use vfs_async::{Globber, VMetadata, VPath, VFS};
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
                    let meta = ret.metadata().await?;
                    Ok(File::new(
                        ret.to_string().as_ref().to_owned(),
                        Content::Ref(Box::new(PathOpener(ret.clone()))),
                        mime_guess::from_path(ret.file_name().unwrap_or(String::from("")))
                            .first_or_octet_stream(),
                        meta.len(),
                    ))
                }
                Err(err) => Err(err),
            }
        }
    });

    Ok(stream)
}
