use super::generic::Extract;
use super::{Rejection, Service};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;

#[derive(Clone, Copy)]
pub struct AndThenReject<T1, T2> {
    t1: T1,
    t2: T2,
}

impl<T1, T2> AndThenReject<T1, T2> {
    pub fn new(t1: T1, t2: T2) -> AndThenReject<T1, T2> {
        AndThenReject { t1, t2 }
    }
}

impl<T1, T2, R> Service<R> for AndThenReject<T1, T2>
where
    T1: Service<R>,
    T1::Output: Extract<R>,
    T2: Send + Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    type Output = T2::Output;
    type Error = T2::Error;
    type Future = AndThenRejectFuture<T1, T2, R>;

    fn call(&self, req: R) -> Self::Future {
        AndThenRejectFuture::new(self.t1.call(req), self.t2.clone())
    }
}

#[pin_project(project = AndThenRejectFutureStateProj)]
enum AndThenRejectFutureState<T1, T2, R>
where
    T1: Service<R>,
    T2: Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    First(#[pin] T1::Future, T2),
    Second(#[pin] T2::Future),
    Done,
}

#[pin_project]
pub struct AndThenRejectFuture<T1, T2, R>
where
    T1: Service<R>,
    T2: Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    #[pin]
    state: AndThenRejectFutureState<T1, T2, R>,
}

impl<T1, T2, R> AndThenRejectFuture<T1, T2, R>
where
    T1: Service<R>,
    T2: Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    pub fn new(t1: T1::Future, t2: T2) -> AndThenRejectFuture<T1, T2, R> {
        AndThenRejectFuture {
            state: AndThenRejectFutureState::First(t1, t2),
        }
    }
}

impl<T1, T2, R> Future for AndThenRejectFuture<T1, T2, R>
where
    T1: Service<R>,
    T1::Output: Extract<R>,
    T2: Clone + Service<<T1 as Service<R>>::Output, Error = <T1 as Service<R>>::Error>,
{
    type Output = Result<T2::Output, Rejection<R, T2::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            let fut2 = match pin.state.project() {
                AndThenRejectFutureStateProj::First(first, second) => {
                    match ready!(first.try_poll(cx)) {
                        Ok(ret) => second.call(ret),
                        Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                        Err(Rejection::Reject(ret, e)) => {
                            return Poll::Ready(Err(Rejection::Reject(ret, e)))
                        }
                    }
                }
                AndThenRejectFutureStateProj::Second(fut) => match ready!(fut.try_poll(cx)) {
                    Ok(some) => {
                        self.set(AndThenRejectFuture {
                            state: AndThenRejectFutureState::Done,
                        });
                        return Poll::Ready(Ok(some));
                    }
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(r, e)) => {
                        let (req, _) = r.unpack();
                        return Poll::Ready(Err(Rejection::Reject(req, e)));
                    }
                },
                AndThenRejectFutureStateProj::Done => panic!("poll after done"),
            };

            self.set(AndThenRejectFuture {
                state: AndThenRejectFutureState::Second(fut2),
            });
        }
    }
}
