use bytes::Bytes;
use futures_core::Stream;
use futures_io::AsyncRead;
use pin_project::pin_project;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

#[pin_project]
pub struct ByteStream<R>(#[pin] R);

impl<R> ByteStream<R> {
    pub fn new(read: R) -> ByteStream<R> {
        ByteStream(read)
    }
}

impl<R: AsyncRead> Stream for ByteStream<R> {
    // The same as our future above:
    type Item = Result<Bytes, std::io::Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let mut buf = [0; 16384];
        match this.0.poll_read(cx, &mut buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(ret)) => {
                if ret == 0 {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(Bytes::copy_from_slice(&buf[0..ret]))))
                }
            }
            Poll::Ready(Err(err)) => Poll::Ready(Some(Err(err))),
        }
    }
}

pub mod path {
    pub fn join(base: &str, path: &str) -> String {
        pathutils::join(base, path)
    }
}
