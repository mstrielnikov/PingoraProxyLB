//! ORY Kratos HTTP session validator.
//!
//! Delegates token verification to a running ORY Kratos instance by calling
//! `GET <endpoint>/sessions/whoami` with `X-Session-Token: <token>`.
//!
//! - HTTP 200 → valid session → `Ok(true)`
//! - HTTP 401 / 403 → invalid/expired session → `Ok(false)`
//! - Other / transport error → `Err(AuthError::Transport)`

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use super::{AuthError, AuthProvider};
use super::private::Sealed;

/// Validates Bearer tokens by asking ORY Kratos `/sessions/whoami`.
#[derive(Clone, Debug)]
pub struct KratosHttpAuth {
    /// Base URL of the ORY Kratos public API, e.g. `http://kratos:4433`.
    endpoint: String,
    client: Client,
}

impl KratosHttpAuth {
    /// Create a new provider pointing at `endpoint` (no trailing slash needed).
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    fn whoami_url(&self) -> String {
        format!("{}/sessions/whoami", self.endpoint.trim_end_matches('/'))
    }
}

impl Sealed for KratosHttpAuth {}

#[async_trait]
impl AuthProvider for KratosHttpAuth {
    async fn authenticate(&self, token: &str) -> Result<bool, AuthError> {
        let url = self.whoami_url();
        tracing::debug!("KratosHttpAuth: GET {}", url);

        let resp = self
            .client
            .get(&url)
            .header("X-Session-Token", token)
            .send()
            .await
            .map_err(|e| AuthError::Transport(format!("Kratos HTTP request failed: {e}")))?;

        match resp.status() {
            StatusCode::OK => {
                tracing::debug!("KratosHttpAuth: session valid");
                Ok(true)
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                tracing::warn!("KratosHttpAuth: session invalid ({})", resp.status());
                Ok(false)
            }
            other => {
                let body = resp.text().await.unwrap_or_default();
                Err(AuthError::Transport(format!(
                    "Kratos unexpected status {other}: {body}"
                )))
            }
        }
    }
}
