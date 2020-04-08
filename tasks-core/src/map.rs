use super::{Rejection, Task};
use futures_core::{ready, TryFuture};
use pin_project::{pin_project, project};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct Map<T1, T2> {
    t1: T1,
    t2: T2,
}

impl<T1, T2> Map<T1, T2> {
    pub fn new(t1: T1, t2: T2) -> Map<T1, T2> {
        Map { t1, t2 }
    }
}

impl<T1, T2, R> Task<R> for Map<T1, T2>
where
    T1: Task<R>,
    T2: Send + Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    type Output = T2::Output;
    type Error = MapError<T2::Error>;
    type Future = MapFuture<T1, T2, R>;

    fn run(&self, req: R) -> Self::Future {
        MapFuture::new(self.t1.run(req), self.t2.clone())
    }
}

#[pin_project]
enum MapFutureState<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    First(#[pin] T1::Future, T2),
    Second(#[pin] T2::Future),
    Done,
}

#[pin_project]
pub struct MapFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    #[pin]
    state: MapFutureState<T1, T2, R>,
}

impl<T1, T2, R> MapFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    pub fn new(t1: T1::Future, t2: T2) -> MapFuture<T1, T2, R> {
        MapFuture {
            state: MapFutureState::First(t1, t2),
        }
    }
}

impl<T1, T2, R> Future for MapFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    type Output = Result<T2::Output, Rejection<R, MapError<T2::Error>>>;
    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            #[project]
            let fut2 = match pin.state.project() {
                MapFutureState::First(first, second) => match ready!(first.try_poll(cx)) {
                    Ok(ret) => second.run(ret),
                    Err(Rejection::Err(err)) => {
                        return Poll::Ready(Err(Rejection::Err(MapError::Err(err))))
                    }
                    Err(Rejection::Reject(ret)) => return Poll::Ready(Err(Rejection::Reject(ret))),
                },
                MapFutureState::Second(fut) => match ready!(fut.try_poll(cx)) {
                    Ok(some) => {
                        self.set(MapFuture {
                            state: MapFutureState::Done,
                        });
                        return Poll::Ready(Ok(some));
                    }
                    Err(Rejection::Err(err)) => {
                        return Poll::Ready(Err(Rejection::Err(MapError::Err(err))))
                    }
                    Err(Rejection::Reject(_)) => {
                        return Poll::Ready(Err(Rejection::Err(MapError::Reject)))
                    }
                },
                MapFutureState::Done => panic!("poll after done"),
            };

            self.set(MapFuture {
                state: MapFutureState::Second(fut2),
            });
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MapError<E> {
    Err(E),
    Reject,
}
