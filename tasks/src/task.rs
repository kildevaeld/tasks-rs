use super::generic::Either;
use super::Rejection;
use futures_core::ready;
use pin_project::pin_project;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait Task<R> {
    type Output;
    type Error;
    type Future: Future<Output = Result<Self::Output, Rejection<R, Self::Error>>> + Send;

    fn run(&self, req: R) -> Self::Future;
}

pub struct TaskFn<F, I, O, E> {
    f: F,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
    _e: PhantomData<E>,
}

impl<F, I, O, E> Clone for TaskFn<F, I, O, E>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        TaskFn {
            f: self.f.clone(),
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E> Copy for TaskFn<F, I, O, E> where F: Copy {}

unsafe impl<F, I, O, E> Sync for TaskFn<F, I, O, E> where F: Sync {}

unsafe impl<F, I, O, E> Send for TaskFn<F, I, O, E> where F: Send {}

impl<F, I, O, E, U> TaskFn<F, I, O, E>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send + 'static,
{
    pub fn new(task: F) -> TaskFn<F, I, O, E> {
        TaskFn {
            f: task,
            _i: PhantomData,
            _o: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<F, I, O, E, U> Task<I> for TaskFn<F, I, O, E>
where
    F: Fn(I) -> U,
    U: Future<Output = Result<O, Rejection<I, E>>> + Send,
{
    type Output = O;
    type Error = E;
    type Future = U;
    fn run(&self, input: I) -> Self::Future {
        (self.f)(input)
    }
}

#[derive(Clone, Copy)]
pub struct Reject<T>(T);

impl<T> Reject<T> {
    pub fn new(task: T) -> Reject<T> {
        Reject(task)
    }
}

impl<T, R> Task<R> for Reject<T>
where
    T: Task<R>,
{
    type Output = T::Output;
    type Error = T::Error;
    type Future = RejectFuture<T, R>;
    fn run(&self, req: R) -> Self::Future {
        RejectFuture(self.0.run(req))
    }
}

#[pin_project]
pub struct RejectFuture<T: Task<R>, R>(#[pin] T::Future);

impl<T, R> Future for RejectFuture<T, R>
where
    T: Task<R>,
{
    type Output = Result<T::Output, Rejection<R, T::Error>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        match ready!(this.0.poll(cx)) {
            Ok(s) => Poll::Ready(Ok(s)),
            Err(Rejection::Err(e)) => Poll::Ready(Err(Rejection::Err(e))),
            Err(Rejection::Reject(_, Some(e))) => Poll::Ready(Err(Rejection::Err(e))),
            Err(Rejection::Reject(r, None)) => Poll::Ready(Err(Rejection::Reject(r, None))),
        }
    }
}

impl<A, B, R> Task<R> for Either<A, B>
where
    A: Task<R>,
    B: Task<R, Error = <A as Task<R>>::Error>,
    R: Send,
{
    type Output = Either<A::Output, B::Output>;
    type Error = A::Error;
    type Future = EitherFuture<A, B, R>;

    fn run(&self, req: R) -> Self::Future {
        match self {
            Either::A(a) => EitherFuture {
                fut: EitherPromise::First(a.run(req)),
                _r: std::marker::PhantomData,
            },
            Either::B(b) => EitherFuture {
                fut: EitherPromise::Second(b.run(req)),
                _r: std::marker::PhantomData,
            },
        }
    }
}

#[pin_project(project = EitherPromiseProj)]
enum EitherPromise<A, B> {
    First(#[pin] A),
    Second(#[pin] B),
}

#[pin_project]
pub struct EitherFuture<A, B, R>
where
    A: Task<R>,
    B: Task<R, Error = <A as Task<R>>::Error>,
{
    #[pin]
    fut: EitherPromise<A::Future, B::Future>,
    _r: std::marker::PhantomData<R>,
}

impl<A, B, R> EitherFuture<A, B, R>
where
    A: Task<R>,
    B: Task<R, Error = <A as Task<R>>::Error>,
{
    pub fn a(fut: A::Future) -> EitherFuture<A, B, R> {
        EitherFuture {
            fut: EitherPromise::First(fut),
            _r: std::marker::PhantomData,
        }
    }

    pub fn b(fut: B::Future) -> EitherFuture<A, B, R> {
        EitherFuture {
            fut: EitherPromise::Second(fut),
            _r: std::marker::PhantomData,
        }
    }
}

impl<A, B, R> Future for EitherFuture<A, B, R>
where
    A: Task<R>,
    B: Task<R, Error = <A as Task<R>>::Error>,
{
    #[allow(clippy::type_complexity)]
    type Output = Result<Either<A::Output, B::Output>, Rejection<R, A::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        match this.fut.project() {
            EitherPromiseProj::First(fut) => match ready!(fut.poll(cx)) {
                Ok(o) => Poll::Ready(Ok(Either::A(o))),
                Err(e) => Poll::Ready(Err(e)),
            },
            EitherPromiseProj::Second(fut) => match ready!(fut.poll(cx)) {
                Ok(o) => Poll::Ready(Ok(Either::B(o))),
                Err(e) => Poll::Ready(Err(e)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(threaded_scheduler)]
    async fn test_task() {
        struct Error {}

        let task = TaskFn::new(|req: ()| async move {
            //
            Result::<(), _>::Err(Error {}.into())
        });

        task.run(()).await;
    }
}
