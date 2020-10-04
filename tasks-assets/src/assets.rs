use super::{
    cache::{CacheSetOptions, NullCache},
    AssetRequest, AssetResponse, Cache, Error, Extensions, Options, Reply, Transform,
};
use futures_util::{
    future::{self, BoxFuture},
    FutureExt,
};
use mime::Mime;
use std::future::Future;
use std::sync::Arc;
use tasks::{middleware, task, BoxTask, Middleware, Rejection, Task, TaskExt};
use tasks_vinyl::{File, Path};

// #[derive(Debug)]
// pub struct NodeFile {
//     pub path: Path,
//     pub mime: Mime,
//     pub size: u64,
// }

// impl NodeFile {
//     pub fn new(path: impl Into<Path>, mime: impl Into<Mime>, size: u64) -> NodeFile {
//         NodeFile {
//             path: path.into(),
//             mime: mime.into(),
//             size: size,
//         }
//     }
// }

#[derive(Debug, PartialEq)]
pub enum Node {
    File(Path, Mime, u64),
    Dir(Path),
}

#[derive(Debug, PartialEq)]
pub struct Dir {
    pub children: Vec<Node>,
    pub path: Path,
}

impl Dir {
    pub fn new(path: impl Into<Path>, children: Vec<Node>) -> Dir {
        Dir {
            path: path.into(),
            children,
        }
    }
}

#[derive(Debug)]
pub enum Asset {
    File(File),
    Dir(Dir),
}

impl Asset {
    pub fn dir(path: impl Into<Path>, children: Vec<Node>) -> Asset {
        Asset::Dir(Dir::new(path, children))
    }
}

pub struct AssetsBuilder<T, C> {
    task: T,
    cache: C,
}

impl<T, C> AssetsBuilder<T, C>
where
    T: Task<AssetRequest, Error = Error> + Send + Sync + Clone + 'static,
    T::Output: Reply,
    C: Cache + 'static,
    T::Future: 'static + Send,
{
    pub fn new(cache: C, task: T) -> AssetsBuilder<T, C> {
        AssetsBuilder { task, cache }
    }

    pub fn transform<T2>(self, transform: T2) -> AssetsBuilder<Transform<T, T2>, C>
    where
        T2: Task<File, Output = File>,
    {
        AssetsBuilder {
            task: Transform::new(self.task, transform),
            cache: self.cache,
        }
    }

    pub fn build(self) -> Assets {
        Assets {
            task: entry_point(self.cache).wrap(self.task).boxed(),
        }
    }
}

#[derive(Clone)]
pub struct Assets {
    task: BoxTask<AssetRequest, Asset, Error>,
}

impl Assets {
    pub fn new<T>(task: T) -> AssetsBuilder<T, NullCache>
    where
        T: Task<AssetRequest, Error = Error> + Send + Sync + Clone + 'static,
        T::Output: Reply,
        T::Future: 'static + Send,
    {
        AssetsBuilder::new(NullCache, task)
    }
}

impl Assets {
    pub fn get(
        &self,
        req: AssetRequest,
    ) -> impl Future<Output = Result<Asset, Error>> + 'static + Send {
        self.task.run(req).then(|ret| match ret {
            Ok(resp) => future::ok(resp),
            Err(Rejection::Err(err)) => future::err(err),
            Err(Rejection::Reject(_, Some(err))) => future::err(err),
            Err(Rejection::Reject(_, None)) => future::err(Error::NotFound),
        })
    }
}

impl Task<AssetRequest> for Assets {
    type Output = Asset;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Asset, Rejection<AssetRequest, Error>>>;
    fn run(&self, req: AssetRequest) -> Self::Future {
        self.get(req)
            .then(|ret| async move {
                match ret {
                    Ok(ret) => Ok(ret),
                    Err(err) => Err(Rejection::Err(err)),
                }
            })
            .boxed()
    }
}

fn entry_point<T, C>(
    cache: C,
) -> impl Middleware<
    AssetRequest,
    T,
    Task = impl Task<AssetRequest, Output = Asset, Error = Error> + Send + Clone,
> + Clone
       + Send
where
    T: Task<AssetRequest, Error = Error> + Clone + Send + Sync,
    T::Output: Reply,
    C: Cache,
{
    let cache = Arc::new(cache);

    middleware!(move |task: T, req: AssetRequest| {
        //
        let cache = cache.clone();
        async move {
            let key = req.cache_key().unwrap();

            if let Some(file) = cache.get(&key).await {
                Ok(Asset::File(file))
            } else {
                let resp = task.run(req).await?.into_response();
                let node = resp.into_node();
                let node = if let Asset::File(mut file) = node {
                    cache
                        .set(&key, &mut file, CacheSetOptions {})
                        .await
                        .map_err(Rejection::Err)?;
                    Asset::File(file)
                } else {
                    node
                };

                Ok(node)
            }
        }
    })
}
