use crate::{Rejection, Service};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::future::Future;
use core::pin::Pin;
// use core::sync::Arc;
use core::task::{Context, Poll};
use futures_core::ready;
// use futures_core::{BoxFuture, FutureExt};
use pin_project::pin_project;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a + Send>>;

pub trait DynamicService<I, O, E>:
    Service<I, Output = O, Error = E, Future = BoxFuture<'static, Result<O, Rejection<I, E>>>>
    + Send
    + Sync
{
    fn box_clone(&self) -> BoxService<I, O, E>;
}

pub fn box_service<I, O, E, T>(task: T) -> BoxService<I, O, E>
where
    T: Sized + 'static + Send + Sync + Service<I, Output = O, Error = E>,
    T: Clone,
    T::Future: 'static + Send,
{
    Box::new(BoxedService(task))
}

pub type BoxService<I, O, E> = Box<dyn DynamicService<I, O, E>>;

struct BoxedService<T>(T);

impl<T, R> Service<R> for BoxedService<T>
where
    T: Service<R>,
    T::Future: 'static + Send,
{
    type Output = T::Output;
    type Error = T::Error;
    #[allow(clippy::type_complexity)]
    type Future = BoxFuture<'static, Result<Self::Output, Rejection<R, Self::Error>>>;
    fn call(&self, req: R) -> Self::Future {
        Box::pin(self.0.call(req))
    }
}

impl<T, R> DynamicService<R, T::Output, T::Error> for BoxedService<T>
where
    T: Service<R> + Clone + 'static + Send + Sync,
    T::Future: 'static + Send,
{
    fn box_clone(&self) -> BoxService<R, T::Output, T::Error> {
        box_service(self.0.clone())
    }
}

impl<I, O, E> Service<I> for BoxService<I, O, E> {
    type Output = O;
    type Error = E;
    #[allow(clippy::type_complexity)]
    type Future = BoxFuture<'static, Result<Self::Output, Rejection<I, Self::Error>>>;
    fn call(&self, req: I) -> Self::Future {
        self.as_ref().call(req)
    }
}

impl<I, O, E> Clone for BoxService<I, O, E> {
    fn clone(&self) -> Self {
        self.as_ref().box_clone()
    }
}

pub struct BoxOrBuilder<I, O, E> {
    task: Vec<BoxService<I, O, E>>,
}

impl<I, O, E> Default for BoxOrBuilder<I, O, E> {
    fn default() -> Self {
        BoxOrBuilder {
            task: Vec::default(),
        }
    }
}

impl<I, O, E> BoxOrBuilder<I, O, E> {
    pub fn push(&mut self, task: BoxService<I, O, E>) {
        self.task.push(task);
    }

    pub fn build(self) -> BoxOr<I, O, E> {
        BoxOr {
            task: Arc::new(self.task),
        }
    }
}

pub struct BoxOr<I, O, E> {
    task: Arc<Vec<BoxService<I, O, E>>>,
}

impl<I, O, E> Clone for BoxOr<I, O, E> {
    fn clone(&self) -> Self {
        BoxOr {
            task: self.task.clone(),
        }
    }
}
impl<I, O, E> Service<I> for BoxOr<I, O, E>
where
    I: Send,
{
    type Output = O;
    type Error = E;
    type Future = BoxOrFuture<I, O, E>;
    fn call(&self, req: I) -> Self::Future {
        BoxOrFuture {
            state: BoxOrFutureState::Init(Some(self.task.clone()), Some(req)),
        }
    }
}

#[pin_project(project = BoxOrFutureStateProj)]
enum BoxOrFutureState<I, O, E> {
    Init(Option<Arc<Vec<BoxService<I, O, E>>>>, Option<I>),
    Next(
        Option<Arc<Vec<BoxService<I, O, E>>>>,
        #[pin] BoxFuture<'static, Result<O, Rejection<I, E>>>,
        usize,
    ),
    Done,
}

#[pin_project]
pub struct BoxOrFuture<I, O, E> {
    #[pin]
    state: BoxOrFutureState<I, O, E>,
}

impl<I, O, E> Future for BoxOrFuture<I, O, E> {
    type Output = Result<O, Rejection<I, E>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();
            let state = match this.state.project() {
                BoxOrFutureStateProj::Init(tasks, req) => {
                    //
                    let tasks = tasks.take().unwrap();
                    let req = req.take().unwrap();
                    let task = match tasks.get(0) {
                        Some(s) => s,
                        None => {
                            self.set(BoxOrFuture {
                                state: BoxOrFutureState::Done,
                            });
                            return Poll::Ready(Err(Rejection::Reject(req, None)));
                        }
                    };

                    let future = task.call(req);
                    Some(BoxOrFutureState::Next(Some(tasks), future, 1))
                }
                BoxOrFutureStateProj::Next(tasks, future, next) => {
                    //
                    match ready!(future.poll(cx)) {
                        Ok(ret) => {
                            self.set(BoxOrFuture {
                                state: BoxOrFutureState::Done,
                            });
                            return Poll::Ready(Ok(ret));
                        }
                        Err(Rejection::Err(err)) => {
                            self.set(BoxOrFuture {
                                state: BoxOrFutureState::Done,
                            });
                            return Poll::Ready(Err(Rejection::Err(err)));
                        }
                        Err(Rejection::Reject(req, err)) => {
                            let tasks = tasks.take().unwrap();

                            let task = match tasks.get(*next) {
                                Some(some) => some,
                                None => return Poll::Ready(Err(Rejection::Reject(req, err))),
                            };

                            let future = task.call(req);
                            Some(BoxOrFutureState::Next(Some(tasks), future, *next + 1))
                        }
                    }
                }
                BoxOrFutureStateProj::Done => None,
            };

            if let Some(state) = state {
                self.set(BoxOrFuture { state });
            }
        }
    }
}
