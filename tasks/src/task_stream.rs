use super::task::Task;
use futures_core::Stream;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};


pub trait TaskStreamExt: Stream + Sized {
    fn with_task<T: Task<Self::Item>>(self, task: T) -> TaskStream<Self, T>;
}

impl<S> TaskStreamExt for S where S: Stream {
    fn with_task<T: Task<Self::Item>>(self, task: T) -> TaskStream<Self, T> {
        TaskStream::new(self, task)
    }
}


#[pin_project]
pub struct TaskStream<S, T> {
    #[pin]
    stream: S,
    task: T,
}

impl<S, T> TaskStream<S, T> {
    pub fn new(stream: S, task: T) -> TaskStream<S, T> {
        TaskStream {
            stream,
            task,
        }
    }
}

impl<S, T> Stream for TaskStream<S, T>
where
    S: Stream,
    T: Task<<S as Stream>::Item>,
{
    type Item = T::Future;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.stream.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(s)) => Poll::Ready(Some(this.task.exec(s))),
            Poll::Ready(None) => Poll::Ready(None),
        }
    }
}
