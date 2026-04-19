//! ORY Kratos local JWT validator.
//!
//! Wraps `auth-framework`'s `AuthFramework + JwtMethod` — the same logic that
//! previously lived inline in `proxy.rs::request_filter`.

use async_trait::async_trait;
use auth_framework::{AuthConfig as AuthFrameworkConfig, AuthFramework, AuthToken};
use auth_framework::methods::{AuthMethodEnum, JwtMethod};
use std::sync::Arc;
use std::time::Duration;
use super::{AuthError, AuthProvider};
use super::private::Sealed;

/// Validates Bearer tokens locally using HMAC-SHA256 JWT verification
/// via the `auth-framework` crate.
#[derive(Clone)]
pub struct KratosLocalAuth {
    inner: Arc<AuthFramework>,
}

impl KratosLocalAuth {
    /// Build and initialise the auth framework synchronously.
    ///
    /// `secret` — HMAC secret for JWT verification.
    /// `issuer` — expected `iss` claim (e.g. `"chainless-lb-backend"`).
    /// `token_lifetime` — validity window for issued tokens.
    pub fn new(secret: &str, issuer: &str, token_lifetime: Duration) -> Result<Self, AuthError> {
        let mut framework =
            AuthFramework::new(AuthFrameworkConfig::new().token_lifetime(token_lifetime));

        let method = AuthMethodEnum::Jwt(
            JwtMethod::new()
                .secret_key(secret)
                .issuer(issuer),
        );
        framework.register_method("jwt", method);

        // initialize() is async; block on it here so construction stays sync.
        tokio::runtime::Handle::current()
            .block_on(framework.initialize())
            .map_err(|e| AuthError::Validation(format!("AuthFramework init failed: {e}")))?;

        tracing::info!("KratosLocalAuth initialised (issuer={})", issuer);
        Ok(Self {
            inner: Arc::new(framework),
        })
    }
}

impl Sealed for KratosLocalAuth {}

#[async_trait]
impl AuthProvider for KratosLocalAuth {
    async fn authenticate(&self, token: &str) -> Result<bool, AuthError> {
        let auth_token = AuthToken::new(
            token.to_string(),
            String::new(),          // access_token placeholder
            Duration::from_secs(3600),
            String::from("jwt"),
        );
        match self.inner.validate_token(&auth_token).await {
            Ok(valid) => {
                if valid {
                    tracing::debug!("KratosLocalAuth: token valid");
                } else {
                    tracing::warn!("KratosLocalAuth: token invalid");
                }
                Ok(valid)
            }
            Err(e) => Err(AuthError::Validation(e.to_string())),
        }
    }
}
