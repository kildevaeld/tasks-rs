use super::error::Error;
use async_trait::async_trait;
use futures_io::{AsyncRead, AsyncWrite};
use vfs_async::VFS;

pub trait Target: VFS {}

impl<T> Target for T where T: VFS {}
