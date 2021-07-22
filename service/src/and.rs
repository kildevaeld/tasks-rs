use super::Extract;
use crate::{
    generic::{Combine, HList, Tuple},
    Rejection, Service,
};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::ready;
use pin_project::pin_project;

#[derive(Clone, Debug)]
pub struct And<T, U> {
    pub(super) first: T,
    pub(super) second: U,
}

impl<'a, T, U, R> Service<R> for And<T, U>
where
    // R: Send,
    T: Service<R>,
    T::Output: Extract<R> + Send,
    U: Service<R, Error = T::Error> + Clone + Send,
    U::Output: Extract<R>,
    <<T::Output as Extract<R>>::Extract as Tuple>::HList:
        Combine<<<U::Output as Extract<R>>::Extract as Tuple>::HList> + Send,
    <<<<T::Output as Extract<R>>::Extract as Tuple>::HList as Combine<
        <<U::Output as Extract<R>>::Extract as Tuple>::HList,
    >>::Output as HList>::Tuple: Send,
{
    #[allow(clippy::type_complexity)]
    type Output = (
        R,
        <<<<T::Output as Extract<R>>::Extract as Tuple>::HList as Combine<
            <<U::Output as Extract<R>>::Extract as Tuple>::HList,
        >>::Output as HList>::Tuple,
    );
    type Error = U::Error;
    type Future = AndFuture<R, T, U>;

    fn call(&self, req: R) -> Self::Future {
        AndFuture {
            state: State::First(self.first.call(req), self.second.clone()),
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
pub struct AndFuture<R, T: Service<R>, U: Service<R>>
where
    T::Output: Extract<R>,
    U::Output: Extract<R>,
{
    #[pin]
    state: State<R, T, U>,
}

#[pin_project(project = StateProj)]
enum State<R, T: Service<R>, U: Service<R>>
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
    T: Service<R>,
    T::Output: Extract<R>,
    U: Service<R, Error = T::Error>,
    U::Output: Extract<R>,
    <<T::Output as Extract<R>>::Extract as Tuple>::HList:
        Combine<<<U::Output as Extract<R>>::Extract as Tuple>::HList> + Send,
{
    #[allow(clippy::type_complexity)]
    type Output = Result<
        (
            R,
            <<<<T::Output as Extract<R>>::Extract as Tuple>::HList as Combine<
                <<U::Output as Extract<R>>::Extract as Tuple>::HList,
            >>::Output as HList>::Tuple,
        ),
        Rejection<R, U::Error>,
    >;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let pin = self.as_mut().project();
            let (ex1, fut2) = match pin.state.project() {
                StateProj::First(first, second) => match ready!(first.poll(cx)) {
                    Ok(ret) => {
                        let (req, first) = ret.unpack();
                        (first, second.call(req))
                    }
                    Err(err) => return Poll::Ready(Err(err)),
                },
                StateProj::Second(ex1, second) => {
                    let (req, ex2) = match ready!(second.poll(cx)) {
                        Ok(second) => second.unpack(),
                        Err(err) => return Poll::Ready(Err(err)),
                    };
                    let ex3 = ex1.take().unwrap().hlist().combine(ex2.hlist()).flatten();
                    self.set(AndFuture { state: State::Done });
                    return Poll::Ready(Ok((req, ex3)));
                }
                StateProj::Done => panic!("polled after complete"),
            };

            self.set(AndFuture {
                state: State::Second(Some(ex1), fut2),
            });
        }
    }
}
