//! Tower-style middleware stack for Chainless-LB.
//!
//! Each middleware concern (rate-limiting, circuit-breaking) is represented as a
//! *type-state* marker so the compiler enforces correct composition at the call site.
//!
//! ```text
//! PipelineBuilder::new()
//!     .with_rate_limit(100)
//!     .with_circuit_breaker(5)
//!     .build()
//! ```

pub mod circuit_breaker;
pub mod pipeline;
pub mod rate_limit;

pub use circuit_breaker::{CircuitBreakerLayer, NoCircuitBreaker, WithCircuitBreaker};
pub use pipeline::{MiddlewarePipeline, PipelineBuilder};
pub use rate_limit::{NoRateLimit, RateLimitLayer, WithRateLimit};

// ── ErasedPipeline ────────────────────────────────────────────────────────────

/// Object-safe mirror of [`MiddlewarePipeline`] used for runtime type erasure.
///
/// When the active middleware combination is only known at runtime (e.g. from
/// config), use `pipeline.erase()` to obtain a `Box<dyn ErasedPipeline>` and
/// store it behind `Arc<dyn ErasedPipeline>`.
pub trait ErasedPipeline: Send + Sync + 'static {
    /// Returns `true` when the rate-limit allows the request.
    fn check_rate_limit(&self) -> bool;
    /// Returns `true` when the circuit is open (upstream considered unhealthy).
    fn is_circuit_open(&self) -> bool;
    /// Record a successful upstream response.
    fn record_success(&self);
    /// Record a failed upstream response.
    fn record_failure(&self);
}

/// Blanket impl: any `MiddlewarePipeline<RL, CB>` automatically implements `ErasedPipeline`.
impl<RL, CB> ErasedPipeline for MiddlewarePipeline<RL, CB>
where
    RL: RateLimitLayer,
    CB: CircuitBreakerLayer,
{
    fn check_rate_limit(&self) -> bool { self.rate_limit.check() }
    fn is_circuit_open(&self) -> bool  { self.circuit_breaker.is_open() }
    fn record_success(&self)           { self.circuit_breaker.record_success(); }
    fn record_failure(&self)           { self.circuit_breaker.record_failure(); }
}

impl ErasedPipeline for std::sync::Arc<dyn ErasedPipeline> {
    fn check_rate_limit(&self) -> bool { (**self).check_rate_limit() }
    fn is_circuit_open(&self) -> bool  { (**self).is_circuit_open() }
    fn record_success(&self)           { (**self).record_success(); }
    fn record_failure(&self)           { (**self).record_failure(); }
}

