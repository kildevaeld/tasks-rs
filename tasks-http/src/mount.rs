use super::{Error, Request, Response};
use futures::future::{self, Ready};
use tasks_core::{
    util::{OneOf2Future, Promise},
    Rejection, Task,
};
use url::Url;

pub fn mount<S: AsRef<str>, T>(path: S, task: T) -> Mount<T>
where
    T: Task<Request, Output = Response, Error = Error>,
{
    Mount::new(path, task)
}

#[derive(Clone)]
pub struct Mount<T> {
    task: T,
    pub(crate) path: String,
}

impl<T> Mount<T> {
    pub fn new<S: AsRef<str>>(path: S, task: T) -> Mount<T> {
        let mut path = path.as_ref().to_string();
        if path.chars().nth(0).unwrap() != '/' {
            path.insert(0, '/');
        }
        let len = path.len();
        if path.chars().nth(len - 1).unwrap() != '/' {
            path.push('/');
        }
        Mount { task, path: path }
    }

    #[inline]
    fn starts_with(&self, url: &Url) -> bool {
        let path = url.path();
        if path.len() < self.path.len() {
            path.starts_with(&self.path.as_str()[0..(self.path.len() - 1)])
        } else {
            path.starts_with(self.path.as_str())
        }
    }

    #[inline]
    fn replace_path(&self, url: &mut Url) {
        let p = {
            let path = url.path();
            if path.ends_with("/") {
                &path[self.path.len()..]
            } else {
                &path[(self.path.len() - 1)..]
            }
            .to_string()
        };
        url.set_path(p.as_str());
    }

    #[inline]
    fn ensure_mount(&self, req: &mut Request, path: String) {
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

impl<T> Task<Request> for Mount<T>
where
    T: Task<Request, Output = Response, Error = Error>,
{
    type Output = Response;
    type Error = Error;

    type Future = OneOf2Future<
        T::Future,
        Ready<Result<Response, Rejection<Request, Error>>>,
        Result<Response, Rejection<Request, Error>>,
    >;

    #[inline(always)]
    fn run(&self, mut req: Request) -> Self::Future {
        let p = if self.starts_with(req.url()) {
            self.replace_path(req.url_mut());
            self.ensure_mount(&mut req, self.path[0..self.path.len() - 1].to_string());

            Promise::First(self.task.run(req))
        } else {
            Promise::Second(future::err(Rejection::Reject(req)))
        };

        OneOf2Future::new(p)
    }
}

#[derive(Debug, Default)]
pub struct MountPath(Vec<String>);

impl MountPath {
    pub fn real_path(&self, req: &Request) -> String {
        let mut out = self.0.join("");
        out.push_str(req.url().path());
        out
    }
}

pub struct RealPath;
