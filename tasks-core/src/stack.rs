use super::{Middleware, Next, Rejection};
use futures_core::TryFuture;
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct Stack<M1, M2> {
    m1: M1,
    m2: M2,
}

impl<M1, M2> Stack<M1, M2> {
    pub fn new(m1: M1, m2: M2) -> Stack<M1, M2> {
        Stack { m1, m2 }
    }
}

impl<M1, M2, R> Middleware<R> for Stack<M1, M2>
where
    M1: Send + Middleware<R>,
    M2: 'static
        + Clone
        + Send
        + Sync
        + Middleware<R, Output = <M1 as Middleware<R>>::Output, Error = <M1 as Middleware<R>>::Error>,
    <M2 as Middleware<R>>::Future: Send + 'static,
    R: 'static,
{
    type Output = M1::Output;
    type Error = M2::Error;
    type Future = M1::Future;
    fn run<N: Clone + 'static + Next<R, Output = Self::Output, Error = Self::Error>>(
        &self,
        req: R,
        next: N,
    ) -> Self::Future {
        self.m1.run(req, StackNext::new(self.m2.clone(), next))
    }
}

#[derive(Clone)]
pub(crate) struct StackNext<M, N> {
    m: M,
    n: N,
}

impl<M, N> StackNext<M, N> {
    pub fn new(m: M, n: N) -> StackNext<M, N> {
        StackNext { m, n }
    }
}

impl<M, N, R> Next<R> for StackNext<M, N>
where
    M: 'static + Send + Sync + Middleware<R>,
    N: 'static
        + Clone
        + Send
        + Next<R, Output = <M as Middleware<R>>::Output, Error = <M as Middleware<R>>::Error>,
    <M as Middleware<R>>::Future: Send + 'static,
    R: 'static,
{
    type Output = M::Output;
    type Error = M::Error;
    fn run(
        &self,
        req: R,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send>> {
        let next: StackNextFuture<M, R> = StackNextFuture {
            future: self.m.run(req, self.n.clone()),
        };
        Box::pin(next)
    }
}

#[pin_project]
struct StackNextFuture<M, R>
where
    M: Middleware<R>,
{
    #[pin]
    future: M::Future,
}

impl<M, R> Future for StackNextFuture<M, R>
where
    M: Middleware<R>,
{
    type Output = Result<M::Output, Rejection<R, M::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        this.future.try_poll(cx)
    }
}
