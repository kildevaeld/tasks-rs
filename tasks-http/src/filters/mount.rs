use crate::{Error, Request};
use futures::future::{self};
use tasks::{filter_fn_one, Rejection, Task};
use url::Url;

pub fn mount<S: AsRef<str>>(path: S) -> Mount {
    Mount::new(path)
}

pub fn realpath() -> impl Task<Request, Output = (Request, (String,)), Error = Error> + Copy {
    filter_fn_one(|req: &mut Request| {
        let m = match req.extensions().get::<MountPath>() {
            Some(p) => p.real_path(req),
            None => req.url().path().to_owned(),
        };
        future::ok(m)
    })
}

#[derive(Clone)]
pub struct Mount {
    pub(crate) path: String,
}

impl Mount {
    pub fn new<S: AsRef<str>>(path: S) -> Mount {
        let mut path = path.as_ref().to_string();
        if path.chars().nth(0).unwrap() != '/' {
            path.insert(0, '/');
        }
        let len = path.len();
        if path.chars().nth(len - 1).unwrap() != '/' {
            path.push('/');
        }
        Mount { path: path }
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

impl Task<Request> for Mount {
    type Output = (Request, ());
    type Error = Error;

    type Future = future::Ready<Result<Self::Output, Rejection<Request, Self::Error>>>;

    #[inline(always)]
    fn run(&self, mut req: Request) -> Self::Future {
        if self.starts_with(req.url()) {
            self.replace_path(req.url_mut());
            self.ensure_mount(&mut req, self.path[0..self.path.len() - 1].to_string());

            future::ok((req, ()))
        } else {
            future::err(Rejection::Reject(req))
        }
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
