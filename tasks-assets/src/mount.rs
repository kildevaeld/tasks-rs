use crate::{AssetRequest, Error};
use futures_util::{
    future::{self},
    ready,
};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tasks::{Rejection, Task};

pub fn mount<S: ToString, T>(path: S, task: T) -> Mount<T> {
    Mount::new(path, task)
}

#[derive(Clone)]
pub struct Mount<T> {
    pub(crate) path: String,
    task: T,
}

impl<T> Mount<T> {
    pub fn new<S: ToString>(path: S, task: T) -> Mount<T> {
        let mut path = path.to_string();
        if path.chars().nth(0).unwrap() != '/' {
            path.insert(0, '/');
        }
        let len = path.len();
        if path.chars().nth(len - 1).unwrap() != '/' {
            path.push('/');
        }
        Mount { path: path, task }
    }

    #[inline]
    fn starts_with(&self, path: &str) -> bool {
        if path.len() < self.path.len() {
            path.starts_with(&self.path.as_str()[0..(self.path.len() - 1)])
        } else {
            path.starts_with(self.path.as_str())
        }
    }

    #[inline]
    fn replace_path(&self, path: &mut String) {
        let p = {
            if path.ends_with("/") {
                &path[self.path.len()..]
            } else {
                &path[(self.path.len() - 1)..]
            }
            .to_string()
        };
        *path = p;
    }

    #[inline]
    fn ensure_mount(&self, req: &mut AssetRequest, path: String) {
        if req.extensions().get::<MountPath>().is_none() {
            req.extensions_mut().insert(MountPath(Vec::default()));
        }
        req.extensions_mut()
            .get_mut::<MountPath>()
            .unwrap()
            .0
            .push(path);
    }
}

impl<T> Task<AssetRequest> for Mount<T>
where
    T: Task<AssetRequest, Error = Error>,
    // T::Output: Reply + Send,
    T::Output: Send,
{
    type Output = T::Output;
    type Error = Error;

    type Future = future::Either<
        MountFuture<T>,
        future::Ready<Result<Self::Output, Rejection<AssetRequest, Self::Error>>>,
    >;

    #[inline(always)]
    fn run(&self, mut req: AssetRequest) -> Self::Future {
        if self.starts_with(req.path()) {
            let url = req.path().to_string();
            self.replace_path(req.path_mut());
            self.ensure_mount(&mut req, self.path[0..self.path.len() - 1].to_string());
            future::Either::Left(MountFuture(self.task.run(req), Some(url)))
        } else {
            future::Either::Right(future::err(Rejection::Reject(req, None)))
        }
    }
}

#[pin_project]
pub struct MountFuture<T>(#[pin] T::Future, Option<String>)
where
    T: Task<AssetRequest>;

impl<T> Future for MountFuture<T>
where
    T: Task<AssetRequest, Error = Error>,
    //T::Output: Reply,
{
    type Output = Result<T::Output, Rejection<AssetRequest, T::Error>>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        match ready!(this.0.poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(Rejection::Err(err)) => Poll::Ready(Err(Rejection::Err(err))),
            Err(Rejection::Reject(mut req, err)) => {
                *req.path_mut() = this.1.take().unwrap();
                Poll::Ready(Err(Rejection::Reject(req, err)))
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct MountPath(Vec<String>);

impl MountPath {
    pub fn push<S: ToString>(req: &mut AssetRequest, path: S) {
        if req.extensions().get::<MountPath>().is_none() {
            req.extensions_mut().insert(MountPath(Vec::default()));
        }
        req.extensions_mut()
            .get_mut::<MountPath>()
            .unwrap()
            .0
            .push(path.to_string());
    }

    pub fn real_path(&self, req: &AssetRequest) -> String {
        let mut out = self.0.join("");
        out.push_str(req.path());
        out
    }
}

pub struct RealPath;
