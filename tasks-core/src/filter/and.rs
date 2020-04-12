use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::ready;
use pin_project::{pin_project, project};

use super::{Combine, Filter, HList, Tuple};
// use crate::reject::CombineRejection;

#[derive(Clone, Copy, Debug)]
pub struct And<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<'a, T, U, R> Filter<R> for And<T, U>
where
    R: Send + Sync,
    T: Filter<R>,
    T::Extract: Send,
    U: Filter<R, Error = T::Error> + Clone + Send,
    <T::Extract as Tuple>::HList: Combine<<U::Extract as Tuple>::HList> + Send,
    <<<T::Extract as Tuple>::HList as Combine<<U::Extract as Tuple>::HList>>::Output as HList>::Tuple: Send,
    //U::Error: CombineRejection<T::Error>,
{
    type Extract = <<<T::Extract as Tuple>::HList as Combine<<U::Extract as Tuple>::HList>>::Output as HList>::Tuple;
    type Error = U::Error; //<U::Error as CombineRejection<T::Error>>::One;
    type Future = AndFuture<R, T, U>;

    fn filter(&self, req: R) -> Self::Future {
        AndFuture {
            state: State::First(self.first.filter(req), self.second.clone()),
        }
    }
}

#[allow(missing_debug_implementations)]
#[pin_project]
pub struct AndFuture<R, T: Filter<R>, U: Filter<R>> {
    #[pin]
    state: State<R, T, U>,
}

#[pin_project]
enum State<R, T: Filter<R>, U: Filter<R>> {
    First(#[pin] T::Future, U),
    Second(Option<T::Extract>, #[pin] U::Future),
    Done,
}

impl<R, T, U> Future for AndFuture<R, T, U>
where
    T: Filter<R>,
    U: Filter<R, Error = T::Error>,
    <T::Extract as Tuple>::HList: Combine<<U::Extract as Tuple>::HList> + Send,
    //U::Error: CombineRejection<T::Error>,
{
    type Output = Result<
            (R, <<<T::Extract as Tuple>::HList as Combine<<U::Extract as Tuple>::HList>>::Output as HList>::Tuple), U::Error>;
    // <U::Error as CombineRejection<T::Error>>::One>;

    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            #[project]
            let (ex1, fut2) = match pin.state.project() {
                State::First(first, second) => match ready!(first.poll(cx)) {
                    Ok((req, first)) => (first, second.filter(req)),
                    Err(err) => return Poll::Ready(Err(err)),
                },
                State::Second(ex1, second) => {
                    let (req, ex2) = match ready!(second.poll(cx)) {
                        Ok(second) => second,
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
