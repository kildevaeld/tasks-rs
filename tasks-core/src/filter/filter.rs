use super::generic::Tuple;
use futures_core::TryFuture;
use futures_util::{
    future::{self, IntoFuture},
    TryFutureExt,
};

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

pub trait IsReject {}

pub trait Filter<'a, R> {
    type Extract: Tuple; // + Send;
    type Error;
    ///: IsReject;
    type Future: Future<Output = Result<Self::Extract, Self::Error>> + Send;
    fn filter(&self, req: &'a R) -> Self::Future;
}

#[derive(Copy, Clone)]
#[allow(missing_debug_implementations)]
pub(crate) struct FilterFn<F> {
    // TODO: could include a `debug_str: &'static str` to be used in Debug impl
    func: F,
}

impl<'a, F, R, U> Filter<'a, R> for FilterFn<F>
where
    F: Fn(&R) -> U,
    U: TryFuture + Send + 'static,
    U::Ok: Tuple + Send,
    //U::Error: IsReject,
{
    type Extract = U::Ok;
    type Error = U::Error;
    type Future = IntoFuture<U>;
    // type Future =
    //     Pin<Box<dyn Future<Output = Result<Self::Extract, Self::Error>> + Send + 'static>>;

    #[inline]
    fn filter(&self, req: &'a R) -> Self::Future {
        (self.func)(req).into_future()
    }
}

pub(crate) fn filter_fn<F, R, U>(func: F) -> FilterFn<F>
where
    F: Fn(&R) -> U,
    U: TryFuture,
    U::Ok: Tuple,
    //U::Error: IsReject,
{
    FilterFn { func }
}

pub(crate) fn filter_fn_one<F, R, U>(
    func: F,
) -> FilterFn<impl Fn(&R) -> future::MapOk<U, fn(U::Ok) -> (U::Ok,)> + Copy>
where
    F: Fn(&R) -> U + Copy,
    U: TryFuture,
    //U::Error: IsReject,
{
    filter_fn(move |route| func(route).map_ok(tup_one as _))
}

fn tup_one<T>(item: T) -> (T,) {
    (item,)
}

// use crate::{Rejection, Task};
// use futures_core::ready;
// use pin_project::pin_project;
// use std::task::{Context, Poll};

// pub struct FilteredTask<F> {
//     filter: F,
// }

// impl<F, R> Task<R> for FilteredTask<F>
// where
//     F: Send + Sync + Filter<'static, R> + Clone,
//     R: Sync + Send + 'static,
// {
//     type Output = F::Extract;
//     type Error = F::Error;
//     //type Future = FilteredTaskFuture<F::Future, R>;
//     type Future =
//         Pin<Box<dyn Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send>>;
//     fn run(&self, req: R) -> Self::Future {
//         let filter = self.filter.clone();
//         let future = async move {
//             let ret = match self.filter.filter(&req).await {
//                 Ok(ret) => ret,
//                 Err(e) => panic!("panic"),
//             };
//             Ok(ret)
//         };

//         Box::pin(future)
//         // FilteredTaskFuture {
//         //     req: req,
//         //     state: self.filter.filter(&req),
//         // }
//     }
// }

// // enum FilteredTaskFutureState<F, C> {
// //     Filter(F)
// //     Future(C)
// // }

// #[pin_project]
// pub struct FilteredTaskFuture<F, R> {
//     #[pin]
//     state: F,
//     req: R,
// }

// impl<F, R> Future for FilteredTaskFuture<F, R>
// where
//     F: TryFuture,
// {
//     type Output = Result<F::Ok, Rejection<R, F::Error>>;
//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         let this = self.as_mut().project();
//         match ready!(this.state.try_poll(cx)) {
//             Ok(ret) => {}
//             Err(_) => {}
//         };

//         Poll::Pending
//     }
// }
