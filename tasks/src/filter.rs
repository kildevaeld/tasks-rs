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
    // TODO: could include a `debug_str: &'static str` to be used in Debug impl
    func: F,
}

impl<F, R, U> Task<R> for FilterFn<F>
where
    F: 'static + Sync + Send + Clone + Fn(&mut R) -> U,
    U: TryFuture + Send,
    U::Ok: Tuple + Send,
    R: Sync + Send + 'static,
    //U::Error: IsReject,
{
    type Output = (R, U::Ok);
    type Error = U::Error;
    // type Future = IntoFuture<U>;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send>>;

    #[inline]
    fn run(&self, mut req: R) -> Self::Future {
        let func = self.func.clone();
        let future = async move {
            let ret = func(&mut req)
                .into_future()
                .await
                .map_err(|e| Rejection::Err(e))?;
            Ok((req, ret))
        };

        Box::pin(future)
        //(self.func)(req).into_future()
    }
}

pub fn filter_fn<F, R, U>(func: F) -> FilterFn<F>
where
    F: Fn(&mut R) -> U,
    U: TryFuture,
    U::Ok: Tuple,
    //U::Error: IsReject,
{
    FilterFn { func }
}

pub fn filter_fn_one<F, R, U>(
    func: F,
) -> FilterFn<impl Fn(&mut R) -> future::MapOk<U, fn(U::Ok) -> (U::Ok,)> + Copy>
where
    F: Fn(&mut R) -> U + Copy,
    U: TryFuture,
    //U::Error: IsReject,
{
    filter_fn(move |req| func(req).map_ok(tup_one as _))
}

fn tup_one<T>(item: T) -> (T,) {
    (item,)
}

// use crate::{Rejection, Task};
// use futures_core::ready;

// pub struct FilteredTask<F> {
//     filter: F,
// }

// impl<F, R> Task<R> for FilteredTask<F>
// where
//     F: Send + Sync + Filter<R> + Clone,
//     R: Sync + Send + 'static,
// {
//     type Output = F::Extract;
//     type Error = F::Error;
//     type Future = FilteredTaskFuture<F::Future>;
//     fn run(&self, req: R) -> Self::Future {
//         FilteredTaskFuture {
//             state: self.filter.filter(req),
//         }
//     }
// }

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
