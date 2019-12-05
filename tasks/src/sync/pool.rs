use super::task::SyncTask;
use crate::error::TaskError;
use crate::task::Task;
use futures_channel::oneshot::{channel, Receiver};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use rayon::{ThreadPoolBuilder, ThreadPool};
use std::sync::{Arc};

#[derive(Clone)]
pub struct Pool<T, INPUT> {
    tp: Arc<ThreadPool>,
    task: Arc<T>,
    _i: std::marker::PhantomData<INPUT>
}

impl<T, INPUT> Pool<T, INPUT>
where
    T: SyncTask<INPUT> + Sync + Send + 'static,
    <T as SyncTask<INPUT>>::Output: Send + 'static,
    <T as SyncTask<INPUT>>::Error: From<TaskError> + Send + 'static,
{
    pub fn new(size: usize, task: T) -> Result<Pool<T,INPUT>, Box<dyn std::error::Error>> {
        let tp = ThreadPoolBuilder::new().num_threads(size).build()?;
        Ok(Self::with_pool(tp, task))
    }

    pub fn with_pool(pool: ThreadPool, task: T) -> Pool<T,INPUT> {
        Pool {
            tp: Arc::new(pool),
            task: Arc::new(task),
            _i: std::marker::PhantomData
        }
    }
}

impl<T, INPUT: Send> Task<INPUT> for Pool<T, INPUT>
where
    T: SyncTask<INPUT> + Sync + Send + 'static,
    <T as SyncTask<INPUT>>::Output: Send + 'static,
    <T as SyncTask<INPUT>>::Error: From<TaskError> + Send + 'static,
{
    type Output = T::Output;
    type Error = T::Error;
    type Future = ChannelReceiverFuture<T::Output, T::Error>;

    fn exec(&self, input: INPUT) -> Self::Future {
        let (sx, rx) = channel();
        let work = self.task.clone();
        self.tp.install(move || {
            let result = work.exec(input);
            if let Err(_) = sx.send(result) {}
        });

        ChannelReceiverFuture::new(rx)
    }

    fn can_exec(&self, input: &INPUT) -> bool {
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
