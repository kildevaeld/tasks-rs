use super::task::Task;
use super::{Producer, TaskProducer};
use futures_core::Stream;
use futures_util::stream::{Buffered, StreamExt};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait ProducerExt: Producer + Sized {
    fn into_stream(self) -> ProducerStream<Self>;
    fn with_task<T: Task<Self::Item, Error = Self::Error>>(self, task: T) -> TaskProducer<Self, T>;
    fn consume(self, n: usize) -> Consume<Self>;
    fn collect(self, n: usize) -> Collect<Self>;
}

impl<P> ProducerExt for P
where
    P: Producer,
{
    fn into_stream(self) -> ProducerStream<Self> {
        ProducerStream(self)
    }

    fn with_task<T: Task<Self::Item, Error = Self::Error>>(self, task: T) -> TaskProducer<Self, T> {
        TaskProducer::new(self, task)
    }

    fn consume(self, n: usize) -> Consume<Self> {
        Consume::new(self, n)
    }

    fn collect(self, n: usize) -> Collect<Self> {
        Collect::new(self, n)
    }
}

#[pin_project]
pub struct ProducerStream<P: Producer>(#[pin] P);

impl<P: Producer> Stream for ProducerStream<P> {
    type Item = P::Future;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.0.poll_next(cx)
    }
}

pub struct Consume<P>
where
    P: Producer,
{
    p: Buffered<ProducerStream<P>>,
    sofe: bool
}

impl<P> Consume<P>
where
    P: Producer,
{
    pub fn new(producer: P, n: usize) -> Consume<P> {
        Consume {
            p: producer.into_stream().buffered(n),
            sofe: true,
        }
    }
}

impl<P> Future for Consume<P>
where
    P: Producer,
{
    type Output = Result<(), P::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };
        loop {
            match unsafe { Pin::new_unchecked(&mut this.p) }.poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(_))) => {}
                Poll::Ready(Some(Err(err))) => {
                    if this.sofe {
                        return Poll::Ready(Err(err));
                    }
                },
                Poll::Ready(None) => return Poll::Ready(Ok(())),
            }
        }
    }
}

pub struct Collect<P>
where
    P: Producer,
{
    p: Buffered<ProducerStream<P>>,
    //current: Option<P::Future>,
    t: Vec<P::Item>,
}

impl<P> Collect<P>
where
    P: Producer,
{
    pub fn new(producer: P, n: usize) -> Collect<P> {
        Collect {
            p: producer.into_stream().buffered(n),
            t: Vec::default(),
        }
    }
}

impl<P> Future for Collect<P>
where
    P: Producer,
{
    type Output = Result<Vec<P::Item>, P::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        loop {
            match unsafe { Pin::new_unchecked(&mut this.p) }.poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(s))) => {
                    this.t.push(s);
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Err(e));
                }
                Poll::Ready(None) => {
                    let out = std::mem::replace(&mut this.t, Vec::default());
                    return Poll::Ready(Ok(out));
                }
            };
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::producer::*;
    use crate::task_fn;
    use async_stream::stream;
    use futures_executor::block_on;
    use futures_util::StreamExt;
    #[test]
    fn test_producer() {
        let stream = stream! {
            for i in 0..5 {
                yield futures_util::future::ok::<i32, ()>(i);
            }
        };

        // let stream = stream.with_task(task_fn!(|i| {
        //     async move { Ok(i + 1) }
        // })).consume(task_fn!(|item| {
        //     async move {
        //         Ok(())
        //     }
        // }));

        let stream = stream
            .with_task(task_fn!(|i| { async move { Ok(i + 1) } }))
            .collect(3);

        let out = block_on(stream);

        assert_eq!(out, Ok(vec![1, 2, 3, 4, 5]));
    }
}
