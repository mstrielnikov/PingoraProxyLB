use async_trait::async_trait;
use crate::auth::{AuthProvider, AuthError};
use std::sync::Arc;

pub struct JwtOidcProvider {
    pub endpoint: String,
    // Would typically hold a jwks cache and client
}

impl JwtOidcProvider {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }
}

impl crate::auth::private::Sealed for JwtOidcProvider {}

#[async_trait]
impl AuthProvider for JwtOidcProvider {
    async fn authenticate(&self, _token: &str) -> Result<bool, AuthError> {
        // Mock OIDC / Ory Kratos integration
        tracing::debug!("Validating JWT via OIDC/Kratos at {}", self.endpoint);
        Ok(true) // accept for now as a mock
    }
}
