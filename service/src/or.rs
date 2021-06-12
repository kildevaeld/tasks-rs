use super::{Either, Rejection, Service};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;

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

impl<T1, T2, R> Service<R> for Or<T1, T2>
where
    T1: Service<R>,
    T2: Send + Clone + Service<R, Error = <T1 as Service<R>>::Error>,
{
    type Output = Either<T1::Output, T2::Output>;
    type Error = Either<T1::Error, T2::Error>;
    type Future = OrFuture<T1, T2, R>;

    fn call(&mut self, req: R) -> Self::Future {
        OrFuture {
            state: OrFutureState::First(self.t1.call(req), self.t2.clone()),
        }
    }
}

#[pin_project(project = OrFutureStateProj)]
enum OrFutureState<T1, T2, R>
where
    T1: Service<R>,
    T2: Service<R, Error = <T1 as Service<R>>::Error>,
{
    First(#[pin] T1::Future, T2),
    Second(#[pin] T2::Future),
    Done,
}

#[pin_project]
pub struct OrFuture<T1, T2, R>
where
    T1: Service<R>,
    T2: Service<R, Error = <T1 as Service<R>>::Error>,
{
    #[pin]
    state: OrFutureState<T1, T2, R>,
}

impl<T1, T2, R> Future for OrFuture<T1, T2, R>
where
    T1: Service<R>,
    T2: Service<R, Error = <T1 as Service<R>>::Error>,
{
    #[allow(clippy::type_complexity)]
    type Output =
        Result<Either<T1::Output, T2::Output>, Rejection<R, Either<T1::Error, T2::Error>>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            let fut2 = match pin.state.project() {
                OrFutureStateProj::First(first, second) => match ready!(first.poll(cx)) {
                    Ok(ret) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Ok(Either::A(ret)));
                    }
                    Err(Rejection::Err(err)) => {
                        return Poll::Ready(Err(Rejection::Err(Either::A(err))))
                    }
                    Err(Rejection::Reject(req, _)) => second.call(req),
                },
                OrFutureStateProj::Second(fut) => match ready!(fut.poll(cx)) {
                    Ok(some) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Ok(Either::B(some)));
                    }
                    Err(Rejection::Err(err)) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Err(Rejection::Err(Either::A(err))));
                    }
                    Err(Rejection::Reject(req, Some(err))) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Err(Rejection::Reject(req, Some(Either::B(err)))));
                    }
                    Err(Rejection::Reject(req, None)) => {
                        self.set(OrFuture {
                            state: OrFutureState::Done,
                        });
                        return Poll::Ready(Err(Rejection::Reject(req, None)));
                    }
                },
                OrFutureStateProj::Done => panic!("poll after done"),
            };

            self.set(OrFuture {
                state: OrFutureState::Second(fut2),
            });
        }
    }
}
