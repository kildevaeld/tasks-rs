use crate::{Extract, Func, One, Rejection, Service};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;

#[derive(Clone, Copy, Debug)]
pub struct Map<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F, R> Service<R> for Map<T, F>
where
    T: Service<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract> + Clone + Send,
{
    type Output = (R, (F::Output,));
    type Error = T::Error;
    type Future = MapFuture<T, F, R>;
    #[inline]
    fn call(&mut self, req: R) -> Self::Future {
        MapFuture {
            extract: self.filter.call(req),
            callback: self.callback.clone(),
        }
    }
}

#[pin_project]
pub struct MapServiceFuture<F> {
    #[pin]
    future: F,
}

impl<F, R, I, E> Future for MapServiceFuture<F>
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
pub struct MapFuture<T: Service<R>, F, R> {
    #[pin]
    extract: T::Future,
    callback: F,
}

impl<T, F, R> Future for MapFuture<T, F, R>
where
    T: Service<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract>,
{
    #[allow(clippy::type_complexity)]
    type Output = Result<(R, (F::Output,)), Rejection<R, T::Error>>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pin = self.project();
        match ready!(pin.extract.poll(cx)) {
            Ok(ret) => {
                let (req, ex) = ret.unpack();
                let ex = pin.callback.call(ex);
                Poll::Ready(Ok((req, (ex,))))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
