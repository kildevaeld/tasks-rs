use super::{Middleware, Next, Rejection, Task};
use futures_core::TryFuture;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct End<M, T> {
    m: M,
    t: T,
}

impl<M, T> End<M, T> {
    pub fn new(m: M, t: T) -> End<M, T> {
        End { m, t }
    }
}

impl<M, T, R> Task<R> for End<M, T>
where
    M: Middleware<R>,
    T: 'static
        + Clone
        + Send
        + Sync
        + Task<R, Output = <M as Middleware<R>>::Output, Error = <M as Middleware<R>>::Error>,
    <T as Task<R>>::Future: Send + 'static,
    R: 'static,
{
    type Output = T::Output;
    type Error = T::Error;
    type Future = M::Future;
    fn run(&self, req: R) -> Self::Future {
        let next = EndNext { t: self.t.clone() };
        self.m.run(req, next)
    }
}

#[derive(Clone)]
struct EndNext<T> {
    t: T,
}

impl<T, R: 'static> Next<R> for EndNext<T>
where
    T: Task<R> + 'static + Send + Sync,
    <T as Task<R>>::Future: Send + 'static,
{
    type Output = T::Output;
    type Error = T::Error;
    fn run(
        &self,
        req: R,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send>> {
        let future: EndNextFuture<T, R> = EndNextFuture {
            future: self.t.run(req),
        };
        Box::pin(future)
    }
}

#[pin_project]
struct EndNextFuture<T, R>
where
    T: Task<R>,
{
    #[pin]
    future: T::Future,
}

impl<T, R> Future for EndNextFuture<T, R>
where
    T: Task<R>,
{
    type Output = Result<T::Output, Rejection<R, T::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        this.future.try_poll(cx)
    }
}
