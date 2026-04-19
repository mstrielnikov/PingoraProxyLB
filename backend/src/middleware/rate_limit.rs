//! Type-state rate-limiting layer.
//!
//! [`NoRateLimit`] is a zero-sized, zero-cost passthrough marker.
//! [`WithRateLimit`] wraps a `governor` GCRA rate-limiter behind `Arc` for cheap cloning.

use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{direct::NotKeyed, InMemoryState},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Sealed helper so external crates cannot implement `RateLimitLayer`.
mod private {
    pub trait Sealed {}
}

/// Behaviour contract for the rate-limit slot in [`super::pipeline::MiddlewarePipeline`].
/// Returns `true` when the request **may proceed**, `false` when it must be rejected (429).
pub trait RateLimitLayer: private::Sealed + Send + Sync + 'static {
    fn check(&self) -> bool;
}

// ── NoRateLimit ──────────────────────────────────────────────────────────────

/// Type-state marker: no rate-limiting; every request passes through.
#[derive(Clone, Debug, Default)]
pub struct NoRateLimit;

impl private::Sealed for NoRateLimit {}

impl RateLimitLayer for NoRateLimit {
    #[inline]
    fn check(&self) -> bool {
        true
    }
}

// ── WithRateLimit ─────────────────────────────────────────────────────────────

type GovernorRL = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// Type-state marker: GCRA rate-limiter with a global (not-keyed) token bucket.
#[derive(Clone)]
pub struct WithRateLimit {
    inner: Arc<GovernorRL>,
}

impl WithRateLimit {
    /// Construct a rate-limiter allowing `rate` requests per second globally.
    ///
    /// Panics if `rate` is 0.
    pub fn new(rate: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(rate).expect("rate-limit rate must be non-zero"),
        );
        Self {
            inner: Arc::new(GovernorRateLimiter::direct(quota)),
        }
    }
}

impl private::Sealed for WithRateLimit {}

impl RateLimitLayer for WithRateLimit {
    #[inline]
    fn check(&self) -> bool {
        self.inner.check().is_ok()
    }
}
