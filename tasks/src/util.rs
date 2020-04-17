use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[pin_project]
pub enum Promise<T1, T2> {
    First(#[pin] T1),
    Second(#[pin] T2),
}

#[pin_project]
pub struct OneOf2Future<T1: Future<Output = V>, T2: Future<Output = V>, V> {
    #[pin]
    inner: Promise<T1, T2>,
}

impl<T1: Future<Output = V>, T2: Future<Output = V>, V> OneOf2Future<T1, T2, V> {
    pub fn new(future: Promise<T1, T2>) -> OneOf2Future<T1, T2, V> {
        OneOf2Future { inner: future }
    }
}

impl<T1: Future<Output = V>, T2: Future<Output = V>, V> Future for OneOf2Future<T1, T2, V> {
    type Output = V;
    fn poll(self: Pin<&mut Self>, waker: &mut Context) -> Poll<Self::Output> {
        let inner = self.project().inner.project();
        match inner {
            __PromiseProjection::First(fut) => fut.poll(waker),
            __PromiseProjection::Second(fut) => fut.poll(waker),
        }
    }
}
