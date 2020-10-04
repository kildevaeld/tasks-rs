use crate::{Asset, AssetRequest, AssetResponse, Error, Extensions, Reply};
use futures_util::{future::TryFuture, ready};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tasks::{Rejection, Task};
use tasks_vinyl::Reply as VinylReply;

use tasks_vinyl::File;

#[derive(Clone)]
pub struct Transform<T1, T2> {
    task: T1,
    transform: T2,
}

impl<T1, T2> Transform<T1, T2> {
    pub fn new(task: T1, transform: T2) -> Transform<T1, T2> {
        Transform { task, transform }
    }
}

impl<T1, T2> Task<AssetRequest> for Transform<T1, T2>
where
    T1: Send + Task<AssetRequest, Error = Error>,
    T1::Output: Reply,
    T2: Send + Clone + Task<File>,
    T2::Output: VinylReply,
    T1::Error: From<T2::Error>,
    <T2::Output as VinylReply>::Future: Send,
{
    type Output = AssetResponse;
    type Error = T1::Error;
    type Future = TransformFuture<T1, T2>;
    fn run(&self, req: AssetRequest) -> Self::Future {
        TransformFuture {
            state: TransformFutureState::Task(self.task.run(req), self.transform.clone()),
        }
    }
}

#[pin_project(project = TransformFutureStateProj)]
pub enum TransformFutureState<T1, T2>
where
    T1: Task<AssetRequest>,
    T2: Task<File>,
    T2::Output: VinylReply,
{
    Task(#[pin] T1::Future, T2),
    Transform(#[pin] T2::Future, Option<AssetRequest>),
    File(
        #[pin] <T2::Output as VinylReply>::Future,
        Option<AssetRequest>,
    ),
    Done,
}

#[pin_project]
pub struct TransformFuture<T1, T2>
where
    T1: Task<AssetRequest>,
    T2: Task<File>,
    T2::Output: VinylReply,
{
    #[pin]
    state: TransformFutureState<T1, T2>,
}

impl<T1, T2> Future for TransformFuture<T1, T2>
where
    T1: Task<AssetRequest, Error = Error>,
    T1::Output: Reply,
    T2: Task<File>,
    T1::Error: From<T2::Error>,
    T2::Output: VinylReply,
{
    type Output = Result<AssetResponse, Rejection<AssetRequest, T1::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();
            let state = match this.state.project() {
                TransformFutureStateProj::Task(future, task) => {
                    //
                    match ready!(future.poll(cx)) {
                        Ok(ret) => {
                            let ret = ret.into_response();
                            //
                            match ret.node {
                                Asset::Dir(dir) => {
                                    self.set(TransformFuture {
                                        state: TransformFutureState::Done,
                                    });
                                    return Poll::Ready(Ok(AssetResponse {
                                        request: ret.request,
                                        node: Asset::Dir(dir),
                                    }));
                                }
                                Asset::File(file) => {
                                    //
                                    Some(TransformFutureState::Transform(
                                        task.run(file),
                                        Some(ret.request),
                                    ))
                                }
                            }
                        }
                        Err(err) => return Poll::Ready(Err(err.into())),
                    }
                }
                TransformFutureStateProj::Transform(future, extensions) => {
                    //
                    match ready!(future.try_poll(cx)) {
                        Ok(o) => {
                            let exts = extensions.take().unwrap();
                            Some(TransformFutureState::File(o.into_file(), Some(exts)))
                        }
                        Err(Rejection::Err(err)) => {
                            self.set(TransformFuture {
                                state: TransformFutureState::Done,
                            });
                            return Poll::Ready(Err(Rejection::Err(err.into())));
                        }
                        Err(Rejection::Reject(_, err)) => {
                            let exts = extensions.take().unwrap();
                            self.set(TransformFuture {
                                state: TransformFutureState::Done,
                            });
                            return Poll::Ready(Err(Rejection::Reject(
                                exts,
                                err.map(|err| err.into()),
                            )));
                        }
                    }
                }
                TransformFutureStateProj::File(future, req) => {
                    //
                    match ready!(future.try_poll(cx)) {
                        Ok(file) => {
                            let exts = req.take().unwrap();
                            self.set(TransformFuture {
                                state: TransformFutureState::Done,
                            });
                            return Poll::Ready(Ok(AssetResponse {
                                node: Asset::File(file),
                                request: exts,
                            }));
                        }
                        Err(err) => {
                            let exts = req.take().unwrap();
                            self.set(TransformFuture {
                                state: TransformFutureState::Done,
                            });
                            return Poll::Ready(Err(Rejection::Reject(exts, Some(Error::Unknown))));
                        }
                    }
                }
                TransformFutureStateProj::Done => panic!("poll after done"),
            };

            if let Some(state) = state {
                self.set(TransformFuture { state });
            }
        }
    }
}
