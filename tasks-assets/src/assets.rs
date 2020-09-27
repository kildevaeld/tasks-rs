use super::{AssetRequest, AssetResponse, Cache, Error, Extensions, Options, Transform};
use futures_util::{future, FutureExt};
use std::future::Future;
use tasks::{Rejection, Task};
use tasks_vinyl::File;

#[derive(Debug)]
pub struct Dir {
    pub name: String,
    pub children: Vec<Node>,
    pub path: String,
}

#[derive(Debug)]
pub enum Node {
    File(File),
    Dir(Dir),
}

#[derive(Clone, Copy)]
pub struct Assets<T, C> {
    task: T,
    cache: C,
}

impl<T, C> Assets<T, C>
where
    T: Task<AssetRequest, Output = AssetResponse, Error = Error>,
    T::Future: 'static,
{
    pub fn new(cache: C, task: T) -> Assets<T, C> {
        Assets { task, cache }
    }

    pub fn run(
        &self,
        path: impl ToString,
        options: Options,
    ) -> impl Future<Output = Result<AssetResponse, Error>> + 'static + Send {
        let assets = AssetRequest {
            path: path.to_string(),
            args: options,
            extensions: Extensions::new(),
        };

        self.task.run(assets).then(|ret| match ret {
            Ok(resp) => future::ok(resp),
            Err(Rejection::Err(err)) => future::err(err),
            Err(Rejection::Reject(_, Some(err))) => future::err(err),
            Err(Rejection::Reject(_, None)) => future::err(Error::Unknown),
        })
    }

    pub fn transform<T2>(self, transform: T2) -> Assets<Transform<T, T2>, C>
    where
        T2: Task<File, Output = File, Error = Error>,
    {
        Assets {
            task: Transform::new(self.task, transform),
            cache: self.cache,
        }
    }
}

impl<T, C> Task<AssetRequest> for Assets<T, C>
where
    T: Task<AssetRequest>,
{
    type Output = T::Output;
    type Error = T::Error;
    type Future = T::Future;
    fn run(&self, req: AssetRequest) -> Self::Future {
        self.task.run(req)
    }
}
