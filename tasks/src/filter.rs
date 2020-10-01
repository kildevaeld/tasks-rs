use super::{Rejection, Task, Tuple};
use futures_core::{ready, TryFuture};
use futures_util::{future, TryFutureExt};
use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub struct FilterFn<F> {
    func: F,
}

impl<F, R, U> Task<R> for FilterFn<F>
where
    F: 'static + Sync + Send + Clone + Fn(&mut R) -> U,
    U: TryFuture + Send,
    U::Ok: Tuple + Send,
    R: Sync + Send + 'static,
{
    type Output = (R, U::Ok);
    type Error = U::Error;
    #[allow(clippy::type_complexity)]
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send>>;

    #[inline]
    fn run(&self, mut req: R) -> Self::Future {
        let func = self.func.clone();
        let future = async move {
            let ret = func(&mut req).into_future().await.map_err(Rejection::Err)?;
            Ok((req, ret))
        };

        Box::pin(future)
    }
}

pub fn filter_fn<F, R, U>(func: F) -> FilterFn<F>
where
    F: Fn(&mut R) -> U,
    U: TryFuture,
    U::Ok: Tuple,
{
    FilterFn { func }
}

pub fn filter_fn_one<F, R, U>(
    func: F,
) -> FilterFn<impl Fn(&mut R) -> future::MapOk<U, fn(U::Ok) -> (U::Ok,)> + Copy>
where
    F: Fn(&mut R) -> U + Copy,
    U: TryFuture,
{
    filter_fn(move |req| func(req).map_ok(tup_one as _))
}

fn tup_one<T>(item: T) -> (T,) {
    (item,)
}

#[pin_project]
pub struct FilteredTaskFuture<F> {
    #[pin]
    state: F,
}

impl<F, R, O, E> Future for FilteredTaskFuture<F>
where
    F: Future<Output = Result<(R, O), E>>,
{
    type Output = Result<O, Rejection<R, E>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        match ready!(this.state.poll(cx)) {
            Ok((_, ret)) => Poll::Ready(Ok(ret)),
            Err(err) => Poll::Ready(Err(Rejection::<R, _>::Err(err))),
        }
    }
}
