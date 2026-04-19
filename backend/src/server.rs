use pingora::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use std::error::Error;

use crate::config::{AppConfig, AuthProviderConfig};
use crate::proxy::upstreams::create_load_balancer;
use crate::cache::create_cache;
use crate::observability::Metrics;
use crate::middleware::{ErasedPipeline, PipelineBuilder};
use crate::auth::{AuthProvider, NoOpAuth};
use crate::proxy;

#[cfg(feature = "auth")]
use crate::auth::{KratosLocalAuth, KratosHttpAuth};
#[cfg(feature = "auth")]
use crate::auth::providers::{OpaAgentAuth, AwsIamAuth, AzureEntraIdAuth, CloudFlareAuth};
#[cfg(feature = "auth")]
use crate::auth::jwt::JwtOidcProvider;

/// Builds a fully operable Pingora Server pre-wired with the Chainless-LB middleware
/// architectures and metrics bindings.
pub fn build_server(config: &AppConfig) -> Result<Server, Box<dyn Error>> {
    let mut server = Server::new(None)?;
    server.bootstrap();

    let lb_impl = create_load_balancer(config)?;
    tracing::debug!("Load balancer initialised ({} upstreams)", config.proxy.upstreams.len());

    let cache = Arc::from(create_cache(config.cache.as_ref()));
    tracing::info!("Cache initialised");

    // ── Build type-state middleware pipeline ──────────────────────────────────
    let rl   = &config.lb.rate_limiter;
    let cb   = &config.lb.circuit_breaker;

    let pipeline: Arc<dyn ErasedPipeline> = match (rl.enabled, cb.enabled) {
        (true, true) => Arc::new(
            PipelineBuilder::new()
                .with_rate_limit(rl.rate)
                .with_circuit_breaker(cb.threshold, Duration::from_secs(cb.reset_secs))
                .build(),
        ),
        (true, false) => Arc::new(
            PipelineBuilder::new()
                .with_rate_limit(rl.rate)
                .build(),
        ),
        (false, true) => Arc::new(
            PipelineBuilder::new()
                .with_circuit_breaker(cb.threshold, Duration::from_secs(cb.reset_secs))
                .build(),
        ),
        (false, false) => Arc::new(PipelineBuilder::new().build()),
    };
    tracing::info!(
        "Middleware pipeline: rate_limit={}, circuit_breaker={}",
        rl.enabled, cb.enabled,
    );

    // ── Build auth provider ───────────────────────────────────────────────────
    let provider_cfg = config.auth.clone().unwrap_or_default().into_provider_config();
    let auth: Arc<dyn AuthProvider> = match provider_cfg {
        AuthProviderConfig::Noop => {
            tracing::info!("Auth provider: NoOp");
            Arc::new(NoOpAuth)
        }
        #[cfg(feature = "auth")]
        AuthProviderConfig::KratosLocal { jwt_secret, issuer } => {
            tracing::info!("Auth provider: KratosLocal (issuer={})", issuer);
            Arc::new(KratosLocalAuth::new(&jwt_secret, &issuer, Duration::from_secs(3600))?)
        }
        #[cfg(feature = "auth")]
        AuthProviderConfig::KratosHttp { endpoint } => {
            tracing::info!("Auth provider: KratosHttp (endpoint={})", endpoint);
            Arc::new(KratosHttpAuth::new(endpoint))
        }
        #[cfg(feature = "auth")]
        AuthProviderConfig::OpaAgent { endpoint } => {
            tracing::info!("Auth provider: OpaAgent (endpoint={})", endpoint);
            Arc::new(OpaAgentAuth::new(endpoint))
        }
        #[cfg(feature = "auth")]
        AuthProviderConfig::AwsIam { jwks_url } => {
            tracing::info!("Auth provider: AwsIam (jwks={})", jwks_url);
            Arc::new(AwsIamAuth::new(jwks_url))
        }
        #[cfg(feature = "auth")]
        AuthProviderConfig::AzureEntraId { tenant_id } => {
            tracing::info!("Auth provider: AzureEntraId (tenant={})", tenant_id);
            Arc::new(AzureEntraIdAuth::new(tenant_id))
        }
        #[cfg(feature = "auth")]
        AuthProviderConfig::CloudFlare { team_domain } => {
            tracing::info!("Auth provider: CloudFlare (team={})", team_domain);
            Arc::new(CloudFlareAuth::new(team_domain))
        }
        #[cfg(feature = "auth")]
        AuthProviderConfig::JwtOidc { endpoint } => {
            tracing::info!("Auth provider: JwtOidc (endpoint={})", endpoint);
            Arc::new(JwtOidcProvider::new(endpoint))
        }
    };

    let metrics = Metrics::new(config.metrics.as_ref());

    // ── Assemble proxy ────────────────────────────────────────────────────────
    let lb = proxy::LB::new(lb_impl, cache, pipeline, auth, metrics, config.proxy.clone());

    let mut service = http_proxy_service(&server.configuration, lb);
    service.add_tcp(&config.server.http_addr);

    if config.proxy.tls.enabled {
        let cert = &config.proxy.tls.cert;
        let key = &config.proxy.tls.key;
        let pqc_settings = crate::tls::get_optimized_tls_settings(cert, key)?;
        
        let mut tls_settings = pingora::listeners::tls::TlsSettings::intermediate(cert, key)?;
        if let Some(kem) = pqc_settings.kem_algorithm {
            // In pingora 0.6 this hits the inner SslAcceptorBuilder
            // For PQC (X25519Kyber768Draft00)
            tls_settings.set_groups_list(&kem)?;
        }
        
        service.add_tls_with_settings(&config.server.https_addr, None, tls_settings);
        tracing::info!("TLS enabled on {} (cert={})", config.server.https_addr, config.proxy.tls.cert);
    }
    tracing::info!("HTTP proxy listening on {} (TLS={})", config.server.http_addr, config.proxy.tls.enabled);
    server.add_service(service);
    
    Ok(server)
}
