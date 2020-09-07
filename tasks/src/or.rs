use super::{Either, Rejection, Task};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;
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
    T2: Send + Clone + Task<R, Error = <T1 as Task<R>>::Error>,
{
    type Output = Either<T1::Output, T2::Output>;
    type Error = T1::Error;
    type Future = OrFuture<T1, T2, R>;

    fn run(&self, req: R) -> Self::Future {
        OrFuture {
            state: OrFutureState::First(self.t1.run(req), self.t2.clone()),
        }
    }
}

#[pin_project(project = OrFutureStateProj)]
enum OrFutureState<T1, T2, R>
where
    T1: Task<R>,
    T2: Task<R, Error = <T1 as Task<R>>::Error>,
{
    First(#[pin] T1::Future, T2),
    Second(#[pin] T2::Future),
    Done,
}

#[pin_project]
pub struct OrFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Task<R, Error = <T1 as Task<R>>::Error>,
{
    #[pin]
    state: OrFutureState<T1, T2, R>,
}

impl<T1, T2, R> Future for OrFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Task<R, Error = <T1 as Task<R>>::Error>,
{
    type Output = Result<Either<T1::Output, T2::Output>, Rejection<R, T1::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            let fut2 = match pin.state.project() {
                OrFutureStateProj::First(first, second) => match ready!(first.try_poll(cx)) {
                    Ok(ret) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Ok(Either::A(ret)));
                    }
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(req, _)) => second.run(req),
                },
                OrFutureStateProj::Second(fut) => match ready!(fut.try_poll(cx)) {
                    Ok(some) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Ok(Either::B(some)));
                    }
                    Err(err) => return Poll::Ready(Err(err)),
                },
                OrFutureStateProj::Done => panic!("poll after done"),
            };

            self.set(OrFuture {
                state: OrFutureState::Second(fut2),
            });
        }
    }
}
