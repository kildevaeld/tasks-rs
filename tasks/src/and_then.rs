use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Extract, Func, Rejection, Task};
use futures_core::{ready, TryFuture};
use pin_project::pin_project;

#[derive(Clone, Copy, Debug)]
pub struct AndThen<T, F> {
    pub(super) filter: T,
    pub(super) callback: F,
}

impl<T, F, R> Task<R> for AndThen<T, F>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract> + Clone + Send,
    F::Output: TryFuture + Send,
    <F::Output as TryFuture>::Error: Into<T::Error>,
    R: Send,
{
    type Output = (R, (<F::Output as TryFuture>::Ok,));
    type Error = T::Error;
    type Future = AndThenFuture<T, F, R>;
    #[inline]
    fn run(&self, req: R) -> Self::Future {
        AndThenFuture {
            state: State::First(self.filter.run(req), self.callback.clone()),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct AndThenFuture<T, F, R>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract>,
    F::Output: TryFuture + Send,
    //<F::Output as TryFuture>::Error: Into<T::Error>,
{
    #[pin]
    state: State<T, F, R>,
}

#[pin_project(project = StateProj)]
enum State<T, F, R>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract>,
    F::Output: TryFuture + Send,
{
    First(#[pin] T::Future, F),
    Second(#[pin] F::Output, Option<R>),
    Done,
}

impl<T, F, R> Future for AndThenFuture<T, F, R>
where
    T: Task<R>,
    T::Output: Extract<R>,
    F: Func<<T::Output as Extract<R>>::Extract>,
    F::Output: TryFuture + Send,
    <F::Output as TryFuture>::Error: Into<T::Error>,
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
                        Err(err) => Err(Rejection::Err(err.into())),
                    };
                    self.set(AndThenFuture { state: State::Done });

                    return Poll::Ready(ex3);
                }
                StateProj::Done => panic!("polled after complete"),
            };
            let fut2 = second.call(ex1);
            self.set(AndThenFuture {
                state: State::Second(fut2, Some(req)),
            });
        }
    }
}
