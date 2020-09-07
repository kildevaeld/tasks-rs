use super::generic::Extract;
use super::{Rejection, Task};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Copy)]
pub struct FilterPipe<T1, T2> {
    t1: T1,
    t2: T2,
}

impl<T1, T2> FilterPipe<T1, T2> {
    pub fn new(t1: T1, t2: T2) -> FilterPipe<T1, T2> {
        FilterPipe { t1, t2 }
    }
}

impl<T1, T2, R> Task<R> for FilterPipe<T1, T2>
where
    T1: Task<R>,
    T1::Output: Extract<R>,
    T2: Send + Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    type Output = T2::Output;
    type Error = T2::Error;
    type Future = FilterPipeFuture<T1, T2, R>;

    fn run(&self, req: R) -> Self::Future {
        FilterPipeFuture::new(self.t1.run(req), self.t2.clone())
    }
}

#[pin_project(project = FilterPipeFutureStateProj)]
enum FilterPipeFutureState<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    First(#[pin] T1::Future, T2),
    Second(#[pin] T2::Future),
    Done,
}

#[pin_project]
pub struct FilterPipeFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    #[pin]
    state: FilterPipeFutureState<T1, T2, R>,
}

impl<T1, T2, R> FilterPipeFuture<T1, T2, R>
where
    T1: Task<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    pub fn new(t1: T1::Future, t2: T2) -> FilterPipeFuture<T1, T2, R> {
        FilterPipeFuture {
            state: FilterPipeFutureState::First(t1, t2),
        }
    }
}

impl<T1, T2, R> Future for FilterPipeFuture<T1, T2, R>
where
    T1: Task<R>,
    T1::Output: Extract<R>,
    T2: Clone + Task<<T1 as Task<R>>::Output, Error = <T1 as Task<R>>::Error>,
{
    type Output = Result<T2::Output, Rejection<R, T2::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            let fut2 = match pin.state.project() {
                FilterPipeFutureStateProj::First(first, second) => match ready!(first.try_poll(cx))
                {
                    Ok(ret) => second.run(ret),
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(ret, e)) => {
                        return Poll::Ready(Err(Rejection::Reject(ret, e)))
                    }
                },
                FilterPipeFutureStateProj::Second(fut) => match ready!(fut.try_poll(cx)) {
                    Ok(some) => {
                        self.set(FilterPipeFuture {
                            state: FilterPipeFutureState::Done,
                        });
                        return Poll::Ready(Ok(some));
                    }
                    Err(Rejection::Err(err)) => return Poll::Ready(Err(Rejection::Err(err))),
                    Err(Rejection::Reject(r, e)) => {
                        let (req, _) = r.unpack();
                        return Poll::Ready(Err(Rejection::Reject(req, e)));
                    }
                },
                FilterPipeFutureStateProj::Done => panic!("poll after done"),
            };

            self.set(FilterPipeFuture {
                state: FilterPipeFutureState::Second(fut2),
            });
        }
    }
}
