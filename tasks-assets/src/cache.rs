use async_trait::async_trait;
use bytes::Bytes;

pub struct CacheSetOptions {}

#[async_trait]
pub trait Cache<Key>: Sync + Send {
    async fn set(&self, key: Key, value: Bytes, options: CacheSetOptions);
    async fn get(&self, key: Key) -> Option<Bytes>;
    async fn rm(&self, key: Key);
}

#[derive(Clone, Copy)]
pub struct NullCache;

#[async_trait]
impl<Key> Cache<Key> for NullCache
where
    Key: Send + 'static,
{
    async fn set(&self, _key: Key, _value: Bytes, _options: CacheSetOptions) {}
    async fn get(&self, _key: Key) -> Option<Bytes> {
        None
    }
    async fn rm(&self, _key: Key) {}
}

pub fn null() -> impl Cache<String> {
    NullCache
}
