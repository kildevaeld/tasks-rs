use super::{Rejection, Task};
use futures_core::{ready, TryFuture};
use pin_project::{pin_project, project};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct Pipe<T1, T2> {
    t1: T1,
    t2: T2,
}

impl<T1, T2> Pipe<T1, T2> {
    pub fn new(t1: T1, t2: T2) -> Pipe<T1, T2> {
        Pipe { t1, t2 }
    }
}

impl<T1, T2, R> Task<R> for Pipe<T1, T2>
where
    T1: Task<R>,
    T2: Send + Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    type Output = T2::Output;
    type Error = T2::Error;
    type Future = PipeFuture<T1, T2, R>;

    fn run(&self, req: R) -> Self::Future {
        PipeFuture::new(self.t1.run(req), self.t2.clone())
    }
}

#[pin_project]
enum PipeFutureState<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    First(#[pin] T1::Future, T2),
    Second(#[pin] T2::Future),
    Done,
}

#[pin_project]
pub struct PipeFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    #[pin]
    state: PipeFutureState<T1, T2, R>,
}

impl<T1, T2, R> PipeFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    pub fn new(t1: T1::Future, t2: T2) -> PipeFuture<T1, T2, R> {
        PipeFuture {
            state: PipeFutureState::First(t1, t2),
        }
    }
}

impl<T1, T2, R> Future for PipeFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    type Output = Result<T2::Output, Rejection<R, T2::Error>>;
    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            #[project]
            let fut2 = match pin.state.project() {
                PipeFutureState::First(first, second) => match ready!(first.try_poll(cx)) {
                    Ok(ret) => second.run(ret),
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(ret)) => return Poll::Ready(Err(Rejection::Reject(ret))),
                },
                PipeFutureState::Second(fut) => match ready!(fut.try_poll(cx)) {
                    Ok(some) => {
                        self.set(PipeFuture {
                            state: PipeFutureState::Done,
                        });
                        return Poll::Ready(Ok(some));
                    }
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(_)) => {
                        panic!("should propragate cause");
                        //return Poll::Ready(Err(Rejection::Err(PipeError::Reject)))
                    }
                },
                PipeFutureState::Done => panic!("poll after done"),
            };

            self.set(PipeFuture {
                state: PipeFutureState::Second(fut2),
            });
        }
    }
}

// #[derive(Debug, PartialEq)]
// pub enum PipeError<E> {
//     Err(E),
//     Reject,
// }
