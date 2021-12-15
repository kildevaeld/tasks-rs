use crate::{Middleware, Rejection, Service};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::future::Future;
use core::pin::Pin;
// use core::sync::Arc;
use core::task::{Context, Poll};
use futures_core::ready;
// use futures_core::{BoxFuture, FutureExt};
use pin_project::pin_project;
use std::marker::PhantomData;

pub fn box_service<'a, I, O, E, T>(task: T) -> BoxService<'a, I, O, E>
where
    T: Sized + 'a + Send + Sync + Service<I, Output = O, Error = E>,
    T: Clone,
    T::Future: 'a + Send,
{
    Box::new(BoxedService(task, PhantomData))
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a + Send>>;

pub trait DynamicService<'a, I, O, E>:
    Service<I, Output = O, Error = E, Future = BoxFuture<'a, Result<O, Rejection<I, E>>>> + Send + Sync
{
    fn box_clone(&self) -> BoxService<'a, I, O, E>;
}

pub type BoxService<'a, I, O, E> = Box<dyn DynamicService<'a, I, O, E> + 'a>;

struct BoxedService<'a, T>(T, PhantomData<&'a ()>);

impl<'a, T, R> Service<R> for BoxedService<'a, T>
where
    T: Service<R>,
    T::Future: 'a + Send,
{
    type Output = T::Output;
    type Error = T::Error;
    #[allow(clippy::type_complexity)]
    type Future = BoxFuture<'a, Result<Self::Output, Rejection<R, Self::Error>>>;
    fn call(&self, req: R) -> Self::Future {
        Box::pin(self.0.call(req))
    }
}

impl<'a, T, R> DynamicService<'a, R, T::Output, T::Error> for BoxedService<'a, T>
where
    T: Service<R> + Clone + 'a + Send + Sync,
    T::Future: 'a + Send,
{
    fn box_clone(&self) -> BoxService<'a, R, T::Output, T::Error> {
        box_service(self.0.clone())
    }
}

impl<'a, I, O, E> Service<I> for BoxService<'a, I, O, E> {
    type Output = O;
    type Error = E;
    #[allow(clippy::type_complexity)]
    type Future = BoxFuture<'a, Result<Self::Output, Rejection<I, Self::Error>>>;
    fn call(&self, req: I) -> Self::Future {
        self.as_ref().call(req)
    }
}

impl<'a, I, O, E> Clone for BoxService<'a, I, O, E> {
    fn clone(&self) -> Self {
        self.as_ref().box_clone()
    }
}

// pub trait DynamicMiddleware<'a, S, I, O, E>:
//     Middleware<I, S, Service = BoxService<'a, I, O, E>> + Send + Sync
// where
//     S: Service<I, Output = O, Error = E, Future = BoxFuture<'a, Result<O, Rejection<I, E>>>>,
// {
//     fn box_clone(&self) -> BoxMiddleware<'a, S, I, O, E>;
// }

// pub type BoxMiddleware<'a, S, I, O, E> = Box<dyn DynamicMiddleware<'a, S, I, O, E> + 'a>;

// impl<'a, S, I, O, E> Middleware<I, S> for BoxMiddleware<'a, S, I, O, E>
// where
//     I: 'a,
//     S: 'a + Service<I, Output = O, Error = E, Future = BoxFuture<'a, Result<O, Rejection<I, E>>>>,
//     S::Future: 'a,
//     E: 'a,
//     O: 'a,
// {
//     type Service = BoxService<'a, I, O, E>;
//     fn wrap(&self, req: S) -> Self::Service {
//         box_service::<'a, I, O, E, _>(self.as_ref().wrap(req))
//     }
// }

// impl<'a, S, I, O, E> Clone for BoxMiddleware<'a, S, I, O, E>
// where
//     S: Service<I, Output = O, Error = E, Future = BoxFuture<'a, Result<O, Rejection<I, E>>>>,
// {
//     fn clone(&self) -> Self {
//         self.as_ref().box_clone()
//     }
// }

// struct BoxedMiddleware<'a, M>(M, PhantomData<&'a ()>);

