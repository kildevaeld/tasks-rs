use crate::filters::{self, header};
use crate::{reply::Reply, Error, KnownError, Request, Response};
use bytes::{Bytes, BytesMut};
use futures::FutureExt;
use futures::{
    future::{self, Either},
    ready, stream, Stream, StreamExt, TryFuture, TryFutureExt,
};
use headers::{
    AcceptRanges, ContentLength, ContentRange, ContentType, IfModifiedSince, IfRange,
    IfUnmodifiedSince, LastModified, Range,
};
use tokio::io::AsyncRead;

use hyper::{Body, StatusCode};
use modifier::Set;
use std::cmp;
use std::fs::Metadata;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use tasks_core::{Task, TaskExt};
use tokio::fs;

pub fn file<P: AsRef<Path>>(
    path: P,
) -> impl Task<Request, Output = (Request, (File,)), Error = Error> + Clone {
    let path = Arc::new(path.as_ref().to_owned());
    filters::any()
        .map(move || ArcPath(path.clone()))
        .and(file_options())
        .and_then(file_reply)
}

pub(crate) fn file_options(
) -> impl Task<Request, Output = (Request, (FileOptions,)), Error = Error> + Copy {
    header::optional()
        .and(header::optional())
        .and(header::optional())
        .and(header::optional())
        .map(
            |if_modified_since, if_unmodified_since, if_range, range| FileOptions {
                if_modified_since,
                if_unmodified_since,
                if_range,
                range,
            },
        )
}

#[derive(Debug)]
pub(crate) struct FileOptions {
    if_modified_since: Option<IfModifiedSince>,
    if_unmodified_since: Option<IfUnmodifiedSince>,
    if_range: Option<IfRange>,
    range: Option<Range>,
}

enum Cond {
    NoBody(Response),
    WithBody(Option<Range>),
}

impl FileOptions {
    fn check(self, last_modified: Option<LastModified>) -> Cond {
        if let Some(since) = self.if_unmodified_since {
            let precondition = last_modified
                .map(|time| since.precondition_passes(time.into()))
                .unwrap_or(false);

            log::trace!(
                "if-unmodified-since? {:?} vs {:?} = {}",
                since,
                last_modified,
                precondition
            );
            if !precondition {
                let mut res = Response::with(Body::empty());
                res.status = StatusCode::PRECONDITION_FAILED;
                return Cond::NoBody(res);
            }
        }

        if let Some(since) = self.if_modified_since {
            log::trace!(
                "if-modified-since? header = {:?}, file = {:?}",
                since,
                last_modified
            );
            let unmodified = last_modified
                .map(|time| !since.is_modified(time.into()))
                // no last_modified means its always modified
                .unwrap_or(false);
            if unmodified {
                let mut res = Response::with(Body::empty());
                res.status = StatusCode::NOT_MODIFIED;
                return Cond::NoBody(res);
            }
        }

        if let Some(if_range) = self.if_range {
            log::trace!("if-range? {:?} vs {:?}", if_range, last_modified);
            let can_range = !if_range.is_modified(None, last_modified.as_ref());

            if !can_range {
                return Cond::WithBody(None);
            }
        }

        Cond::WithBody(self.range)
    }
}

/// A file response.
#[derive(Debug)]
pub struct File {
    resp: Response,
}

// Silly wrapper since Arc<PathBuf> doesn't implement AsRef<Path> ;_;
#[derive(Clone, Debug)]
pub(crate) struct ArcPath(pub(crate) Arc<PathBuf>);

impl AsRef<Path> for ArcPath {
    fn as_ref(&self) -> &Path {
        (*self.0).as_ref()
    }
}

impl Reply for File {
    fn into_response(self) -> Response {
        self.resp
    }
}

pub(crate) fn file_reply(
    path: ArcPath,
    conditionals: FileOptions,
) -> impl Future<Output = Result<File, Error>> + Send {
    fs::File::open(path.clone()).then(move |res| match res {
        Ok(f) => Either::Left(file_conditional(f, path, conditionals)),
        Err(err) => {
            let rej = match err.kind() {
                io::ErrorKind::NotFound => {
                    log::debug!("file not found: {:?}", path.as_ref().display());
                    KnownError::NotFound
                }
                io::ErrorKind::PermissionDenied => {
                    log::warn!("file permission denied: {:?}", path.as_ref().display());
                    KnownError::FilePermission(None)
                }
                _ => {
                    log::error!(
                        "file open error (path={:?}): {} ",
                        path.as_ref().display(),
                        err
                    );
                    KnownError::FileOpen(None)
                }
            };
            Either::Right(future::err(rej.into()))
        }
    })
}

async fn file_metadata(f: fs::File) -> Result<(fs::File, Metadata), Error> {
    match f.metadata().await {
        Ok(meta) => Ok((f, meta)),
        Err(err) => {
            log::debug!("file metadata error: {}", err);
            Err(KnownError::NotFound.into())
        }
    }
}

