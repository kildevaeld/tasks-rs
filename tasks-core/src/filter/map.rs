use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::{ready, TryFuture};
use pin_project::pin_project;

use super::{Filter, Func};

#[derive(Clone, Copy, Debug)]
pub struct Map<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<'a, T, F, R> Filter<'a, R> for Map<T, F>
where
    T: Filter<'a, R>,
    F: Func<T::Extract> + Clone + Send,
{
    type Extract = (F::Output,);
    type Error = T::Error;
    type Future = MapFuture<'a, T, F, R>;
    #[inline]
    fn filter(&self, req: &'a R) -> Self::Future {
        MapFuture {
            extract: self.filter.filter(req),
            callback: self.callback.clone(),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct MapFuture<'a, T: Filter<'a, R>, F, R> {
    #[pin]
    extract: T::Future,
    callback: F,
}

impl<'a, T, F, R> Future for MapFuture<'a, T, F, R>
where
    T: Filter<'a, R>,
    F: Func<T::Extract>,
{
    type Output = Result<(F::Output,), T::Error>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.project();
        match ready!(pin.extract.try_poll(cx)) {
            Ok(ex) => {
                let ex = (pin.callback.call(ex),);
                Poll::Ready(Ok(ex))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
