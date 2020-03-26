#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]

//! A set of middleware for sharing data between requests in the Iron
//! framework.

use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use plugin::{Plugin, SyncExtensible};
use typemap::Key;
use crate::{Middleware, Next, NextFuture};
use std::marker::PhantomData;

/// The type that can be returned by `eval` to indicate error.
#[derive(Clone, Debug)]
pub enum PersistentError {
    /// The value was not found.
    NotFound,
}

impl Error for PersistentError {
   
}

impl fmt::Display for PersistentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            PersistentError::NotFound => f.write_str("Value not found in extensions."),
        }
    }
}

impl From<PersistentError> for ValseError {
    fn from(error: PersistentError) -> ValseError {
        ValseError::new(error, StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Helper trait for overloading the constructors of `Read`/`Write`/`State`.
/// This is an implementation detail, and should not be used for any other
/// purpose.
///
/// For example, this trait lets you construct a `Read<T>` from either a `T` or
/// an `Arc<T>`.
pub trait PersistentInto<T> {
    /// Convert `self` into a value of type `T`.
    fn persistent_into(self) -> T;
}

impl<T> PersistentInto<T> for T {
    fn persistent_into(self) -> T {
        self
    }
}

impl<T> PersistentInto<Arc<T>> for T {
    fn persistent_into(self) -> Arc<T> {
        Arc::new(self)
    }
}

impl<T> PersistentInto<Arc<Mutex<T>>> for T {
    fn persistent_into(self) -> Arc<Mutex<T>> {
        Arc::new(Mutex::new(self))
    }
}

impl<T> PersistentInto<Arc<RwLock<T>>> for T {
    fn persistent_into(self) -> Arc<RwLock<T>> {
        Arc::new(RwLock::new(self))
    }
}

/// Middleware for data that persists between requests with read and write capabilities.
///
/// The data is stored behind a `RwLock`, so multiple read locks
/// can be taken out concurrently.
///
/// If most threads need to take out a write lock, you may want to
/// consider `Write`, which stores the data behind a `Mutex`, which
/// has a faster locking speed.
///
/// `State` can be linked as `BeforeMiddleware` to add data to the `Request`
/// extensions and it can be linked as an `AfterMiddleware` to add data to
/// the `Response` extensions.
///
/// `State` also implements `Plugin`, so the data stored within can be
/// accessed through `request.get::<State<P>>()` as an `Arc<RwLock<P::Value>>`.
pub struct State<P: Key, INPUT> {
    data: Arc<RwLock<P::Value>>,
    _i: PhantomData<INPUT>
}

/// Middleware for data that persists between Requests with read-only capabilities.
///
/// The data is stored behind an Arc, so multiple threads can have
/// concurrent, non-blocking access.
///
/// `Read` can be linked as `BeforeMiddleware` to add data to the `Request`
/// extensions and it can be linked as an `AfterMiddleware` to add data to
/// the `Response` extensions.
///
/// `Read` also implements `Plugin`, so the data stored within can be
/// accessed through `request.get::<Read<P>>()` as an `Arc<P::Value>`.
pub struct Read<P: Key, INPUT> {
    data: Arc<P::Value>,
    _i: PhantomData<INPUT>
}

/// Middleware for data that persists between Requests for data which mostly
/// needs to be written instead of read.
///
/// The data is stored behind a `Mutex`, so only one request at a time can
/// access the data. This is more performant than `State` in the case where
/// most uses of the data require a write lock.
///
/// `Write` can be linked as `BeforeMiddleware` to add data to the `Request`
/// extensions and it can be linked as an `AfterMiddleware` to add data to
/// the `Response` extensions.
///
/// `Write` also implements `Plugin`, so the data stored within can be
/// accessed through `request.get::<Write<P>>()` as an `Arc<Mutex<P::Value>>`.
pub struct Write<P: Key> {
    data: Arc<Mutex<P::Value>>,
}

impl<P: Key, INPUT> Clone for Read<P, INPUT>
where
    P::Value: Send + Sync,
{
    fn clone(&self) -> Read<P, INPUT> {
        Read {
            data: self.data.clone(),
            _i: PhantomData
        }
    }
}

impl<P: Key, INPUT> Clone for State<P, INPUT>
where
    P::Value: Send + Sync,
{
    fn clone(&self) -> State<P, INPUT> {
        State {
            data: self.data.clone(),
            _i: PhantomData
        }
    }
}

impl<P: Key> Clone for Write<P>
where
    P::Value: Send + Sync,
{
    fn clone(&self) -> Write<P> {
        Write {
            data: self.data.clone(),
        }
    }
}

impl<P: Key,INPUT: 'static> Key for State<P,INPUT>
where
    P::Value: 'static,
{
    type Value = Arc<RwLock<P::Value>>;
}

impl<P: Key, INPUT: 'static> Key for Read<P, INPUT>
where
    P::Value: 'static,
{
    type Value = Arc<P::Value>;
}

impl<P: Key> Key for Write<P>
where
    P::Value: 'static,
{
    type Value = Arc<Mutex<P::Value>>;
}

impl<P: Key, INPUT: SyncExtensible + 'static> Plugin<INPUT> for State<P, INPUT>
where
    P::Value: Send + Sync,
{
    type Error = PersistentError;
    fn eval(req: &mut INPUT) -> Result<Arc<RwLock<P::Value>>, PersistentError> {
        req.extensions()
            .get::<State<P, INPUT>>()
            .cloned()
            .ok_or(PersistentError::NotFound)
    }
}

impl<P: Key, INPUT: SyncExtensible + 'static> Plugin<INPUT> for Read<P, INPUT>
where
    P::Value: Send + Sync,
{
    type Error = PersistentError;
    fn eval(req: &mut INPUT) -> Result<Arc<P::Value>, PersistentError> {
        req.extensions()
            .get::<Read<P, INPUT>>()
            .cloned()
            .ok_or(PersistentError::NotFound)
    }
}

impl<P: Key, INPUT: SyncExtensible> Plugin<INPUT> for Write<P>
where
    P::Value: Send + Sync,
{
    type Error = PersistentError;
    fn eval(req: &mut INPUT) -> Result<Arc<Mutex<P::Value>>, PersistentError> {
        req.extensions()
            .get::<Write<P>>()
            .cloned()
            .ok_or(PersistentError::NotFound)
    }
}

impl<P: Key, INPUT: SyncExtensible + 'static> Middleware<INPUT> for State<P, INPUT>
where
    P::Value: Send + Sync,
{
    type Output = Response;
    type Error = ValseError;
    type Future = NextFuture<Self::Output, Self::Error>;
    fn execute(&self, mut req: INPUT, next: Next<INPUT, Self::Output, Self::Error>) -> Self::Future {
        req.extensions_mut().insert::<State<P, INPUT>>(self.data.clone());
        next.exec(req)
    }
}

impl<P: Key, INPUT: SyncExtensible + 'static> Middleware<INPUT> for Read<P, INPUT>
where
    P::Value: Send + Sync,
{
    type Output = Response;
    type Error = ValseError;
    type Future = NextFuture<Self::Output, Self::Error>;
    fn execute(&self, mut req: INPUT, next: Next<INPUT, Self::Output, Self::Error>) -> Self::Future {
        req.extensions_mut().insert::<Read<P, INPUT>>(self.data.clone());
        next.exec(req)
    }
}

impl<P: Key, INPUT: SyncExtensible> Middleware<INPUT> for Write<P>
where
    P::Value: Send + Sync,
{
    type Output = Response;
    type Error = ValseError;
    type Future = NextFuture<Self::Output, Self::Error>;
    fn execute(&self, mut req:INPUT, next: Next<INPUT, Self::Output, Self::Error>) -> Self::Future {
        req.extensions_mut().insert::<Write<P>>(self.data.clone());
        next.exec(req)
    }
}

impl<P: Key,INPUT> State<P,INPUT>
where
    P::Value: Send + Sync,
{
    /// Construct a new `State` middleware
    ///
    /// The data is initialized with the passed-in value.
    pub fn middleware<T>(start: T) -> State<P ,INPUT>
    where
        T: PersistentInto<Arc<RwLock<P::Value>>>,
    {
        State {
            data: start.persistent_into(),
            _i: PhantomData
        }
    }
}

impl<P: Key, INPUT> Read<P, INPUT>
where
    P::Value: Send + Sync,
{
    /// Construct a new `Read` middleware
    ///
    /// The data is initialized with the passed-in value.
    pub fn middleware<T>(start: T) -> Read<P, INPUT>
    where
        T: PersistentInto<Arc<P::Value>>,
    {
        Read {
            data: start.persistent_into(),
            _i: PhantomData
        }
    }
}

impl<P: Key> Write<P>
where
    P::Value: Send + Sync,
{
    /// Construct a new `Write` middleware
    ///
    /// The data is initialized with the passed-in value.
    pub fn middleware<T>(start: T) -> Write<P>
    where
        T: PersistentInto<Arc<Mutex<P::Value>>>,
    {
        Write {
            data: start.persistent_into(),
        }
    }
}
