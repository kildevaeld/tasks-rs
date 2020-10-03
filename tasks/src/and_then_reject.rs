use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Extract, Func, Rejection, Task};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub struct AndThenReject<T, F, O> {
    pub(super) filter: T,
    pub(super) callback: F,
    _v: PhantomData<O>,
}

impl<T, F, O, R> Task<R> for AndThenReject<T, F, O>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract> + Clone + Send,
    F::Output: Future<Output = Result<O, Rejection<R, T::Error>>> + Send,
    R: Send,
{
    type Output = (R, (<F::Output as TryFuture>::Ok,));
    type Error = T::Error;
    type Future = AndThenRejectFuture<T, F, R, O>;
    #[inline]
    fn run(&self, req: R) -> Self::Future {
        AndThenRejectFuture {
            state: State::First(self.filter.run(req), self.callback.clone()),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct AndThenRejectFuture<T, F, R, O>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract> + Clone + Send,
    F::Output: Future<Output = Result<O, Rejection<R, T::Error>>> + Send,
    R: Send,
{
    #[pin]
    state: State<T, F, R, O>,
}

#[pin_project(project = StateProj)]
enum State<T, F, R, O>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract> + Clone + Send,
    F::Output: Future<Output = Result<O, Rejection<R, T::Error>>> + Send,
    R: Send,
{
    First(#[pin] T::Future, F),
    Second(#[pin] F::Output, Option<R>),
    Done,
}

impl<T, F, R, O> Future for AndThenRejectFuture<T, F, R, O>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract> + Clone + Send,
    F::Output: Future<Output = Result<O, Rejection<R, T::Error>>> + Send,
    R: Send,
{
    #[allow(clippy::type_complexity)]
    type Output = Result<(R, (<F::Output as TryFuture>::Ok,)), Rejection<R, T::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            let ((req, ex1), second) = match pin.state.project() {
                StateProj::First(first, second) => match ready!(first.try_poll(cx)) {
                    Ok(first) => {
                        let first = first.unpack();
                        (first, second)
                    }
                    Err(err) => return Poll::Ready(Err(err)),
                },
                StateProj::Second(second, req) => {
                    let ex3 = match ready!(second.try_poll(cx)) {
                        Ok(item) => Ok((req.take().unwrap(), (item,))),
                        Err(err) => Err(err),
                    };
                    self.set(AndThenRejectFuture { state: State::Done });

                    return Poll::Ready(ex3);
                }
                StateProj::Done => panic!("polled after complete"),
            };
            let fut2 = second.call(ex1);
            self.set(AndThenRejectFuture {
                state: State::Second(fut2, Some(req)),
            });
        }
    }
}
