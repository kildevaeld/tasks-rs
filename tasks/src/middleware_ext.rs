use super::middleware::{IntoMiddleware, Middleware, Next};
use super::task::{ConditionalTask, IntoTask, Task};
use futures_channel::oneshot::Sender;
use futures_util::future::FutureExt;
use pin_utils::unsafe_pinned;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct MiddlewareChain<S, F> {
    s: Arc<S>,
    f: Arc<F>,
}

impl<S, F> Middleware for MiddlewareChain<S, F>
where
    S: Middleware + Send + Sync + 'static,
    <S as Middleware>::Input: Send,
    <S as Middleware>::Output: Send,
    <S as Middleware>::Error: Send,
    F: Middleware<
            Input = <S as Middleware>::Input,
            Output = <S as Middleware>::Output,
            Error = <S as Middleware>::Error,
        > + Send
        + Sync
        + 'static,
    <F as Middleware>::Future: Sync,
{
    type Input = S::Input;
    type Output = S::Output;
    type Error = S::Error;
    type Future = MiddlewareChainFuture<Self::Output, Self::Error, S::Future>;

    fn execute(
        &self,
        req: Self::Input,
        next: Next<Self::Input, Self::Output, Self::Error>,
    ) -> Self::Future {
        let (n, sx, rx) = Next::new();
        let f = self.f.clone();
        let fut = rx.then(move |req| match req {
            Ok(req) => f.execute(req, next),
            Err(e) => panic!("reciever channel closed: {}", e),
        });

        let fut2 = self.s.execute(req, n);

        MiddlewareChainFuture::new(Box::pin(fut), fut2, sx)
    }
}

#[derive(Clone, Copy)]
enum State {
    Exec,
    Done,
}

pub struct MiddlewareChainFuture<Res, E, F> {
    s: Pin<Box<dyn Future<Output = Result<Res, E>> + Send + Sync>>,
    f: F,
    sx: Option<Sender<Result<Res, E>>>,
    state: State,
}

impl<Res, E, F> MiddlewareChainFuture<Res, E, F> {
    unsafe_pinned!(s: Pin<Box<dyn Future<Output = Result<Res, E>> + Send + Sync>>);
    unsafe_pinned!(f: F);
    unsafe_pinned!(sx: Option<Sender<Result<Res, E>>>);
    unsafe_pinned!(state: State);

    pub fn new(
        s: Pin<Box<dyn Future<Output = Result<Res, E>> + Send + Sync>>,
        f: F,
        sx: Sender<Result<Res, E>>,
    ) -> MiddlewareChainFuture<Res, E, F> {
        MiddlewareChainFuture {
            s: s,
            f,
            sx: Some(sx),
            state: State::Exec,
        }
    }
}

impl<Res, E, F> Future for MiddlewareChainFuture<Res, E, F>
where
    F: Future<Output = Result<Res, E>>,
{
    type Output = Result<Res, E>;
    fn poll(mut self: Pin<&mut Self>, waker: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.state;

        match self.as_mut().f().poll(waker) {
            Poll::Pending => {},
            Poll::Ready(m) => return Poll::Ready(m)
        }

        match state {
            State::Exec => match self.as_mut().s().poll(waker) {
                Poll::Pending => {}
                Poll::Ready(m) => {
                    match self.as_mut().sx().take().unwrap().send(m) {
                        Err(_) => panic!("send channel closed"),
                        _ => {
                            *self.as_mut().state() = State::Done;
                        }
                    };
                }
            },
            State::Done => panic!("already exhusted"),
        };

        Poll::Pending
    }
}

pub struct MiddlewareHandler<M, H> {
    m: Arc<M>,
    h: Arc<H>,
}

impl<M, H> MiddlewareHandler<M, H> {
    pub fn new(m: M, h: H) -> MiddlewareHandler<M, H> {
        MiddlewareHandler {
            m: Arc::new(m),
            h: Arc::new(h),
        }
    }
}

