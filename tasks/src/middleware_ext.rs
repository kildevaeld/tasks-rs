use super::middleware::{IntoMiddleware, Middleware, Next};
use super::task::{IntoTask, Task};
use futures_channel::oneshot::{Receiver, Sender};
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
    type Future = MiddlewareChainFuture<S, F, Self::Input, Self::Output, Self::Error>;

    fn execute(
        &self,
        req: Self::Input,
        next: Next<Self::Input, Self::Output, Self::Error>,
    ) -> Self::Future {
        MiddlewareChainFuture::new(self.s.clone(), self.f.clone(), req, next)
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
    M: Middleware + Send + Sync + 'static,
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
    type Future = MiddlewareHandlerFuture<M, H, Self::Input, Self::Output, Self::Error>;

    fn exec(&self, req: Self::Input) -> Self::Future {
        MiddlewareHandlerFuture::new(self.m.clone(), self.h.clone(), req)
    }

    fn can_exec(&self, input: &Self::Input) -> bool {
        self.h.can_exec(input)
    }
}

// impl<M, H> ConditionalTask for MiddlewareHandler<M, H>
// where
//     M: Middleware + 'static + Sync + Send,
//     H: ConditionalTask<
//             Input = <M as Middleware>::Input,
//             Output = <M as Middleware>::Output,
//             Error = <M as Middleware>::Error,
//         > + Send
//         + Sync
//         + 'static,
//     <M as Middleware>::Input: Send + 'static,
//     <M as Middleware>::Output: Send + 'static,
//     <M as Middleware>::Error: Send + 'static,
//     <H as Task>::Future: Send + Sync,
// {
//     fn can_exec(&self, input: &Self::Input) -> bool {
//         self.h.can_exec(input)
//     }
// }

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

pub enum MiddlewareChainFutureState<R, M1, M2, Req, Res, Err> {
    Init(R, Next<Req, Res, Err>),
    Middleware1(
        M1,
        Sender<Result<Res, Err>>,
        Receiver<Req>,
        Next<Req, Res, Err>,
    ),
    Middleware2(M1, M2, Sender<Result<Res, Err>>),
    Middleware3(M1),
    None,
}

pub struct MiddlewareChainFuture<S1, S2, Req, Res, Err>
where
    S1: Middleware<Input = Req, Output = Res, Error = Err>,
    S2: Middleware<Input = Req, Output = Res, Error = Err>,
{
    s1: Arc<S1>,
    s2: Arc<S2>,
    state: MiddlewareChainFutureState<Req, S1::Future, S2::Future, Req, Res, Err>,
}

impl<S1, S2, Req, Res, Err> MiddlewareChainFuture<S1, S2, Req, Res, Err>
where
    S1: Middleware<Input = Req, Output = Res, Error = Err>,
    S2: Middleware<Input = Req, Output = Res, Error = Err>,
{
    pub fn new(
        m1: Arc<S1>,
        m2: Arc<S2>,
        req: Req,
        next: Next<Req, Res, Err>,
    ) -> MiddlewareChainFuture<S1, S2, Req, Res, Err> {
        MiddlewareChainFuture {
            s1: m1,
            s2: m2,
            state: MiddlewareChainFutureState::Init(req, next),
        }
    }
}

