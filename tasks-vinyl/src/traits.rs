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
use vfs_async::{Globber, OpenOptions, VFile, VPath, VFS};

pub trait Reply {}

impl Reply for File {}

impl Reply for () {}

pub trait VinylStream: Stream {
    fn pipe<T>(self, task: T) -> Pipe<Self, T>
    where
        Self: Sized + Send,
        Self::Item: Future<Output = Result<File, Error>>,
        T: Task<File, Error = Error>,
        T::Output: Reply,
    {
        Pipe { stream: self, task }
    }
}

impl<S> VinylStream for S where S: Stream {}

#[pin_project]
pub struct Pipe<S, T>
where
    S: Stream,
    S::Item: Future<Output = Result<File, Error>>,
{
    #[pin]
    stream: S,
    task: T,
}

impl<S, T> Stream for Pipe<S, T>
where
    T: Clone + Task<File, Error = Error>,
    S: Stream,
    S::Item: Future<Output = Result<File, Error>>,
{
    type Item = PipeFuture<S::Item, T>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match ready!(this.stream.poll_next(cx)) {
            Some(next) => Poll::Ready(Some(PipeFuture(PipeState::First(next, this.task.clone())))),
            None => Poll::Ready(None),
        }
    }
}

#[pin_project]
enum PipeState<F, T>
where
    T: Task<File>,
{
    First(#[pin] F, T),
    Second(#[pin] T::Future),
    Done,
}

#[pin_project]
pub struct PipeFuture<F, T>(#[pin] PipeState<F, T>)
where
    T: Task<File>;

impl<F, T> Future for PipeFuture<F, T>
where
    T: Task<File, Error = Error>,
    F: Future<Output = Result<File, Error>>,
{
    type Output = Result<T::Output, Error>;
    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();
            #[project]
            let state = match this.0.project() {
                PipeState::First(fut, task) => {
                    //
                    match ready!(fut.poll(cx)) {
                        Ok(ret) => PipeState::<F, T>::Second(task.run(ret)),
                        Err(err) => return Poll::Ready(Err(err)),
                    }
                }
                PipeState::Second(fut) => match ready!(fut.poll(cx)) {
                    Ok(ret) => {
                        self.set(PipeFuture(PipeState::Done));
                        return Poll::Ready(Ok(ret));
                    }
                    Err(err) => match err {
                        Rejection::Err(err) => return Poll::Ready(Err(err)),
                        Rejection::Reject(_, Some(err)) => return Poll::Ready(Err(err)),
                        _ => panic!("reject"),
                    },
                },
                PipeState::Done => panic!("poll after done"),
            };

            self.set(PipeFuture(state));
        }
    }
}