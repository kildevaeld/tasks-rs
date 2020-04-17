use super::Extract;
use futures_core::{ready, TryFuture};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{one, Func, One};

use crate::task::{Rejection, Task};
use pin_project::pin_project;

#[derive(Clone, Copy, Debug)]
pub struct Map<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F, R> Task<R> for Map<T, F>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract> + Clone + Send,
{
    type Output = (R, (F::Output,));
    type Error = T::Error;
    type Future = MapFuture<T, F, R>;
    #[inline]
    fn run(&self, req: R) -> Self::Future {
        MapFuture {
            extract: self.filter.run(req),
            callback: self.callback.clone(),
        }
    }
}

// impl<T, F, R> Task<R> for Map<T, F>
// where
//     T: Filter<R>,
//     F: Func<T::Extract> + Clone + Send,
// {
//     type Output = F::Output;
//     type Error = T::Error;
//     type Future = MapTaskFuture<<Self as Filter<R>>::Future>;
//     fn run(&self, req: R) -> Self::Future {
//         MapTaskFuture {
//             future: self.filter(req),
//         }
//     }
// }

#[pin_project]
pub struct MapTaskFuture<F> {
    #[pin]
    future: F,
}

impl<F, R, I, E> Future for MapTaskFuture<F>
where
    F: Future<Output = Result<(R, One<I>), E>>,
{
    type Output = Result<I, Rejection<R, E>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        match ready!(this.future.poll(cx)) {
            Ok((_, o)) => Poll::Ready(Ok(o.0)),
            Err(err) => Poll::Ready(Err(Rejection::Err(err))),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct MapFuture<T: Task<R>, F, R> {
    #[pin]
    extract: T::Future,
    callback: F,
}

impl<T, F, R> Future for MapFuture<T, F, R>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract>,
{
    type Output = Result<(R, (F::Output,)), Rejection<R, T::Error>>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.project();
        match ready!(pin.extract.try_poll(cx)) {
            Ok(ret) => {
                let (req, ex) = ret.unpack();
                // let ex = (pin.callback.call(ex),);
                // Poll::Ready(Ok(ex))
                let ex = pin.callback.call(ex);
                Poll::Ready(Ok((req, (ex,))))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
