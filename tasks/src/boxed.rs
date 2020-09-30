use crate::{Rejection, Task};
use futures_util::future::{BoxFuture, FutureExt};

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
