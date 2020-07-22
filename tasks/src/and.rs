use super::Extract;
use crate::{Combine, HList, Rejection, Task, Tuple};
use futures_core::ready;
use pin_project::{pin_project, project};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone, Debug)]
pub struct And<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<'a, T, U, R> Task<R> for And<T, U>
where
    R: Send + Sync,
    T: Task<R>,
    T::Output: Extract<R> + Send,
    U: Task<R, Error = T::Error> + Clone + Send,
    U::Output: Extract<R>,
    <<T::Output as Extract<R>>::Extract as Tuple>::HList:
        Combine<<<U::Output as Extract<R>>::Extract as Tuple>::HList> + Send,
    <<<<T::Output as Extract<R>>::Extract as Tuple>::HList as Combine<
        <<U::Output as Extract<R>>::Extract as Tuple>::HList,
    >>::Output as HList>::Tuple: Send,
{
    type Output = (
        R,
        <<<<T::Output as Extract<R>>::Extract as Tuple>::HList as Combine<
            <<U::Output as Extract<R>>::Extract as Tuple>::HList,
        >>::Output as HList>::Tuple,
    );
    type Error = U::Error;
    type Future = AndFuture<R, T, U>;

    fn run(&self, req: R) -> Self::Future {
        AndFuture {
            state: State::First(self.first.run(req), self.second.clone()),
        }
    }
}

impl<T, U> Copy for And<T, U>
where
    T: Copy,
    U: Copy,
{
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct AndFuture<R, T: Task<R>, U: Task<R>>
where
    T::Output: Extract<R>,
    U::Output: Extract<R>,
{
    #[pin]
    state: State<R, T, U>,
}

#[pin_project]
enum State<R, T: Task<R>, U: Task<R>>
where
    T::Output: Extract<R>,
    U::Output: Extract<R>,
{
    First(#[pin] T::Future, U),
    Second(Option<<T::Output as Extract<R>>::Extract>, #[pin] U::Future),
    Done,
}

impl<R, T, U> Future for AndFuture<R, T, U>
where
    T: Task<R>,
    T::Output: Extract<R>,
    U: Task<R, Error = T::Error>,
    U::Output: Extract<R>,
    <<T::Output as Extract<R>>::Extract as Tuple>::HList:
        Combine<<<U::Output as Extract<R>>::Extract as Tuple>::HList> + Send,
{
    type Output = Result<
        (
            R,
            <<<<T::Output as Extract<R>>::Extract as Tuple>::HList as Combine<
                <<U::Output as Extract<R>>::Extract as Tuple>::HList,
            >>::Output as HList>::Tuple,
        ),
        Rejection<R, U::Error>,
    >;

    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            #[project]
            let (ex1, fut2) = match pin.state.project() {
                State::First(first, second) => match ready!(first.poll(cx)) {
                    Ok(ret) => {
                        let (req, first) = ret.unpack();
                        (first, second.run(req))
                    }
                    Err(err) => return Poll::Ready(Err(err)),
                },
                State::Second(ex1, second) => {
                    let (req, ex2) = match ready!(second.poll(cx)) {
                        Ok(second) => second.unpack(),
                        Err(err) => return Poll::Ready(Err(From::from(err))),
                    };
                    let ex3 = ex1.take().unwrap().hlist().combine(ex2.hlist()).flatten();
                    self.set(AndFuture { state: State::Done });
                    return Poll::Ready(Ok((req, ex3)));
                }
                State::Done => panic!("polled after complete"),
            };

            self.set(AndFuture {
                state: State::Second(Some(ex1), fut2),
            });
        }
    }
}
