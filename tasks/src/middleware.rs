use futures_channel::oneshot::{channel, Receiver, Sender};
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project::pin_project;

pub struct Next<Req, Res, Err> {
    ret: Receiver<Result<Res, Err>>,
    pub(crate) send: Sender<Req>,
}

impl<Req, Res, Err> Next<Req, Res, Err> {
    pub(crate) fn new() -> (Next<Req, Res, Err>, Sender<Result<Res, Err>>, Receiver<Req>) {
        let (sx1, rx1) = channel();
        let (sx2, rx2) = channel();
        (
            Next {
                ret: rx1,
                send: sx2,
            },
            sx1,
            rx2,
        )
    }

    pub fn exec(self, req: Req) -> NextFuture<Res, Err> {
        if self.send.send(req).is_err() {
            // NextFuture { inner: None }
            panic!("should not happen");
        } else {
            NextFuture::new(self.ret)
        }
    }
}

#[pin_project]
pub struct NextFuture<Res, Err> {
    #[pin]
    pub(crate) inner: Receiver<Result<Res, Err>>,
}

impl<Res, Err> NextFuture<Res, Err> {
    //unsafe_pinned!(inner: Receiver<Result<Res, Err>>);
    pub fn new(chan: Receiver<Result<Res, Err>>) -> NextFuture<Res, Err> {
        NextFuture { inner: chan }
    }
}

impl<Res, Err> Future for NextFuture<Res, Err> {
    type Output = Result<Res, Err>;

    fn poll(mut self: Pin<&mut Self>, waker: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.poll(waker) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(s)) => Poll::Ready(s),
            Poll::Ready(Err(e)) => panic!("channel closed {:?}", e),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ChannelErr;

impl fmt::Display for ChannelErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Internal Server Error.")
    }
}

impl Error for ChannelErr {
    fn description(&self) -> &str {
        "Internal Server Error"
    }
}

pub trait Middleware {
    type Input;
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Self::Error>> + Send + 'static;
    fn execute(
        &self,
        req: Self::Input,
        next: Next<Self::Input, Self::Output, Self::Error>,
    ) -> Self::Future;
}

pub trait IntoMiddleware {
    type Input;
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Self::Error>> + Send + 'static;
    type Middleware: Middleware<
        Input = Self::Input,
        Output = Self::Output,
        Error = Self::Error,
        Future = Self::Future,
    >;
    fn into_middleware(self) -> Self::Middleware;
}

impl<T> IntoMiddleware for T
where
    T: Middleware,
{
    type Input = T::Input;
    type Output = T::Output;
    type Error = T::Error;
    type Future = T::Future;
    type Middleware = T;
    fn into_middleware(self) -> Self::Middleware {
        self
    }
}

impl<T> Middleware for std::sync::Arc<T>
where
    T: Middleware,
{
    type Input = T::Input;
    type Output = T::Output;
    type Error = T::Error;
    type Future = T::Future;
    fn execute(
        &self,
        req: Self::Input,
        next: Next<Self::Input, Self::Output, Self::Error>,
    ) -> Self::Future {
        self.as_ref().execute(req, next)
    }
}

use std::marker::PhantomData;

pub struct MiddlewareFn<F, I, O, E> {
    inner: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
    _e: PhantomData<E>,
}

impl<F, I, O, E, U> MiddlewareFn<F, I, O, E>
where
    F: (Fn(I, Next<I, O, E>) -> U) + Send + Sync + std::marker::Unpin,
    U: Future<Output = Result<O, E>> + Send + 'static,
{
    pub fn new(middleware: F) -> MiddlewareFn<F, I, O, E> {
        MiddlewareFn {
            inner: middleware,
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E, U> Middleware for MiddlewareFn<F, I, O, E>
where
    F: (Fn(I, Next<I, O, E>) -> U) + Send + Sync + std::marker::Unpin,
    U: Future<Output = Result<O, E>> + Send + 'static,
{
    type Input = I;
    type Output = O;
    type Error = E;
    type Future = U;

    fn execute(&self, req: I, next: Next<I, O, E>) -> Self::Future {
        (self.inner)(req, next)
    }
}

pub fn middleware_fn<F, I, O, E, U>(f: F) -> MiddlewareFn<F, I, O, E>
where
    F: (Fn(I, Next<I, O, E>) -> U) + Send + Sync + std::marker::Unpin,
    U: Future<Output = Result<O, E>> + Send + 'static,
{
    MiddlewareFn {
        inner: f,
        _i: PhantomData,
        _o: PhantomData,
        _e: PhantomData,
    }
}
