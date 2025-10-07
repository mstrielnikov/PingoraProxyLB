use async_trait::async_trait;
use pingora::prelude::*;
use pingora_proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use pingora::http::ResponseHeader;
use crate::load_balancer::LoadBalancerTrait;
use crate::cache::CacheTrait;
use crate::metrics::Metrics;
use std::sync::Arc;
use serde_json::json;
use bytes::Bytes;
use auth_framework::{AuthFramework, AuthToken};
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::time::Duration;

#[derive(Clone)]
pub struct BodyCtx {
    accumulated_body: Vec<u8>,
    selected_host: Option<String>,
}

pub struct LB<LBImpl>
where
    LBImpl: LoadBalancerTrait + Send + Sync + 'static,
{
    upstreams: LBImpl,
    cache: Arc<dyn CacheTrait + Send + Sync + 'static>,
    auth: Arc<AuthFramework>,
    rate_limiter: Arc<RateLimiter<governor::state::direct::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    metrics: Metrics,
}

impl<LBImpl> LB<LBImpl>
where
    LBImpl: LoadBalancerTrait + Send + Sync + 'static,
{
    pub fn new(
        upstreams: LBImpl,
        cache: Arc<dyn CacheTrait + Send + Sync + 'static>,
        auth: Arc<AuthFramework>,
        metrics: Metrics,
        rate_limit: u32,
    ) -> Self {
        let rate_limiter = RateLimiter::direct(Quota::per_second(NonZeroU32::new(rate_limit).unwrap_or(NonZeroU32::new(100).unwrap())));
        Self {
            upstreams,
            cache,
            auth,
            rate_limiter: Arc::new(rate_limiter),
            metrics,
        }
    }

    async fn send_json_response(&self, session: &mut Session, token: String) -> Result<bool> {
        let body = json!({ "token": token }).to_string();
        let body_bytes = Bytes::from(body);
        let mut resp = ResponseHeader::build(200, Some(body_bytes.len()))?;
        resp.insert_header("Content-Type", "application/json")?;
        session.write_response_header(Box::new(resp), false).await?;
        session.write_response_body(Some(body_bytes), true).await?;
        Ok(true)
    }

    async fn send_json_error(&self, session: &mut Session, status: u16, message: &str) -> Result<bool> {
        let body = json!({ "error": message }).to_string();
        let body_bytes = Bytes::from(body);
        let mut resp = ResponseHeader::build(status, Some(body_bytes.len()))?;
        resp.insert_header("Content-Type", "application/json")?;
        session.write_response_header(Box::new(resp), false).await?;
        session.write_response_body(Some(body_bytes), true).await?;
        Ok(true)
    }
}

#[async_trait]
impl<LBImpl> ProxyHttp for LB<LBImpl>
where
    LBImpl: LoadBalancerTrait + Send + Sync + 'static,
{
    type CTX = BodyCtx;

    fn new_ctx(&self) -> Self::CTX {
        tracing::trace!("New request context created");
        BodyCtx {
            accumulated_body: Vec::new(),
            selected_host: None,
        }
    }

    async fn upstream_peer(&self, _session: &mut Session, ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
        let max_retries = 3;
        for attempt in 0..max_retries {
            match self.upstreams.select(b"", 256) {
                Some(backend) => {
                    let host = backend.addr.to_string();
                    ctx.selected_host = Some(host.clone());
                    tracing::debug!("Attempt {}: Selected upstream backend: {}", attempt + 1, host);
                    let peer = Box::new(HttpPeer::new(backend.addr, false, String::new()));
                    return Ok(peer);
                }
                None => {
                    tracing::warn!("Attempt {}: No healthy upstream available", attempt + 1);
                    if attempt + 1 == max_retries {
                        return Err(Error::new_str("No healthy upstream available after retries"));
                    }
                }
            }
        }
        Err(Error::new_str("No healthy upstream available"))
    }

    async fn request_filter(&self, session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        if let Some(counter) = &self.metrics.request_counter {
            counter.add(1, &[]);
        }
        let path = session.req_header().uri.path();
        tracing::debug!("Request filter for path: {}", path);

        if self.rate_limiter.check().is_err() {
            return self.send_json_error(session, 429, "Rate limit exceeded").await;
        }

        if path.starts_with("/public") || path == "/health" {
            tracing::debug!("Public path access: {}", path);
            return Ok(false);
        }

        let auth_header = session.req_header().headers.get("Authorization");
        if let Some(auth_value) = auth_header {
            if let Ok(auth_str) = auth_value.to_str() {
                if let Some(token) = auth_str.strip_prefix("Bearer ").map(|s| s.trim()) {
                    let cache_key = format!("valid_session:{}", token);
                    let revocation_key = format!("revoked_token:{}", token);
                    if self.cache.get(&revocation_key).await.is_some() {
                        tracing::warn!("Revoked token detected: {}", token);
                        return self.send_json_error(session, 401, "Token revoked").await;
                    }
                    if self.cache.get(&cache_key).await.is_some() {
                        tracing::trace!("Token valid via cache: {}", token);
                        if let Some(counter) = &self.metrics.cache_hits {
                            counter.add(1, &[]);
                        }
                        return Ok(false);
                    } else {
                        if let Some(counter) = &self.metrics.cache_misses {
                            counter.add(1, &[]);
                        }
                    }

                    let auth_token = AuthToken::new(
                        token.to_string(),
                        String::new(), // Placeholder access_token
                        Duration::from_secs(3600), // Placeholder duration
                        String::from("jwt"), // Placeholder auth_method
                    );
                    return match self.auth.validate_token(&auth_token).await {
                        Ok(true) => {
                            tracing::debug!("Token validated by AuthFramework");
                            let token_string = token.to_string();
                            self.cache.insert(cache_key.clone(), token_string).await;
                            Ok(false)
                        }
                        Ok(false) | Err(_) => {
                            tracing::warn!("Token validation failed for path: {}", path);
                            self.cache.evict(&cache_key).await;
                            self.send_json_error(session, 401, "Invalid or expired token").await
                        }
                    };
                }
            }
        }

        tracing::warn!("Request missing required Authorization header for path: {}", path);
        self.send_json_error(session, 401, "Missing Authorization header").await
    }

    async fn proxy_upstream_filter(&self, _session: &mut Session, _ctx: &mut Self::CTX) -> Result<bool> {
        tracing::trace!("Proxy upstream filter - continuing");
        Ok(false)
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        if let Some(host) = &ctx.selected_host {
            upstream_request.insert_header("Host", host)?;
            tracing::trace!("Upstream request headers modified (Host: {})", host);
        }
        Ok(())
    }
}
