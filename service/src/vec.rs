use super::{Middleware, Rejection, Service};
use alloc::{sync::Arc, vec::Vec};
use futures_core::future::BoxFuture;

#[derive(Clone, Debug)]
pub struct VecService<T> {
    tasks: Arc<Vec<T>>,
}

impl<T> VecService<T> {
    pub fn new(tasks: Vec<T>) -> VecService<T> {
        VecService {
            tasks: Arc::new(tasks),
        }
    }
}

impl<T, R> Service<R> for VecService<T>
where
    T: Service<R> + Send + Sync + 'static,
    R: Send + 'static,
{
    type Output = T::Output;
    type Error = T::Error;
    type Future = BoxFuture<'static, Result<Self::Output, Rejection<R, Self::Error>>>;
    fn call(&self, mut req: R) -> Self::Future {
        let tasks = self.tasks.clone();
        let fut = async move {
            for task in tasks.iter() {
                match task.call(req).await {
                    Ok(ret) => return Ok(ret),
                    Err(Rejection::Err(err)) => return Err(Rejection::Err(err)),
                    Err(Rejection::Reject(r, _)) => {
                        req = r;
                    }
                }
            }

            Err(Rejection::Reject(req, None))
        };

        Box::pin(fut)
    }
}

// pub struct MiddlewareStack<M> {
//     stack: Arc<Vec<M>>,
// }
