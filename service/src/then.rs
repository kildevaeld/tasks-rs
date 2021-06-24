use super::{Rejection, Service};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct Then<T1, T2> {
    t1: T1,
    t2: T2,
}

impl<T1, T2> Then<T1, T2> {
    pub fn new(t1: T1, t2: T2) -> Then<T1, T2> {
        Then { t1, t2 }
    }
}

impl<T1, T2, R> Service<R> for Then<T1, T2>
where
    T1: Service<R>,
    T2: Send + Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    type Output = T2::Output;
    type Error = T2::Error;
    type Future = ThenFuture<T1, T2, R>;

    fn call(&self, req: R) -> Self::Future {
        ThenFuture::new(self.t1.call(req), self.t2.clone())
    }
}

#[pin_project(project = ThenFutureStateProj)]
enum ThenFutureState<T1, T2, R>
where
    T1: Service<R>,
    T2: Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    First(#[pin] T1::Future, T2),
    Second(#[pin] T2::Future),
    Done,
}

#[pin_project]
pub struct ThenFuture<T1, T2, R>
where
    T1: Service<R>,
    T2: Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    #[pin]
    state: ThenFutureState<T1, T2, R>,
}

impl<T1, T2, R> ThenFuture<T1, T2, R>
where
    T1: Service<R>,
    T2: Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    pub fn new(t1: T1::Future, t2: T2) -> ThenFuture<T1, T2, R> {
        ThenFuture {
            state: ThenFutureState::First(t1, t2),
        }
    }
}

impl<T1, T2, R> Future for ThenFuture<T1, T2, R>
where
    T1: Service<R>,
    T2: Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    type Output = Result<T2::Output, Rejection<R, T2::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            let fut2 = match pin.state.project() {
                ThenFutureStateProj::First(first, second) => match ready!(first.try_poll(cx)) {
                    Ok(ret) => second.call(ret),
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(ret, e)) => {
                        return Poll::Ready(Err(Rejection::Reject(ret, e)))
                    }
                },
                ThenFutureStateProj::Second(fut) => match ready!(fut.try_poll(cx)) {
                    Ok(some) => {
                        self.set(ThenFuture {
                            state: ThenFutureState::Done,
                        });
                        return Poll::Ready(Ok(some));
                    }
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(_, Some(err))) => {
                        return Poll::Ready(Err(Rejection::Err(err)));
                    }
                    Err(Rejection::Reject(_, None)) => {
                        panic!("rejected");
                    }
                },
                ThenFutureStateProj::Done => panic!("poll after done"),
            };

            self.set(ThenFuture {
                state: ThenFutureState::Second(fut2),
            });
        }
    }
}
