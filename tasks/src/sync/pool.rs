use super::task::SyncTask;
use crate::error::TaskError;
use crate::task::Task;
use futures_channel::oneshot::{channel, Receiver};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use threadpool::ThreadPool;

#[derive(Clone)]
pub struct Pool<T> {
    tp: ThreadPool,
    task: std::sync::Arc<T>,
}

impl<T> Pool<T>
where
    T: SyncTask + Sync + Send + 'static,
    <T as SyncTask>::Input: Send,
    <T as SyncTask>::Output: Send + 'static,
    <T as SyncTask>::Error: From<TaskError> + Send + 'static,
{
    pub fn new(size: usize, task: T) -> Pool<T> {
        let tp = ThreadPool::new(size);
        Self::with_pool(tp, task)
    }

    pub fn with_pool(pool: ThreadPool, task: T) -> Pool<T> {
        Pool {
            tp: pool,
            task: std::sync::Arc::new(task),
        }
    }
}

impl<T> Task for Pool<T>
where
    T: SyncTask + Sync + Send + 'static,
    <T as SyncTask>::Input: Send,
    <T as SyncTask>::Output: Send + 'static,
    <T as SyncTask>::Error: From<TaskError> + Send + 'static,
{
    type Input = T::Input;
    type Output = T::Output;
    type Error = T::Error;
    type Future = ChannelReceiverFuture<T::Output, T::Error>;

    fn exec(&self, input: Self::Input) -> Self::Future {
        let (sx, rx) = channel();
        let work = self.task.clone();
        self.tp.execute(move || {
            let result = work.exec(input);
            if let Err(_) = sx.send(result) {}
        });

        ChannelReceiverFuture::new(rx)
    }

    fn can_exec(&self, input: &Self::Input) -> bool {
        self.task.can_exec(input)
    }
}


#[pin_project]
pub struct ChannelReceiverFuture<O, E: From<TaskError>> {
    #[pin]
    rx: Receiver<Result<O, E>>,
}

impl<O, E: From<TaskError>> ChannelReceiverFuture<O, E> {
    pub fn new(rx: Receiver<Result<O, E>>) -> ChannelReceiverFuture<O, E> {
        ChannelReceiverFuture { rx }
    }
}

impl<O, E: From<TaskError>> Future for ChannelReceiverFuture<O, E> {
    type Output = Result<O, E>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.rx.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(s)) => Poll::Ready(s),
            Poll::Ready(Err(_e)) => Poll::Ready(Err(TaskError::ReceiverClosed.into())),
        }
    }
}
