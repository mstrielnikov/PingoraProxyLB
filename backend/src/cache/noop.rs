use async_trait::async_trait;
use crate::cache::traits::CacheTrait;

#[derive(Clone)]
pub struct NoOpCache;

#[async_trait]
impl CacheTrait for NoOpCache {
    async fn get(&self, _key: &str) -> Option<String> {
        None
    }
    async fn insert(&self, _key: String, _value: String) {}
    async fn evict(&self, _key: &str) {}
}
