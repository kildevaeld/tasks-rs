use super::{Rejection, Task};
use futures_util::future::{BoxFuture, FutureExt};

pub fn boxtask<I, O, E, T>(task: T) -> Box<dyn BoxedClonableTask<I, O, E>>
where
    T: Sized + 'static + Send + Task<I, Output = O, Error = E>,
    T: Clone,
    T::Future: 'static,
{
    Box::new(BoxedTask(task))
}

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

impl<T, R> BoxedClonableTask<R, T::Output, T::Error> for BoxedTask<T>
where
    T: Task<R> + Clone + 'static + Send,
    T::Future: 'static,
{
    fn box_clone(&self) -> Box<dyn BoxedClonableTask<R, T::Output, T::Error>> {
        boxtask(self.0.clone())
    }
}

pub trait BoxedClonableTask<I, O, E>:
    Task<I, Output = O, Error = E, Future = BoxFuture<'static, Result<O, Rejection<I, E>>>> + Send
{
    fn box_clone(&self) -> Box<dyn BoxedClonableTask<I, O, E>>;
}

impl<I, O, E> Task<I> for Box<dyn BoxedClonableTask<I, O, E>> {
    type Output = O;
    type Error = E;
    #[allow(clippy::type_complexity)]
    type Future = BoxFuture<'static, Result<Self::Output, Rejection<I, Self::Error>>>;
    fn run(&self, req: I) -> Self::Future {
        self.as_ref().run(req)
    }
}

impl<I, O, E> Clone for Box<dyn BoxedClonableTask<I, O, E>> {
    fn clone(&self) -> Self {
        self.as_ref().box_clone()
    }
}
