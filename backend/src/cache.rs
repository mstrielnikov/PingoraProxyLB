use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use moka::future::Cache as MokaCache;
use crate::config::{AppConfig, CacheStrategy};

#[async_trait]
pub trait CacheTrait {
    async fn get(&self, key: &str) -> Option<String>;
    async fn insert(&self, key: String, value: String);
    async fn evict(&self, key: &str);
}

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

pub fn create_cache(config: &AppConfig) -> Box<dyn CacheTrait + Send + Sync + 'static> {
    if config.cache.enabled {
        let ttl = Duration::from_secs(config.cache.ttl_secs);
        let mut builder = MokaCache::builder()
            .time_to_live(ttl)
            .max_capacity(config.cache.max_capacity);
        match config.cache.strategy {
            CacheStrategy::LRU => builder = builder.eviction_policy(moka::policy::EvictionPolicy::lru()),
            CacheStrategy::LFU => builder = builder.eviction_policy(moka::policy::EvictionPolicy::tiny_lfu()),
            CacheStrategy::TTL => {}
        }
        let cache = builder.build();
        Box::new(MokaCacheImpl { inner: Arc::new(cache) })
    } else {
        Box::new(NoOpCache)
    }
}
