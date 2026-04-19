pub mod cache;
pub mod config;
pub mod observability;
pub mod middleware;
pub mod auth;
pub mod proxy;
pub mod server;
pub mod tls;
pub mod ebpf;

// Re-export key types and traits
pub use cache::{CacheTrait, create_cache, NoOpCache};
#[cfg(feature = "cache_moka")]
pub use cache::MokaCacheImpl;
pub use config::{AppConfig, AuthConfig, AuthProviderConfig, CacheStrategy, LoadBalancerStrategy};
pub use proxy::upstreams::{LoadBalancerTrait, PingoraLB};
pub use observability::{init_logging, Metrics};
pub use middleware::{
    ErasedPipeline, MiddlewarePipeline, PipelineBuilder,
    NoRateLimit, WithRateLimit, RateLimitLayer,
    NoCircuitBreaker, WithCircuitBreaker, CircuitBreakerLayer,
};
pub use auth::{
    AuthProvider, AuthError,
    NoOpAuth,
};
#[cfg(feature = "auth")]
pub use auth::{KratosLocalAuth, KratosHttpAuth};
pub use proxy::{LB, BodyCtx};
