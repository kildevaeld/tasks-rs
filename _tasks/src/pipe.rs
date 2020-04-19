use super::task::{Task};
use std::sync::Arc;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use super::utils::Promise;


pub struct Pipe<S1, S2> {
    pub(crate) s1: S1,
    pub(crate) s2: Arc<S2>,
}

impl<S1, S2, I> Task<I> for Pipe<S1, S2>
where
    S1: Task<I>,
    S2: Task<<S1 as Task<I>>::Output, Error = <S1 as Task<I>>::Error>
        + 'static
        + Send
        + Sync,
    <S1 as Task<I>>::Output: 'static,
    <S1 as Task<I>>::Error: Send + 'static,
    <S2 as Task<<S1 as Task<I>>::Output>>::Future: Send + 'static,
{
    //type Input = S1::Input;
    type Error = S1::Error;
    type Output = S2::Output;
    type Future = PipeFuture<<S1 as Task<I>>::Future, S2, S1::Output, Self::Error>;
    
    fn exec(&self, input: I) -> Self::Future {
        PipeFuture::new(self.s1.exec(input), &self.s2)
    }

    fn can_exec(&self, input: &I) -> bool {
        self.s1.can_exec(input)
    }

}

pub struct PipeFuture<F: Future<Output = Result<V, E>>, T: Task<V, Error = E>, V, E> {
    current: Promise<F, <T as Task<V>>::Future>,
    task: Arc<T>,
}

impl<F: Future<Output = Result<V, E>>, T: Task<V, Error = E>, V, E> PipeFuture<F, T, V, E> {
    pub fn new(current: F, next: &Arc<T>) -> PipeFuture<F, T, V, E> {
        PipeFuture { current: Promise::First(current), task: next.clone() }
    }
}

impl<F: Future<Output = Result<V, E>>, T: Task<V, Error = E>, V, E> Future for PipeFuture<F, T, V, E> {
    type Output = Result<<T as Task<V>>::Output, <T as Task<V>>::Error>;

    fn poll(self: Pin<&mut Self>, waker: &mut Context) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        match &mut this.current {
            Promise::First(fut) => {
                //
                match unsafe { Pin::new_unchecked(fut) }.poll(waker) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Ok(next)) => {
                        let mut fut = this.task.exec(next);
                        let poll = unsafe { Pin::new_unchecked(&mut fut)}.poll(waker);
                        this.current = Promise::Second(fut);
                        poll
                    },
                    Poll::Ready(Err(err)) => Poll::Ready(Err(err))
                }
            },
            Promise::Second(fut) => unsafe { Pin::new_unchecked(fut) }.poll(waker),
        }
    }
}