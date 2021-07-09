use super::{Extract, HList, Rejection, Service, Tuple};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;

#[derive(Clone, Debug)]
pub struct Flatten<T> {
    task: T,
}

impl<T> Flatten<T> {
    pub fn new(task: T) -> Flatten<T> {
        Flatten { task }
    }
}

impl<T, R> Service<R> for Flatten<T>
where
    T: Service<R>,
    T::Output: Extract<R>,
{
    type Output = (
        R,
        <<<T::Output as Extract<R>>::Extract as Tuple>::HList as HList>::Tuple,
    );
    type Error = T::Error;
    type Future = FlattenFuture<T, R>;
    fn call(&self, req: R) -> Self::Future {
        FlattenFuture {
            future: self.task.call(req),
        }
    }
}

#[pin_project]
pub struct FlattenFuture<T, R>
where
    T: Service<R>,
    T::Output: Extract<R>,
{
    #[pin]
    future: T::Future,
}

impl<T, R> Future for FlattenFuture<T, R>
where
    T: Service<R>,
    T::Output: Extract<R>,
{
    type Output = Result<
        (
            R,
            <<<T::Output as Extract<R>>::Extract as Tuple>::HList as HList>::Tuple,
        ),
        Rejection<R, T::Error>,
    >;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match ready!(this.future.poll(cx)) {
            Ok(ret) => {
                //
                let (req, ret) = ret.unpack();
                let out = ret.hlist().flatten();
                Poll::Ready(Ok((req, out)))
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
