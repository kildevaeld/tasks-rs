use super::file::{file_options, file_reply, ArcPath, File};
use crate::{Error, KnownError, Request};
use futures::{future, TryFutureExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tasks::{One, Task, TaskExt};
use urlencoding::decode;

pub fn dir(
    path: impl Into<PathBuf>,
) -> impl Task<Request, Output = (Request, One<File>), Error = Error> + Clone {
    let base = Arc::new(path.into());
    crate::filters::get()
        .and(path_from_tail(base))
        .and(file_options())
        .and_then(file_reply)
}

fn path_from_tail(
    base: Arc<PathBuf>,
) -> impl Task<Request, Output = (Request, One<ArcPath>), Error = Error> + Clone {
    crate::filters::url().and_then(move |tail: url::Url| {
        future::ready(sanitize_path(base.as_ref(), tail.path())).and_then(|mut buf| async {
            let is_dir = tokio::fs::metadata(buf.clone())
                .await
                .map(|m| m.is_dir())
                .unwrap_or(false);

            if is_dir {
                log::debug!("dir: appending index.html to directory path");
                buf.push("index.html");
            }
            log::trace!("dir: {:?}", buf);
            Ok(ArcPath(Arc::new(buf)))
        })
    })
}

fn sanitize_path(base: impl AsRef<Path>, tail: &str) -> Result<PathBuf, Error> {
    let mut buf = PathBuf::from(base.as_ref());
    let p = match decode(tail) {
        Ok(p) => p,
        Err(err) => {
            log::debug!("dir: failed to decode route={:?}: {:?}", tail, err);
            // FromUrlEncodingError doesn't implement StdError
            return Err(KnownError::NotFound.into());
        }
    };
    log::trace!("dir? base={:?}, route={:?}", base.as_ref(), p);
    for seg in p.split('/') {
        if seg.starts_with("..") {
            log::warn!("dir: rejecting segment starting with '..'");
            return Err(KnownError::NotFound.into());
        } else if seg.contains('\\') {
            log::warn!("dir: rejecting segment containing with backslash (\\)");
            return Err(KnownError::NotFound.into());
        } else {
            buf.push(seg);
        }
    }
    Ok(buf)
}