fn file_conditional(
    f: fs::File,
    path: ArcPath,
    conditionals: FileOptions,
) -> impl Future<Output = Result<File, Error>> + Send {
    file_metadata(f).map_ok(move |(file, meta)| {
        let mut len = meta.len();
        let modified = meta.modified().ok().map(LastModified::from);

        let resp = match conditionals.check(modified) {
            Cond::NoBody(resp) => resp,
            Cond::WithBody(range) => {
                bytes_range(range, len)
                    .map(|(start, end)| {
                        let sub_len = end - start;
                        let buf_size = optimal_buf_size(&meta);
                        let stream = file_stream(file, buf_size, (start, end));
                        let body = Body::wrap_stream(stream);

                        let mut resp = Response::with(body).set(StatusCode::OK);

                        if sub_len != len {
                            resp = resp.set(StatusCode::PARTIAL_CONTENT).set(
                                ContentRange::bytes(start..end, len).expect("valid ContentRange"),
                            );

                            len = sub_len;
                        }

                        let mime = mime_guess::from_path(path.as_ref()).first_or_octet_stream();

                        resp = resp
                            .set(ContentLength(len))
                            .set(ContentType::from(mime))
                            .set(AcceptRanges::bytes());

                        if let Some(last_modified) = modified {
                            resp = resp.set(last_modified);
                        }

                        resp
                    })
                    .unwrap_or_else(|BadRange| {
                        // bad byte range
                        Response::with(Body::empty())
                            .set(StatusCode::RANGE_NOT_SATISFIABLE)
                            .set(ContentRange::unsatisfied_bytes(len))
                    })
            }
        };

        File { resp }
    })
}

struct BadRange;

fn bytes_range(range: Option<Range>, max_len: u64) -> Result<(u64, u64), BadRange> {
    use std::ops::Bound;

    let range = if let Some(range) = range {
        range
    } else {
        return Ok((0, max_len));
    };

    let ret = range
        .iter()
        .map(|(start, end)| {
            let start = match start {
                Bound::Unbounded => 0,
                Bound::Included(s) => s,
                Bound::Excluded(s) => s + 1,
            };

            let end = match end {
                Bound::Unbounded => max_len,
                Bound::Included(s) => s + 1,
                Bound::Excluded(s) => s,
            };

            if start < end && end <= max_len {
                Ok((start, end))
            } else {
                log::trace!("unsatisfiable byte range: {}-{}/{}", start, end, max_len);
                Err(BadRange)
            }
        })
        .next()
        .unwrap_or(Ok((0, max_len)));
    ret
}

fn file_stream(
    mut file: fs::File,
    buf_size: usize,
    (start, end): (u64, u64),
) -> impl Stream<Item = Result<Bytes, io::Error>> + Send {
    use std::io::SeekFrom;

    let seek = async move {
        if start != 0 {
            file.seek(SeekFrom::Start(start)).await?;
        }
        Ok(file)
    };

    seek.into_stream()
        .map(move |result| {
            let mut buf = BytesMut::new();
            let mut len = end - start;
            let mut f = match result {
                Ok(f) => f,
                Err(f) => return Either::Left(stream::once(future::err(f))),
            };

            Either::Right(stream::poll_fn(move |cx| {
                if len == 0 {
                    return Poll::Ready(None);
                }
                reserve_at_least(&mut buf, buf_size);

                let n = match ready!(Pin::new(&mut f).poll_read_buf(cx, &mut buf)) {
                    Ok(n) => n as u64,
                    Err(err) => {
                        log::debug!("file read error: {}", err);
                        return Poll::Ready(Some(Err(err)));
                    }
                };

                if n == 0 {
                    log::debug!("file read found EOF before expected length");
                    return Poll::Ready(None);
                }

                let mut chunk = buf.split().freeze();
                if n > len {
                    chunk = chunk.split_to(len as usize);
                    len = 0;
                } else {
                    len -= n;
                }

                Poll::Ready(Some(Ok(chunk)))
            }))
        })
        .flatten()
}

fn reserve_at_least(buf: &mut BytesMut, cap: usize) {
    if buf.capacity() - buf.len() < cap {
        buf.reserve(cap);
    }
}

const DEFAULT_READ_BUF_SIZE: usize = 8_192;

fn optimal_buf_size(metadata: &Metadata) -> usize {
    let block_size = get_block_size(metadata);

    // If file length is smaller than block size, don't waste space
    // reserving a bigger-than-needed buffer.
    cmp::min(block_size as u64, metadata.len()) as usize
}

#[cfg(unix)]
fn get_block_size(metadata: &Metadata) -> usize {
    use std::os::unix::fs::MetadataExt;
    //TODO: blksize() returns u64, should handle bad cast...
    //(really, a block size bigger than 4gb?)

    // Use device blocksize unless it's really small.
    cmp::max(metadata.blksize() as usize, DEFAULT_READ_BUF_SIZE)
}

#[cfg(not(unix))]
fn get_block_size(_metadata: &Metadata) -> usize {
    DEFAULT_READ_BUF_SIZE
}
