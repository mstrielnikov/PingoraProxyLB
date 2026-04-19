use async_trait::async_trait;
use pingora::prelude::*;
use pingora_proxy::{ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use pingora::http::ResponseHeader;
use std::sync::Arc;
use serde_json::json;
use bytes::Bytes;

use crate::proxy::upstreams::LoadBalancerTrait;
use crate::cache::CacheTrait;
use crate::observability::Metrics;

use crate::auth::AuthProvider;
use crate::config::ProxyConfig;

#[derive(Clone)]
pub struct BodyCtx {
    pub accumulated_body: Vec<u8>,
    pub selected_host: Option<String>,
}

pub struct LB<LBImpl, P, A>
where
    LBImpl: LoadBalancerTrait + Send + Sync + 'static,
    P: Send + Sync + 'static,
    A: AuthProvider + Send + Sync + 'static,
{
    upstreams: LBImpl,
    cache: Arc<dyn CacheTrait + Send + Sync + 'static>,
    pipeline: P,
    auth: A,
    metrics: Metrics,
    config: ProxyConfig,
}

impl<LBImpl, P, A> LB<LBImpl, P, A>
where
    LBImpl: LoadBalancerTrait + Send + Sync + 'static,
    P: crate::middleware::ErasedPipeline + Send + Sync + 'static,
    A: AuthProvider + Send + Sync + 'static,
{
    pub fn new(
        upstreams: LBImpl,
        cache: Arc<dyn CacheTrait + Send + Sync + 'static>,
        pipeline: P,
        auth: A,
        metrics: Metrics,
        config: ProxyConfig,
    ) -> Self {
        Self { upstreams, cache, pipeline, auth, metrics, config }
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

    async fn send_json_error(
        &self,
        session: &mut Session,
        status: u16,
        message: &str,
    ) -> Result<bool> {
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
impl<LBImpl, P, A> ProxyHttp for LB<LBImpl, P, A>
where
    LBImpl: LoadBalancerTrait + Send + Sync + 'static,
    P: crate::middleware::ErasedPipeline + Send + Sync + 'static,
    A: AuthProvider + Send + Sync + 'static,
{
    type CTX = BodyCtx;

    fn new_ctx(&self) -> Self::CTX {
        tracing::trace!("New request context created");
        BodyCtx { accumulated_body: Vec::new(), selected_host: None }
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        if self.pipeline.is_circuit_open() {
            tracing::warn!("Circuit breaker OPEN — rejecting upstream selection");
            return Err(Error::new_str("Circuit breaker open: upstream unavailable"));
        }

        let max_retries = 3;
        for attempt in 0..max_retries {
            match self.upstreams.select(b"", 256) {
                Some(backend) => {
                    let host = backend.addr.to_string();
                    ctx.selected_host = Some(host.clone());
                    tracing::debug!("Attempt {}: upstream selected: {}", attempt + 1, host);
                    self.pipeline.record_success();
                    
                    let mut peer = HttpPeer::new(backend.addr, self.config.upstream_tls, self.config.upstream_sni.clone());
                    if let Some(recv_buf) = self.config.tcp_recv_buf {
                        peer.options.tcp_recv_buf = Some(recv_buf);
                    }
                    if let Some(_send_buf) = self.config.tcp_send_buf {
                        // Pingora HttpPeer doesn't support generic send buf natively inside PeerOptions
                        // but we leave this branch for connection_filter hooks if needed later
                    }
                    return Ok(Box::new(peer));
                }
                None => {
                    tracing::warn!("Attempt {}: no healthy upstream", attempt + 1);
                    self.pipeline.record_failure();
                    if attempt + 1 == max_retries {
                        return Err(Error::new_str("No healthy upstream after retries"));
                    }
                }
            }
        }
        Err(Error::new_str("No healthy upstream available"))
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<bool> {
        self.metrics.record_request();

        let path = session.req_header().uri.path().to_owned();
        tracing::debug!("request_filter path={}", path);

        if !self.pipeline.check_rate_limit() {
            tracing::warn!("Rate limit exceeded path={}", path);
            return self.send_json_error(session, 429, "Rate limit exceeded").await;
        }

        if path.starts_with("/public") || path == "/health" {
            tracing::debug!("Public path — skipping auth: {}", path);
            return Ok(false);
        }

        let token = match session
            .req_header()
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").map(|t| t.trim().to_owned()))
        {
            Some(t) => t,
            None => {
                tracing::warn!("Missing Authorization header path={}", path);
                return self
                    .send_json_error(session, 401, "Missing Authorization header")
                    .await;
            }
        };

        let revocation_key = format!("revoked_token:{}", token);
        if self.cache.get(&revocation_key).await.is_some() {
            tracing::warn!("Revoked token path={}", path);
            return self.send_json_error(session, 401, "Token revoked").await;
        }

        let cache_key = format!("valid_session:{}", token);
        if self.cache.get(&cache_key).await.is_some() {
            tracing::trace!("Token valid (cache hit) path={}", path);
            self.metrics.record_cache_hit();
            return Ok(false);
        }
        self.metrics.record_cache_miss();

        match self.auth.authenticate(&token).await {
            Ok(true) => {
                tracing::debug!("Auth: token valid path={}", path);
                self.cache.insert(cache_key, token).await;
                Ok(false)
            }
            Ok(false) => {
                tracing::warn!("Auth: token invalid path={}", path);
                self.send_json_error(session, 401, "Invalid or expired token").await
            }
            Err(e) => {
                tracing::error!("Auth provider error: {} path={}", e, path);
                self.send_json_error(session, 502, "Authentication provider error").await
            }
        }
    }

    async fn proxy_upstream_filter(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<bool> {
        Ok(false)
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        if let Some(host) = &ctx.selected_host {
            if let Err(e) = upstream_request.insert_header("Host", host) {
                tracing::warn!("Failed to insert Host header: {}", e);
            } else {
                tracing::trace!("Upstream Host: {}", host);
            }
        }
        Ok(())
    }
}
