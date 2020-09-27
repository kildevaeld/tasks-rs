use super::{AssetRequest, AssetResponse, Cache, Error, Extensions, Options, Transform};
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
{
    pub fn new(cache: C, task: T) -> Assets<T, C> {
        Assets { task, cache }
    }

    pub async fn run(&self, path: impl ToString, options: Options) -> Result<AssetResponse, Error> {
        let assets = AssetRequest {
            path: path.to_string(),
            args: options,
            extensions: Extensions::new(),
        };

        match self.task.run(assets).await {
            Ok(resp) => Ok(resp),
            Err(Rejection::Err(err)) => Err(err),
            Err(Rejection::Reject(_, Some(err))) => Err(err),
            Err(Rejection::Reject(_, None)) => Err(Error::Unknown),
        }
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
