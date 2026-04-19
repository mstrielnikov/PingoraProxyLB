use async_trait::async_trait;
use crate::cache::traits::CacheTrait;
use moka::future::Cache as MokaCache;
use std::sync::Arc;

#[derive(Clone)]
pub struct MokaCacheImpl {
    pub inner: Arc<MokaCache<String, String>>,
}

#[async_trait]
impl CacheTrait for MokaCacheImpl {
    async fn get(&self, key: &str) -> Option<String> {
        self.inner.get(key).await
    }
    async fn insert(&self, key: String, value: String) {
        self.inner.insert(key, value).await;
    }
    async fn evict(&self, key: &str) {
        self.inner.invalidate(key).await;
    }
}
