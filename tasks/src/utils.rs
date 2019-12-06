use std::pin::Pin;
use std::task::{Poll, Context};
use std::future::Future;
use pin_project::pin_project;


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

pub enum Promise3<T1, T2, T3> {
    First(T1),
    Second(T2),
    Third(T3),
}

pub struct OneOf3Future<T1: Future<Output = V>, T2: Future<Output = V>, T3: Future<Output = V>, V> {
    inner: Promise3<T1, T2, T3>,
}

impl<T1: Future<Output = V>, T2: Future<Output = V>, T3: Future<Output = V>, V>
    OneOf3Future<T1, T2, T3, V>
{
    pub fn new(future: Promise3<T1, T2, T3>) -> OneOf3Future<T1, T2, T3, V> {
        OneOf3Future { inner: future }
    }
}

impl<T1: Future<Output = V>, T2: Future<Output = V>, T3: Future<Output = V>, V> Future
    for OneOf3Future<T1, T2, T3, V>
{
    type Output = V;
    fn poll(self: Pin<&mut Self>, waker: &mut Context) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        match &mut this.inner {
            Promise3::First(fut) => unsafe { Pin::new_unchecked(fut) }.poll(waker),
            Promise3::Second(fut) => unsafe { Pin::new_unchecked(fut) }.poll(waker),
            Promise3::Third(fut) => unsafe { Pin::new_unchecked(fut) }.poll(waker),
        }
    }
}

pub enum Promise4<T1, T2, T3, T4> {
    First(T1),
    Second(T2),
    Third(T3),
    Fourth(T4),
}

pub struct OneOf4Future<
    T1: Future<Output = V>,
    T2: Future<Output = V>,
    T3: Future<Output = V>,
    T4: Future<Output = V>,
    V,
> {
    inner: Promise4<T1, T2, T3, T4>,
}

impl<
        T1: Future<Output = V>,
        T2: Future<Output = V>,
        T3: Future<Output = V>,
        T4: Future<Output = V>,
        V,
    > OneOf4Future<T1, T2, T3, T4, V>
{
    pub fn new(future: Promise4<T1, T2, T3, T4>) -> OneOf4Future<T1, T2, T3, T4, V> {
        OneOf4Future { inner: future }
    }
}

impl<
        T1: Future<Output = V>,
        T2: Future<Output = V>,
        T3: Future<Output = V>,
        T4: Future<Output = V>,
        V,
    > Future for OneOf4Future<T1, T2, T3, T4, V>
{
    type Output = V;
    fn poll(self: Pin<&mut Self>, waker: &mut Context) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        match &mut this.inner {
            Promise4::First(fut) => unsafe { Pin::new_unchecked(fut) }.poll(waker),
            Promise4::Second(fut) => unsafe { Pin::new_unchecked(fut) }.poll(waker),
            Promise4::Third(fut) => unsafe { Pin::new_unchecked(fut) }.poll(waker),
            Promise4::Fourth(fut) => unsafe { Pin::new_unchecked(fut) }.poll(waker),
        }
    }
}

