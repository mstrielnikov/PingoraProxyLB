use pingora_load_balancing::{Backend, LoadBalancer, health_check::HealthCheck};
use pingora_load_balancing::selection::{RoundRobin, BackendSelection, BackendIter};
use std::sync::Arc;
use std::time::Duration;
use crate::config::AppConfig;

pub trait LoadBalancerTrait {
    fn select(&self, key: &[u8], max: usize) -> Option<Backend>;
}

#[derive(Clone)]
pub struct PingoraLB {
    pub inner: Arc<LoadBalancer<RoundRobin>>,
}

impl PingoraLB {
    pub fn new(upstreams: Arc<LoadBalancer<RoundRobin>>) -> Self {
        Self { inner: upstreams }
    }
}

impl LoadBalancerTrait for PingoraLB
where
    RoundRobin: BackendSelection,
    <RoundRobin as BackendSelection>::Iter: BackendIter,
{
    fn select(&self, key: &[u8], max: usize) -> Option<Backend> {
        self.inner.select(key, max)
    }
}

pub struct CustomHttpHealthCheck {
    inner: pingora_load_balancing::health_check::HttpHealthCheck,
    period: Duration,
    timeout: Duration,
}

impl CustomHttpHealthCheck {
    pub fn new(host: &str, tls: bool, period: Duration, timeout: Duration) -> Self {
        Self {
            inner: pingora_load_balancing::health_check::HttpHealthCheck::new(host, tls),
            period,
            timeout,
        }
    }
}

#[async_trait::async_trait]
impl HealthCheck for CustomHttpHealthCheck {
    async fn check(&self, backend: &Backend) -> Result<(), Box<pingora::Error>> {
        tracing::debug!("Custom health check for {} with period {:?}, timeout {:?}", backend.addr, self.period, self.timeout);
        self.inner.check(backend).await
    }

    fn health_threshold(&self, _healthy: bool) -> usize {
        1 // Default: 1 successful check to mark healthy, 1 failed to mark unhealthy
    }
}

pub fn create_load_balancer(config: &AppConfig) -> Result<PingoraLB, Box<dyn std::error::Error>> {
    let upstream_addrs = config.proxy.upstreams.clone();
    let mut upstreams = LoadBalancer::try_from_iter(upstream_addrs.clone())?;
    upstreams.set_health_check(Box::new(CustomHttpHealthCheck::new(
        "/health",
        false,
        Duration::from_secs(config.lb.health_check.period),
        Duration::from_secs(config.lb.health_check.timeout),
    )));
    if config.lb.circuit_breaker.enabled {
        tracing::warn!("Circuit breaker not supported in pingora v0.6.0; ignoring");
    }
    Ok(PingoraLB::new(Arc::new(upstreams)))
}