impl<S1, S2, Req, Res, Err> Future for MiddlewareChainFuture<S1, S2, Req, Res, Err>
where
    S1: Middleware<Input = Req, Output = Res, Error = Err>,
    S2: Middleware<Input = Req, Output = Res, Error = Err>,
{
    type Output = Result<Res, Err>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        'poll: loop {
            let state = std::mem::replace(&mut this.state, MiddlewareChainFutureState::None);
            match state {
                MiddlewareChainFutureState::Init(req, n) => {
                    let (next, sx, rx) = Next::new();
                    let fut = this.s1.execute(req, next);
                    this.state = MiddlewareChainFutureState::Middleware1(fut, sx, rx, n);
                }
                MiddlewareChainFutureState::Middleware1(mut fut, sx, mut rx, next) => {
                    match unsafe { Pin::new_unchecked(&mut fut) }.poll(cx) {
                        Poll::Pending => {}
                        Poll::Ready(Ok(ret)) => return Poll::Ready(Ok(ret)),
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    }

                    match unsafe { Pin::new_unchecked(&mut rx) }.poll(cx) {
                        Poll::Pending => {
                            this.state = MiddlewareChainFutureState::Middleware1(fut, sx, rx, next);
                            break 'poll;
                        }
                        Poll::Ready(Ok(req)) => {
                            let fut2 = this.s2.execute(req, next);
                            this.state = MiddlewareChainFutureState::Middleware2(fut, fut2, sx);
                        }
                        Poll::Ready(Err(err)) => {
                            this.state = MiddlewareChainFutureState::Middleware3(fut);
                            return Poll::Pending;
                        }
                    }
                }
                MiddlewareChainFutureState::Middleware2(fut, mut fut2, sx) => {
                    match unsafe { Pin::new_unchecked(&mut fut2) }.poll(cx) {
                        Poll::Pending => {
                            this.state = MiddlewareChainFutureState::Middleware2(fut, fut2, sx);
                            return Poll::Pending;
                        }
                        Poll::Ready(s) => {
                            sx.send(s);
                            this.state = MiddlewareChainFutureState::Middleware3(fut);
                            break;
                        }
                    }
                }
                MiddlewareChainFutureState::Middleware3(mut fut) => {
                    match unsafe { Pin::new_unchecked(&mut fut) }.poll(cx) {
                        Poll::Pending => {
                            this.state = MiddlewareChainFutureState::Middleware3(fut);
                            return Poll::Pending;
                        }
                        Poll::Ready(s) => return Poll::Ready(s),
                    }
                }
                MiddlewareChainFutureState::None => panic!("invalid state"),
            }
        }

        Poll::Pending
    }
}

pub enum MiddlewareHandlerFutureState<M1, M2, Req, Res, Err> {
    Init(Req),
    Middleware1(M1, Sender<Result<Res, Err>>, Receiver<Req>),
    Middleware2(M1, M2, Sender<Result<Res, Err>>),
    Middleware3(M1),
    None,
}

pub struct MiddlewareHandlerFuture<S1, S2, Req, Res, Err>
where
    S1: Middleware<Input = Req, Output = Res, Error = Err>,
    S2: Task<Input = Req, Output = Res, Error = Err>,
{
    s1: Arc<S1>,
    s2: Arc<S2>,
    state: MiddlewareHandlerFutureState<S1::Future, S2::Future, Req, Res, Err>,
}

impl<S1, S2, Req, Res, Err> MiddlewareHandlerFuture<S1, S2, Req, Res, Err>
where
    S1: Middleware<Input = Req, Output = Res, Error = Err>,
    S2: Task<Input = Req, Output = Res, Error = Err>,
{
    pub fn new(
        m1: Arc<S1>,
        m2: Arc<S2>,
        req: Req,
    ) -> MiddlewareHandlerFuture<S1, S2, Req, Res, Err> {
        MiddlewareHandlerFuture {
            s1: m1,
            s2: m2,
            state: MiddlewareHandlerFutureState::Init(req),
        }
    }
}

