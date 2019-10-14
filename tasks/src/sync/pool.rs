use crate::error::TaskError;
use crate::task::{ConditionalTask, Task};
use futures_channel::oneshot::{channel, Receiver, };
use pin_utils::unsafe_pinned;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use threadpool::ThreadPool;
use super::task::SyncTask;



// pub struct Pool<T> {
//     tp: ThreadPool,
//     func: Arc<F>,
//     _i: PhantomData<I>,
//     _o: PhantomData<O>,
//     _e: PhantomData<E>,
// }

// impl<F, I, O, E> Pool<F, I, O, E> {
//     pub fn new(size: usize, func: F) -> Pool<F, I, O, E> {
//         let tp = ThreadPool::new(size);
//         Self::with_pool(tp, func)
//     }

//     pub fn with_pool(pool: ThreadPool, func: F) -> Pool<F, I, O, E> {
//         Pool {
//             tp: pool,
//             func: Arc::new(func),
//             _i: PhantomData,
//             _o: PhantomData,
//             _e: PhantomData,
//         }
//     }
// }

// impl<F, I, O, E: std::error::Error> Task for Pool<F, I, O, E>
// where
//     F: (Fn(I) -> Result<O, E>) + Send + Sync + 'static,
//     O: Send + 'static,
//     I: Send + 'static,
//     E: Send + 'static + From<TaskError>,
// {
//     type Input = I;
//     type Output = O;
//     type Error = E;
//     type Future = ChannelReceiverFuture<O, E>;

//     fn exec(&self, input: I) -> Self::Future {
//         let (sx, rx) = channel();
//         let work = self.func.clone();
//         self.tp.execute(move || {
//             let result = work(input);
//             sx.send(result);
//         });

//         ChannelReceiverFuture::new(rx)
//     }
// }

#[derive(Clone)]
pub struct Pool<T> {
    tp: ThreadPool,
    task: T
}

impl<T> Pool<T> {
    pub fn new(size: usize, task: T) -> Pool<T> {
        let tp = ThreadPool::new(size);
        Self::with_pool(tp, task)
    }

    pub fn with_pool(pool: ThreadPool, task: T) -> Pool<T> {
        Pool {
            tp: pool,
            task
        }
    }
}

impl<T> Task for Pool<T>
where
    T: SyncTask + Clone + Send + 'static,
    <T as SyncTask>::Input: Send,
    <T as SyncTask>::Output: Send + 'static,
    <T as SyncTask>::Error: From<TaskError> + Send
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
            sx.send(result);
        });

        ChannelReceiverFuture::new(rx)
    }
}

pub struct ChannelReceiverFuture<O, E: From<TaskError>> {
    rx: Receiver<Result<O, E>>,
}

impl<O, E: From<TaskError>> ChannelReceiverFuture<O, E> 
{
    unsafe_pinned!(rx: Receiver<Result<O, E>>);

    pub fn new(rx: Receiver<Result<O, E>>) -> ChannelReceiverFuture<O, E> {
        ChannelReceiverFuture { rx }
    }
}

impl<O, E: From<TaskError>> Future for ChannelReceiverFuture<O, E> 
{
    type Output = Result<O, E>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().rx().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(s)) => Poll::Ready(s),
            Poll::Ready(Err(_e)) => Poll::Ready(Err(TaskError::ReceiverClosed.into())),
        }
    }
}
