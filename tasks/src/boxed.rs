use crate::{Rejection, Task};
use futures_core::ready;
use futures_util::future::{BoxFuture, FutureExt};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub trait DynamicTask<I, O, E>:
    Task<I, Output = O, Error = E, Future = BoxFuture<'static, Result<O, Rejection<I, E>>>>
    + Send
    + Sync
{
    fn box_clone(&self) -> BoxTask<I, O, E>;
}

pub fn boxtask<I, O, E, T>(task: T) -> BoxTask<I, O, E>
where
    T: Sized + 'static + Send + Sync + Task<I, Output = O, Error = E>,
    T: Clone,
    T::Future: 'static,
{
    Box::new(BoxedTask(task))
}

pub type BoxTask<I, O, E> = Box<dyn DynamicTask<I, O, E>>;

struct BoxedTask<T>(T);

impl<T, R> Task<R> for BoxedTask<T>
where
    T: Task<R>,
    T::Future: 'static,
{
    type Output = T::Output;
    type Error = T::Error;
    #[allow(clippy::type_complexity)]
    type Future = BoxFuture<'static, Result<Self::Output, Rejection<R, Self::Error>>>;
    fn run(&self, req: R) -> Self::Future {
        self.0.run(req).boxed()
    }
}

impl<T, R> DynamicTask<R, T::Output, T::Error> for BoxedTask<T>
where
    T: Task<R> + Clone + 'static + Send + Sync,
    T::Future: 'static,
{
    fn box_clone(&self) -> BoxTask<R, T::Output, T::Error> {
        boxtask(self.0.clone())
    }
}

impl<I, O, E> Task<I> for BoxTask<I, O, E> {
    type Output = O;
    type Error = E;
    #[allow(clippy::type_complexity)]
    type Future = BoxFuture<'static, Result<Self::Output, Rejection<I, Self::Error>>>;
    fn run(&self, req: I) -> Self::Future {
        self.as_ref().run(req)
    }
}

impl<I, O, E> Clone for BoxTask<I, O, E> {
    fn clone(&self) -> Self {
        self.as_ref().box_clone()
    }
}

pub struct BoxOrBuilder<I, O, E> {
    task: Vec<BoxTask<I, O, E>>,
}

impl<I, O, E> Default for BoxOrBuilder<I, O, E> {
    fn default() -> Self {
        BoxOrBuilder {
            task: Vec::default(),
        }
    }
}

impl<I, O, E> BoxOrBuilder<I, O, E> {
    pub fn push(&mut self, task: BoxTask<I, O, E>) {
        self.task.push(task);
    }

    pub fn build(self) -> BoxOr<I, O, E> {
        BoxOr {
            task: Arc::new(self.task),
        }
    }
}

pub struct BoxOr<I, O, E> {
    task: Arc<Vec<BoxTask<I, O, E>>>,
}

impl<I, O, E> Clone for BoxOr<I, O, E> {
    fn clone(&self) -> Self {
        BoxOr {
            task: self.task.clone(),
        }
    }
}
impl<I, O, E> Task<I> for BoxOr<I, O, E>
where
    I: Send,
{
    type Output = O;
    type Error = E;
    type Future = BoxOrFuture<I, O, E>;
    fn run(&self, req: I) -> Self::Future {
        BoxOrFuture {
            state: BoxOrFutureState::Init(Some(self.task.clone()), Some(req)),
        }
    }
}

#[pin_project(project = BoxOrFutureStateProj)]
enum BoxOrFutureState<I, O, E> {
    Init(Option<Arc<Vec<BoxTask<I, O, E>>>>, Option<I>),
    Next(
        Option<Arc<Vec<BoxTask<I, O, E>>>>,
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

                    let future = task.run(req);
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

                            let future = task.run(req);
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