impl<S1, S2, Req, Res, Err> Future for MiddlewareHandlerFuture<S1, S2, Req, Res, Err>
where
    S1: Middleware<Input = Req, Output = Res, Error = Err>,
    S2: Task<Input = Req, Output = Res, Error = Err>,
{
    type Output = Result<Res, Err>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        'poll: loop {
            let state = std::mem::replace(&mut this.state, MiddlewareHandlerFutureState::None);
            match state {
                MiddlewareHandlerFutureState::Init(req) => {
                    let (next, sx, rx) = Next::new();
                    let fut = this.s1.execute(req, next);
                    this.state = MiddlewareHandlerFutureState::Middleware1(fut, sx, rx);
                }
                MiddlewareHandlerFutureState::Middleware1(mut fut, sx, mut rx) => {
                    match unsafe { Pin::new_unchecked(&mut fut) }.poll(cx) {
                        Poll::Pending => {
                        }
                        Poll::Ready(Ok(ret)) => return Poll::Ready(Ok(ret)),
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    }

                    match unsafe { Pin::new_unchecked(&mut rx) }.poll(cx) {
                        Poll::Pending => {
                            this.state = MiddlewareHandlerFutureState::Middleware1(fut, sx, rx);
                            break 'poll;
                        }
                        Poll::Ready(Ok(req)) => {
                            let fut2 = this.s2.exec(req);
                            this.state = MiddlewareHandlerFutureState::Middleware2(fut, fut2, sx);
                        }
                        Poll::Ready(Err(_)) => {
                            this.state = MiddlewareHandlerFutureState::Middleware3(fut);
                            return Poll::Pending;
                        }
                    }
                }
                MiddlewareHandlerFutureState::Middleware2(fut, mut fut2, sx) => {
                    match unsafe { Pin::new_unchecked(&mut fut2) }.poll(cx) {
                        Poll::Pending => {
                            this.state = MiddlewareHandlerFutureState::Middleware2(fut, fut2, sx);
                            return Poll::Pending;
                        }
                        Poll::Ready(s) => {
                            sx.send(s);
                            this.state = MiddlewareHandlerFutureState::Middleware3(fut);
                            break;
                        }
                    }
                }
                MiddlewareHandlerFutureState::Middleware3(mut fut) => {
                    match unsafe { Pin::new_unchecked(&mut fut) }.poll(cx) {
                        Poll::Pending => {
                            this.state = MiddlewareHandlerFutureState::Middleware3(fut);
                            return Poll::Pending;
                        }
                        Poll::Ready(s) => return Poll::Ready(s),
                    }
                }
                MiddlewareHandlerFutureState::None => panic!("invalid state"),
            }
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::super::middleware::*;
    use super::super::task::*;
    use super::*;
    use futures_util::future::FutureExt;
    use pin_utils::unsafe_pinned;

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

    #[test]
    fn test_task_pipe_multiple() {
        let s = middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            let o = input + 1;
            next.exec(o)
                .then(|m| futures_util::future::ready(m.map(|m| m + 1)))
        })
        .stack(middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            let o = input + 1;
            next.exec(o)
                .then(|m| futures_util::future::ready(m.map(|m| m + 1)))
        }))
        .then(task_fn!(|input: i32| futures_util::future::ok(input + 1)));

        let ret = futures_executor::block_on(s.exec(1));
        assert_eq!(ret, Ok(6));
    }

    #[test]
    fn test_task_pipe_multiple_no_next() {
        let s = middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            let o = input + 1;
            next.exec(o)
                .then(|m| futures_util::future::ready(m.map(|m| m + 1)))
        })
        .stack(middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            let o = input + 1;
            next.exec(o)
                .then(|m| futures_util::future::ready(m.map(|m| m + 1)))
        }))
        .stack(middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            futures_util::future::ok(input + 1)
        }))
        .then(task_fn!(|input: i32| futures_util::future::ok(input + 1)));

        let ret = futures_executor::block_on(s.exec(1));
        assert_eq!(ret, Ok(6));
    }

    #[test]
    fn test_task_pipe_multiple_no_next2() {
        let s = middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            let o = input + 1;
            next.exec(o)
                .then(|m| futures_util::future::ready(m.map(|m| m + 1)))
        })
        .stack(middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            futures_util::future::ok(input + 1)
        }))
        .stack(middleware_fn!(|input: i32, next: Next<i32, i32, ()>| {
            let o = input + 1;
            next.exec(o)
                .then(|m| futures_util::future::ready(m.map(|m| m + 1)))
        }))
        .then(task_fn!(|input: i32| futures_util::future::ok(input + 1)));

        let ret = futures_executor::block_on(s.exec(1));
        assert_eq!(ret, Ok(4));
    }
}