// impl<'a, M, R, T> Middleware<R, T> for BoxedMiddleware<'a, M>
// where
//     T: Service<R>,
//     T::Future: 'a + Send,
// {
//     type Service = BoxService<'a, R, T::Output, T::Error>;
//     fn wrap(&self, req: R) -> Self::Service {
//         Box::pin(self.0.call(req))
//     }
// }

// pub struct BoxOrBuilder<I, O, E> {
//     task: Vec<BoxService<I, O, E>>,
// }

// impl<I, O, E> Default for BoxOrBuilder<I, O, E> {
//     fn default() -> Self {
//         BoxOrBuilder {
//             task: Vec::default(),
//         }
//     }
// }

// impl<I, O, E> BoxOrBuilder<I, O, E> {
//     pub fn push(&mut self, task: BoxService<I, O, E>) {
//         self.task.push(task);
//     }

//     pub fn build(self) -> BoxOr<I, O, E> {
//         BoxOr {
//             task: Arc::new(self.task),
//         }
//     }
// }

// pub struct BoxOr<I, O, E> {
//     task: Arc<Vec<BoxService<I, O, E>>>,
// }

// impl<I, O, E> Clone for BoxOr<I, O, E> {
//     fn clone(&self) -> Self {
//         BoxOr {
//             task: self.task.clone(),
//         }
//     }
// }
// impl<I, O, E> Service<I> for BoxOr<I, O, E>
// where
//     I: Send,
// {
//     type Output = O;
//     type Error = E;
//     type Future = BoxOrFuture<I, O, E>;
//     fn call(&self, req: I) -> Self::Future {
//         BoxOrFuture {
//             state: BoxOrFutureState::Init(Some(self.task.clone()), Some(req)),
//         }
//     }
// }

// #[pin_project(project = BoxOrFutureStateProj)]
// enum BoxOrFutureState<I, O, E> {
//     Init(Option<Arc<Vec<BoxService<I, O, E>>>>, Option<I>),
//     Next(
//         Option<Arc<Vec<BoxService<I, O, E>>>>,
//         #[pin] BoxFuture<'static, Result<O, Rejection<I, E>>>,
//         usize,
//     ),
//     Done,
// }

// #[pin_project]
// pub struct BoxOrFuture<I, O, E> {
//     #[pin]
//     state: BoxOrFutureState<I, O, E>,
// }

// impl<I, O, E> Future for BoxOrFuture<I, O, E> {
//     type Output = Result<O, Rejection<I, E>>;
//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         loop {
//             let this = self.as_mut().project();
//             let state = match this.state.project() {
//                 BoxOrFutureStateProj::Init(tasks, req) => {
//                     //
//                     let tasks = tasks.take().unwrap();
//                     let req = req.take().unwrap();
//                     let task = match tasks.get(0) {
//                         Some(s) => s,
//                         None => {
//                             self.set(BoxOrFuture {
//                                 state: BoxOrFutureState::Done,
//                             });
//                             return Poll::Ready(Err(Rejection::Reject(req, None)));
//                         }
//                     };

//                     let future = task.call(req);
//                     Some(BoxOrFutureState::Next(Some(tasks), future, 1))
//                 }
//                 BoxOrFutureStateProj::Next(tasks, future, next) => {
//                     //
//                     match ready!(future.poll(cx)) {
//                         Ok(ret) => {
//                             self.set(BoxOrFuture {
//                                 state: BoxOrFutureState::Done,
//                             });
//                             return Poll::Ready(Ok(ret));
//                         }
//                         Err(Rejection::Err(err)) => {
//                             self.set(BoxOrFuture {
//                                 state: BoxOrFutureState::Done,
//                             });
//                             return Poll::Ready(Err(Rejection::Err(err)));
//                         }
//                         Err(Rejection::Reject(req, err)) => {
//                             let tasks = tasks.take().unwrap();

//                             let task = match tasks.get(*next) {
//                                 Some(some) => some,
//                                 None => return Poll::Ready(Err(Rejection::Reject(req, err))),
//                             };

//                             let future = task.call(req);
//                             Some(BoxOrFutureState::Next(Some(tasks), future, *next + 1))
//                         }
//                     }
//                 }
//                 BoxOrFutureStateProj::Done => None,
//             };

//             if let Some(state) = state {
//                 self.set(BoxOrFuture { state });
//             }
//         }
//     }
// }