impl<M, H> Task for MiddlewareHandler<M, H>
where
    M: Middleware,
    H: Task<
            Input = <M as Middleware>::Input,
            Output = <M as Middleware>::Output,
            Error = <M as Middleware>::Error,
        > + Send
        + Sync
        + 'static,
    <M as Middleware>::Input: Send + 'static,
    <M as Middleware>::Output: Send + 'static,
    <M as Middleware>::Error: Send + 'static,
    <H as Task>::Future: Send + Sync,
{
    type Input = M::Input;
    type Output = M::Output;
    type Error = M::Error;
    type Future = MiddlewareChainFuture<Self::Output, Self::Error, M::Future>;

    fn exec(&self, req: Self::Input) -> Self::Future {
        let (n, sx, rx) = Next::new();
        let f = self.h.clone();
        let fut = rx.then(move |req| match req {
            Ok(req) => f.exec(req),
            Err(e) => {
                // The channel was closed, which means the sender was dropped
                panic!("channel was closed: {}", e);
            }
        });

        let fut2 = self.m.execute(req, n);

        MiddlewareChainFuture::new(Box::pin(fut), fut2, sx)
    }
}

impl<M, H> ConditionalTask for MiddlewareHandler<M, H>
where
    M: Middleware,
    H: ConditionalTask<
            Input = <M as Middleware>::Input,
            Output = <M as Middleware>::Output,
            Error = <M as Middleware>::Error,
        > + Send
        + Sync
        + 'static,
    <M as Middleware>::Input: Send + 'static,
    <M as Middleware>::Output: Send + 'static,
    <M as Middleware>::Error: Send + 'static,
    <H as Task>::Future: Send + Sync,
{
    fn can_exec(&self, input: &Self::Input) -> bool {
        self.h.can_exec(input)
    }
}

pub trait MiddlewareExt: Middleware + Sized {
    fn stack<M: IntoMiddleware>(self, other: M) -> MiddlewareChain<Self, M::Middleware>;
    fn then<M: IntoTask<Input = Self::Input, Output = Self::Output, Error = Self::Error>>(
        self,
        handler: M,
    ) -> MiddlewareHandler<Self, M::Task>;
}

impl<T> MiddlewareExt for T
where
    T: Middleware,
{
    fn stack<M: IntoMiddleware>(self, other: M) -> MiddlewareChain<Self, M::Middleware> {
        MiddlewareChain {
            s: Arc::new(self),
            f: Arc::new(other.into_middleware()),
        }
    }

    fn then<M: IntoTask<Input = Self::Input, Output = Self::Output, Error = Self::Error>>(
        self,
        handler: M,
    ) -> MiddlewareHandler<Self, M::Task> {
        MiddlewareHandler::new(self, handler.into_task())
    }
}

#[cfg(test)]
mod tests {
    use super::super::middleware::*;
    use super::super::task::*;
    use super::*;

    use super::super::*;

    #[test]
    fn test_task_pipe() {
        let s = middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            let o = input + 1;
            next.exec(o)
                .then(|m| futures_util::future::ready(m.map(|m| m + 1)))
        })
        .then(task_fn!(|input: i32| futures_util::future::ok(input + 1)));

        let ret = futures_executor::block_on(s.exec(1));
        assert_eq!(ret, Ok(4));
    }

    #[test]
    fn test_task_pipe_no_next() {
        let s = middleware_fn!(|input: i32, _next: Next<i32, i32, ()>| {
            let o = input + 1;
            // next.execute(o)
            //     .then(|m| futures_util::future::ready(m.map(|m| m + 1)))
            futures_util::future::ok(o)
        })
        .then(task_fn!(|input: i32| futures_util::future::ok(input + 1)));

        let ret = futures_executor::block_on(s.exec(1));
        assert_eq!(ret, Ok(2));
    }

}
