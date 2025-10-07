mod proxy;
mod config;
mod cache;
mod load_balancer;
mod logging;
mod metrics;

use crate::config::{AppConfig, LoadBalancerStrategy};
use crate::cache::create_cache;
use crate::load_balancer::{PingoraLB, create_load_balancer};
use crate::logging::init_logging;
use crate::metrics::Metrics;
use crate::proxy::LB;
use pingora::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use auth_framework::{AuthFramework, AuthConfig as AuthFrameworkConfig, methods::JwtMethod};
use tokio::runtime::Runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load_config()?;
    logging::init_logging(&config.logging);
    tracing::info!("Starting Chainless LB Backend server with config: {:?}", config);

    let mut server = Server::new(None)?;
    server.bootstrap();

    let lb_impl = create_load_balancer(&config)?;
    tracing::debug!("Load balancer initialized with {} upstreams", config.proxy.upstreams.len());

    let cache = create_cache(&config);
    tracing::info!("Embedded Moka cache initialized (enabled: {}, strategy: {:?}", config.cache.enabled, config.cache.strategy);

    let rt = Runtime::new()?;
    let mut auth = AuthFramework::new(AuthFrameworkConfig::new().token_lifetime(Duration::from_secs(3600)));
    let jwt_secret = config.auth.jwt.jwt_secret.clone();
    let auth_method = match config.auth.jwt.auth_type.as_str() {
        "jwt" => {
            let jwt_method = JwtMethod::new()
                .secret_key(&jwt_secret)
                .issuer("chainless-lb-backend");
            auth_framework::methods::AuthMethodEnum::Jwt(jwt_method)
        }
        "oidc" => unimplemented!("OIDC support not implemented"),
        _ => panic!("Unsupported auth type: {}", config.auth.jwt.auth_type),
    };
    auth.register_method(&config.auth.jwt.auth_type, auth_method);
    rt.block_on(auth.initialize())?;
    tracing::info!("AuthFramework initialized with method: {}", config.auth.jwt.auth_type);

    let metrics = Metrics::new(&config.metrics);

    let lb = LB::new(
        lb_impl,
        Arc::from(cache),
        Arc::new(auth),
        metrics,
        config.lb.rateLimiter.rate,
    );

    let mut service = http_proxy_service(&server.configuration, lb);
    service.add_tcp("0.0.0.0:3000");

    if config.proxy.tls.enabled {
        service.add_tls("0.0.0.0:443", &config.proxy.tls.cert, &config.proxy.tls.key)?;
        tracing::info!("TLS enabled on 0.0.0.0:443 with cert: {}", config.proxy.tls.cert);
    }
    tracing::info!("HTTP proxy service added on 0.0.0.0:3000 (TLS: {})", config.proxy.tls.enabled);
    server.add_service(service);

    tracing::info!("Server starting... Use RUST_LOG=debug for more details");
    server.run_forever()
}
