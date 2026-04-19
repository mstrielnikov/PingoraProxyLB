use pingora::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use chainless_lb_backend::config::load_config;
use chainless_lb_backend::proxy::upstreams::create_load_balancer;
use chainless_lb_backend::cache::create_cache;
use chainless_lb_backend::observability::{init_logging, Metrics};
use chainless_lb_backend::middleware::PipelineBuilder;
use chainless_lb_backend::auth::NoOpAuth;
use chainless_lb_backend::proxy;
use chainless_lb_backend::config::LoggingConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging(&LoggingConfig { level: "info".to_string() });
    
    // We only load Config to initialize some upstream IPs for demonstration.
    // The pipeline itself is structurally locked by Rust's compiler (NoOpAuth, RL=1000, CB=Enabled).
    let config = load_config(None)?;

    // Instantiate Pingora server
    let mut server = Server::new(None)?;
    server.bootstrap();

    // Statically typed builder chain mapping to exact memory layouts
    let pipeline = PipelineBuilder::new()
        .with_rate_limit(1000)
        .with_circuit_breaker(5, Duration::from_secs(30))
        .build();

    // Statically typed Auth Provider
    let auth = NoOpAuth;

    // Load dynamic upstreams safely behind LB generic trait
    let lb_impl = create_load_balancer(&config)?;
    let cache = Arc::from(create_cache(config.cache.as_ref()));
    let metrics = Metrics::new(config.metrics.as_ref());

    // The proxy state machine is natively dispatched!
    let lb = proxy::LB::new(lb_impl, cache, pipeline, auth, metrics, config.proxy.clone());

    let mut service = http_proxy_service(&server.configuration, lb);
    service.add_tcp("127.0.0.1:8001");
    
    tracing::info!("HPC Static-Dispatch Server running on 127.0.0.1:8001");
    server.add_service(service);
    server.run_forever();
}
