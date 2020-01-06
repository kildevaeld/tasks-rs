use super::task::Task;
use super::PipeFuture;
use futures_core::Stream;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub trait Producer {
    type Item;
    type Error;
    type Future: Future<Output = Result<Self::Item, Self::Error>>;
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Future>>;
}

impl<S, V, E> Producer for S where S: Stream, <S as Stream>::Item: Future<Output = Result<V, E>> {
    type Item = V;
    type Error = E;
    type Future = S::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Future>> {
        <S as Stream>::poll_next(self, cx)
    }
}


#[pin_project]
pub struct TaskProducer<P, T> {
    #[pin]
    p: P,
    t: Arc<T>,
}

impl<P, T> TaskProducer<P, T> {
    pub fn new(p: P, t: T) -> TaskProducer<P, T> {
        TaskProducer { p, t: Arc::new(t) }
    }
}

impl<P, T> Producer for TaskProducer<P, T>
where
    P: Producer,
    T: Task<<P as Producer>::Item, Error = <P as Producer>::Error>,
{
    type Item = T::Output;
    type Error = T::Error;
    type Future = PipeFuture<P::Future, T, P::Item, P::Error>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Future>> {
        let this = self.project();
        match this.p.poll_next(cx) {
            Poll::Ready(Some(s)) => Poll::Ready(Some(PipeFuture::new(s, &this.t))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

