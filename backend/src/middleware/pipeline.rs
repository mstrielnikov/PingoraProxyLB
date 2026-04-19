//! Composable middleware pipeline.
//!
//! Assembles any combination of [`RateLimitLayer`] + [`CircuitBreakerLayer`] implementations
//! via the type-state [`PipelineBuilder`] and produces a zero-overhead
//! [`MiddlewarePipeline`] that exposes the combined checks.

use super::circuit_breaker::{CircuitBreakerLayer, NoCircuitBreaker, WithCircuitBreaker};
use super::rate_limit::{NoRateLimit, RateLimitLayer, WithRateLimit};
use std::time::Duration;

// ── MiddlewarePipeline ────────────────────────────────────────────────────────

/// The assembled, immutable middleware pipeline.
///
/// The type parameters encode exactly which layers are active:
/// - `RL`: `NoRateLimit` or `WithRateLimit`
/// - `CB`: `NoCircuitBreaker` or `WithCircuitBreaker`
#[derive(Clone, Debug)]
pub struct MiddlewarePipeline<RL: RateLimitLayer, CB: CircuitBreakerLayer> {
    pub(crate) rate_limit: RL,
    pub(crate) circuit_breaker: CB,
}

impl<RL: RateLimitLayer, CB: CircuitBreakerLayer> MiddlewarePipeline<RL, CB> {
    /// Returns `true` when the rate-limit allows the request to proceed.
    #[inline]
    pub fn check_rate_limit(&self) -> bool {
        self.rate_limit.check()
    }

    /// Returns `true` when the circuit is open (upstream is considered unhealthy).
    #[inline]
    pub fn is_circuit_open(&self) -> bool {
        self.circuit_breaker.is_open()
    }

    /// Record a successful upstream response.
    #[inline]
    pub fn record_success(&self) {
        self.circuit_breaker.record_success();
    }

    /// Record a failed upstream response.
    #[inline]
    pub fn record_failure(&self) {
        self.circuit_breaker.record_failure();
    }
}

// ── PipelineBuilder ───────────────────────────────────────────────────────────

/// Fluent type-state builder for [`MiddlewarePipeline`].
///
/// Use `.with_rate_limit(rate)` and/or `.with_circuit_breaker(threshold, reset_timeout)`
/// in any order, then call `.build()`.
///
/// # Example
/// ```rust,ignore
/// let pipeline = PipelineBuilder::new()
///     .with_rate_limit(100)
///     .with_circuit_breaker(5, Duration::from_secs(30))
///     .build();
/// ```
#[derive(Debug)]
pub struct PipelineBuilder<RL: RateLimitLayer, CB: CircuitBreakerLayer> {
    rate_limit: RL,
    circuit_breaker: CB,
}

impl PipelineBuilder<NoRateLimit, NoCircuitBreaker> {
    /// Create a builder with no active layers (all pass-through).
    pub fn new() -> Self {
        Self {
            rate_limit: NoRateLimit,
            circuit_breaker: NoCircuitBreaker,
        }
    }
}

impl Default for PipelineBuilder<NoRateLimit, NoCircuitBreaker> {
    fn default() -> Self {
        Self::new()
    }
}

impl<CB: CircuitBreakerLayer> PipelineBuilder<NoRateLimit, CB> {
    /// Add a GCRA global rate-limiter allowing `rate` requests per second.
    pub fn with_rate_limit(self, rate: u32) -> PipelineBuilder<WithRateLimit, CB> {
        PipelineBuilder {
            rate_limit: WithRateLimit::new(rate),
            circuit_breaker: self.circuit_breaker,
        }
    }
}

impl<RL: RateLimitLayer> PipelineBuilder<RL, NoCircuitBreaker> {
    /// Add a circuit-breaker that opens after `threshold` consecutive failures and
    /// attempts a half-open probe after `reset_timeout`.
    pub fn with_circuit_breaker(
        self,
        threshold: u32,
        reset_timeout: Duration,
    ) -> PipelineBuilder<RL, WithCircuitBreaker> {
        PipelineBuilder {
            rate_limit: self.rate_limit,
            circuit_breaker: WithCircuitBreaker::new(threshold, reset_timeout),
        }
    }
}

impl<RL: RateLimitLayer, CB: CircuitBreakerLayer> PipelineBuilder<RL, CB> {
    /// Consume the builder and produce the assembled [`MiddlewarePipeline`].
    pub fn build(self) -> MiddlewarePipeline<RL, CB> {
        MiddlewarePipeline {
            rate_limit: self.rate_limit,
            circuit_breaker: self.circuit_breaker,
        }
    }
}
