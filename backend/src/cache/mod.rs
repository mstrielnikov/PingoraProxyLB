pub mod traits;
pub mod noop;
pub mod factory;

#[cfg(feature = "cache_moka")]
pub mod moka;
#[cfg(feature = "cache_redis")]
pub mod redis;
#[cfg(feature = "cache_memcache")]
pub mod memcache;

pub use traits::CacheTrait;
pub use noop::NoOpCache;
pub use factory::create_cache;

#[cfg(feature = "cache_moka")]
pub use moka::MokaCacheImpl;
