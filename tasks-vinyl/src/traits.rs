use super::util;
use super::{runtime::Mutex, Content, Error, File};
use futures_core::{future::BoxFuture, ready, Stream};
use futures_util::{stream::Buffered, FutureExt, StreamExt, TryStreamExt};
use pin_project::{pin_project, project};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tasks::{Rejection, Task};
use vfs_async::{Globber, OpenOptions, VFile, VPath, VFS};

pub trait Reply {
    type Future: Future<Output = Result<File, Self::Error>>;
    type Error;
    fn into_file(self) -> Self::Future;
}

impl Reply for File {
    type Future = futures_util::future::Ready<Result<File, Error>>;
    type Error = Error;
    fn into_file(self) -> Self::Future {
        futures_util::future::ok(self)
    }
}

impl Reply for (File, ()) {
    type Future = futures_util::future::Ready<Result<File, Error>>;
    type Error = Error;
    fn into_file(self) -> Self::Future {
        futures_util::future::ok(self.0)
    }
}

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

    fn write_to<D: VinylStreamDestination>(
        self,
        mut path: D,
    ) -> BoxFuture<'static, Result<D::Output, Error>>
    where
        Self: Sized + Send + 'static,
        Self::Item: Future<Output = Result<File, Error>> + Send,
        D: Send + Sync + 'static,
    {
        // self.buffer_unordered(10)
        //     .and_then(move |file| path.write(file))
        //     //.try_fold(0, |prev, _| async move { Ok(prev + 1) })
        //     .boxed()
        let this = self;
        async move {
            futures_util::pin_mut!(this);
            while let Some(next) = this.next().await {
                let next = next.await?;
                path.write(next).await?;
            }
            Ok(path.finish().await?)
        }
        .boxed()
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

pub trait VinylStreamDestination {
    type Output;
    fn write(&mut self, _file: File) -> BoxFuture<'static, Result<(), Error>>;
    fn finish(self) -> BoxFuture<'static, Result<Self::Output, Error>>;
}

pub trait IntoVinylStreamDestination {
    type Destination: VinylStreamDestination;
    fn into_destination(self) -> Self::Destination;
}

impl<T> IntoVinylStreamDestination for T
where
    T: VinylStreamDestination,
{
    type Destination = T;
    fn into_destination(self) -> Self::Destination {
        self
    }
}

pub struct Discard;

impl VinylStreamDestination for Discard {
    type Output = ();
    fn write(&mut self, _file: File) -> BoxFuture<'static, Result<(), Error>> {
        futures_util::future::ok(()).boxed()
    }
    fn finish(self) -> BoxFuture<'static, Result<Self::Output, Error>> {
        futures_util::future::ok(()).boxed()
    }
}

#[derive(Default)]
pub struct Vector(Arc<Mutex<Vec<Result<File, Error>>>>);

impl VinylStreamDestination for Vector {
    type Output = Vec<Result<File, Error>>;
    fn write(&mut self, _file: File) -> BoxFuture<'static, Result<(), Error>> {
        let out = self.0.clone();
        async move {
            let mut out = out.lock().await;
            out.push(Ok(_file));
            Ok(())
        }
        .boxed()
    }

    fn finish(self) -> BoxFuture<'static, Result<Self::Output, Error>> {
        async move { Ok(std::mem::replace(&mut *self.0.lock().await, Vec::default())) }.boxed()
    }
}

// #[pin_project]
// pub struct DestinationStream<D, S> {
//     dest: D,
//     #[pin]
//     stream: S,
// }

// impl<D, S> Stream for DestinationStream<D, S>
// where
//     D: VinylStreamDestination,
//     S: Stream,
//     S::Item: Future<Output = Result<File, Error>>,
// {
//     type Item = Result<D::Future, Error>;
//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         let this = self.project();
//         match ready!(this.stream.poll_next(cx)) {
//             Some(Ok(s)) => {
//                 let out = this.dest.write(s);
//                 Poll::Ready(Some(Ok(out)))
//             }
//             Some(Err(e)) => Poll::Ready(Some(Err(e))),
//             None => Poll::Ready(None),
//         }
//     }
// }
