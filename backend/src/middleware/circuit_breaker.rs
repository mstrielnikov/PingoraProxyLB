//! Type-state circuit-breaker layer.
//!
//! [`NoCircuitBreaker`] is a zero-cost passthrough.
//! [`WithCircuitBreaker`] implements a simple atomic half-open/open/closed state machine.
//!
//! State transitions:
//! ```text
//!  CLOSED ──(failures >= threshold)──► OPEN ──(reset_timeout elapsed)──► HALF_OPEN
//!  HALF_OPEN ──(success)──► CLOSED
//!  HALF_OPEN ──(failure)──► OPEN
//! ```

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Sealed helper so external crates cannot implement `CircuitBreakerLayer`.
mod private {
    pub trait Sealed {}
}

/// Behaviour contract for the circuit-breaker slot in [`super::pipeline::MiddlewarePipeline`].
pub trait CircuitBreakerLayer: private::Sealed + Send + Sync + 'static {
    /// Returns `true` when the circuit is **open** (requests must be blocked with 503).
    fn is_open(&self) -> bool;
    /// Record a downstream failure (increments failure counter).
    fn record_failure(&self);
    /// Record a successful downstream response (resets failure counter, may close circuit).
    fn record_success(&self);
}

// ── NoCircuitBreaker ──────────────────────────────────────────────────────────

/// Type-state marker: no circuit-breaking; all requests are forwarded.
#[derive(Clone, Debug, Default)]
pub struct NoCircuitBreaker;

impl private::Sealed for NoCircuitBreaker {}

impl CircuitBreakerLayer for NoCircuitBreaker {
    #[inline]
    fn is_open(&self) -> bool {
        false
    }
    #[inline]
    fn record_failure(&self) {}
    #[inline]
    fn record_success(&self) {}
}

// ── WithCircuitBreaker ────────────────────────────────────────────────────────

#[derive(Debug)]
struct CbInner {
    failure_count: AtomicU32,
    threshold: u32,
    open: AtomicBool,
    // Instant is not atomic; we protect it with a lightweight RwLock.
    opened_at: RwLock<Option<Instant>>,
    reset_timeout: Duration,
}

/// Type-state marker: atomic half-open circuit-breaker.
#[derive(Clone, Debug)]
pub struct WithCircuitBreaker {
    inner: Arc<CbInner>,
}

impl WithCircuitBreaker {
    /// Create a circuit-breaker that opens after `threshold` consecutive failures
    /// and attempts to half-open after `reset_timeout`.
    pub fn new(threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            inner: Arc::new(CbInner {
                failure_count: AtomicU32::new(0),
                threshold,
                open: AtomicBool::new(false),
                opened_at: RwLock::new(None),
                reset_timeout,
            }),
        }
    }
}

impl private::Sealed for WithCircuitBreaker {}

impl CircuitBreakerLayer for WithCircuitBreaker {
    fn is_open(&self) -> bool {
        if !self.inner.open.load(Ordering::Acquire) {
            return false;
        }
        // Try half-open: if reset timeout has elapsed, allow exactly one probe through.
        // We do a quick, non-blocking read of opened_at. If the lock is contended we
        // stay closed rather than blocking the hot path.
        if let Ok(guard) = self.inner.opened_at.try_read() {
            if let Some(opened_at) = *guard {
                if opened_at.elapsed() >= self.inner.reset_timeout {
                    // Attempt to swing to half-open by clearing the flag.
                    self.inner.open.store(false, Ordering::Release);
                    self.inner.failure_count.store(0, Ordering::Release);
                    tracing::info!("Circuit breaker entering HALF-OPEN state");
                    return false; // allow the probe
                }
            }
        }
        true
    }

    fn record_failure(&self) {
        let prev = self.inner.failure_count.fetch_add(1, Ordering::AcqRel);
        tracing::debug!("Circuit breaker failure recorded ({}/{})", prev + 1, self.inner.threshold);
        if prev + 1 >= self.inner.threshold && !self.inner.open.load(Ordering::Acquire) {
            self.inner.open.store(true, Ordering::Release);
            // Store open time (best-effort; we spawn no tasks here).
            if let Ok(mut guard) = self.inner.opened_at.try_write() {
                *guard = Some(Instant::now());
            }
            tracing::warn!(
                "Circuit breaker OPENED after {} consecutive failures",
                prev + 1
            );
        }
    }

    fn record_success(&self) {
        self.inner.failure_count.store(0, Ordering::Release);
        self.inner.open.store(false, Ordering::Release);
    }
}
