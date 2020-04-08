use super::{Rejection, Task};
use futures_core::{ready, TryFuture};
use pin_project::{pin_project, project};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct Or<T1, T2> {
    t1: T1,
    t2: T2,
}

impl<T1, T2> Or<T1, T2> {
    pub fn new(t1: T1, t2: T2) -> Or<T1, T2> {
        Or { t1, t2 }
    }
}

impl<T1, T2, R> Task<R> for Or<T1, T2>
where
    T1: Task<R>,
    T2: Send + Clone + Task<R, Output = <T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    type Output = T1::Output;
    type Error = T1::Error;
    type Future = OrFuture<T1, T2, R>;

    fn run(&self, req: R) -> Self::Future {
        OrFuture {
            state: OrFutureState::First(self.t1.run(req), self.t2.clone()),
        }
    }
}

#[pin_project]
enum OrFutureState<T1, T2, R>
where
    T1: Task<R>,
    T2: Task<R, Output = <T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    First(#[pin] T1::Future, T2),
    Second(#[pin] T2::Future),
    Done,
}

#[pin_project]
pub struct OrFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Task<R, Output = <T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    #[pin]
    state: OrFutureState<T1, T2, R>,
}

impl<T1, T2, R> Future for OrFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Task<R, Output = <T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    type Output = Result<T1::Output, Rejection<R, T1::Error>>;

    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            #[project]
            let fut2 = match pin.state.project() {
                OrFutureState::First(first, second) => match ready!(first.try_poll(cx)) {
                    Ok(ret) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Ok(ret));
                    }
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(req)) => second.run(req),
                },
                OrFutureState::Second(fut) => match ready!(fut.try_poll(cx)) {
                    Ok(some) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Ok(some));
                    }
                    Err(err) => return Poll::Ready(Err(err)),
                },
                OrFutureState::Done => panic!("poll after done"),
            };

            self.set(OrFuture {
                state: OrFutureState::Second(fut2),
            });
        }
    }
}
