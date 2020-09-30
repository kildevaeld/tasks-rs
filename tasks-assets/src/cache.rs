use crate::Error;
use async_mutex::Mutex;
use bytes::{Bytes, BytesMut};
use futures_io::AsyncWrite;
use futures_util::{
    future::{BoxFuture, FutureExt},
    stream::StreamExt,
    AsyncWriteExt,
};
use mime::Mime;
use sha2::{digest::generic_array::GenericArray, Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tasks_vinyl::{Content, File, Path};

#[derive(Debug, Clone, PartialEq, Hash, PartialOrd, Ord, Eq)]
pub struct CacheKey(pub(crate) Vec<u8>);

impl std::ops::Deref for CacheKey {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct CacheSetOptions {}

pub trait Cache: Sync + Send {
    fn set<'a>(
        &'a self,
        key: &'a CacheKey,
        value: &'a mut File,
        options: CacheSetOptions,
    ) -> BoxFuture<'a, Result<(), Error>>;
    fn get<'a>(&self, key: &'a CacheKey) -> BoxFuture<'a, Option<File>>;
    fn rm(&self, key: CacheKey) -> BoxFuture<'static, ()>;
}

#[derive(Clone, Copy)]
pub struct NullCache;

impl Cache for NullCache {
    fn set<'a>(
        &'a self,
        _key: &'a CacheKey,
        value: &'a mut File,
        _options: CacheSetOptions,
    ) -> BoxFuture<'a, Result<(), Error>> {
        {
            async move { Ok(()) }.boxed()
        }
    }
    fn get<'a>(&self, key: &'a CacheKey) -> BoxFuture<'a, Option<File>> {
        async { None }.boxed()
    }
    fn rm(&self, _key: CacheKey) -> BoxFuture<'static, ()> {
        async {}.boxed()
    }
}

pub fn null() -> NullCache {
    NullCache
}

pub struct MemoryCache {
    inner: Arc<Mutex<HashMap<CacheKey, (Path, Mime, Bytes)>>>,
}

impl MemoryCache {
    pub fn new() -> MemoryCache {
        MemoryCache {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Cache for MemoryCache {
    fn set<'a>(
        &self,
        key: &'a CacheKey,
        file: &'a mut File,
        options: CacheSetOptions,
    ) -> BoxFuture<'a, Result<(), Error>> {
        let inner = self.inner.clone();
        async move {
            let mime = file.mime.clone();
            // let path = file.path.clone();

            let content = std::mem::replace(&mut file.content, Content::None);
            let mut stream = content.into_stream().await?;

            let mut dest = Vec::new();
            while let Some(next) = stream.next().await {
                let next = next?;
                dest.write(&next).await?;
            }

            let data = Bytes::from(dest);

            let value = (file.path.clone(), file.mime.clone(), data.clone());

            let mut cache = inner.lock().await;
            file.content = Content::Bytes(data);

            cache.insert(key.clone(), value);

            Ok(())
        }
        .boxed()
    }

    fn get<'a>(&self, key: &'a CacheKey) -> BoxFuture<'a, Option<File>> {
        let inner = self.inner.clone();
        async move {
            let cache = inner.lock().await;
            let value = match cache.get(&key) {
                Some(s) => s,
                None => return None,
            };

            Some(File::new(
                value.0.clone(),
                value.2.clone(),
                value.1.clone(),
                value.2.len() as u64,
            ))
        }
        .boxed()
    }

    fn rm(&self, key: CacheKey) -> BoxFuture<'static, ()> {
        let inner = self.inner.clone();
        async move {
            let mut cache = inner.lock().await;
            cache.remove(&key);
        }
        .boxed()
    }
}
