use crate::cache::traits::CacheTrait;
use crate::cache::noop::NoOpCache;
use crate::config::CacheConfig;

#[cfg(feature = "cache_moka")]
use crate::cache::moka::MokaCacheImpl;
#[cfg(feature = "cache_moka")]
use crate::config::CacheStrategy;
#[cfg(feature = "cache_moka")]
use moka::future::Cache as MokaCache;
#[cfg(feature = "cache_moka")]
use std::time::Duration;
#[cfg(feature = "cache_moka")]
use std::sync::Arc;

pub fn create_cache(config: Option<&CacheConfig>) -> Box<dyn CacheTrait + Send + Sync + 'static> {
    if let Some(cfg) = config {
        if cfg.enabled {
            #[cfg(feature = "cache_moka")]
            {
                let ttl = Duration::from_secs(cfg.ttl_secs);
                let mut builder = MokaCache::builder()
                    .time_to_live(ttl)
                    .max_capacity(cfg.max_capacity);
                match cfg.strategy {
                    CacheStrategy::LRU => builder = builder.eviction_policy(moka::policy::EvictionPolicy::lru()),
                    CacheStrategy::LFU => builder = builder.eviction_policy(moka::policy::EvictionPolicy::tiny_lfu()),
                    CacheStrategy::TTL => {}
                }
                let cache = builder.build();
                return Box::new(MokaCacheImpl { inner: Arc::new(cache) });
            }
            #[cfg(not(feature = "cache_moka"))]
            {
                tracing::warn!("Cache is enabled in config, but cache_moka feature is disabled. Using NoOpCache.");
                return Box::new(NoOpCache);
            }
        } else {
            Box::new(NoOpCache)
        }
    } else {
        Box::new(NoOpCache)
    }
}
