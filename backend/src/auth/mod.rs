//! AuthN/AuthZ provider abstraction.
//!
//! Each provider is a first-class type implementing [`AuthProvider`].
//! Select the provider at startup via config and store it as a generic
//! parameter on [`crate::proxy::LB`].
//!
//! # Provider matrix
//!
//! | Type               | Status      | Notes                                     |
//! |--------------------|-------------|-------------------------------------------|
//! | [`NoOpAuth`]       | ✅ Ready    | Passthrough — every token passes          |
//! | [`KratosLocalAuth`]| ✅ Ready    | Local JWT validation via `auth-framework` |
//! | [`KratosHttpAuth`] | ✅ Ready    | ORY Kratos `/sessions/whoami` HTTP check  |
//! | [`OpaAgentAuth`]   | 🚧 Stub    | OPA `/v1/data/allow` policy check         |
//! | [`AwsIamAuth`]     | 🚧 Stub    | AWS IAM / Cognito token validation        |
//! | [`AzureEntraIdAuth`]| 🚧 Stub   | Azure Entra ID JWKS + JWT verify          |
//! | [`CloudFlareAuth`] | 🚧 Stub    | Cloudflare Access JWT verify              |

#[cfg(feature = "auth")]
pub mod kratos_http;
#[cfg(feature = "auth")]
pub mod kratos_local;
pub mod noop;
#[cfg(feature = "auth")]
pub mod providers;
pub mod typestate;
#[cfg(feature = "auth")]
pub mod jwt;

use async_trait::async_trait;
use thiserror::Error;
use std::sync::Arc;

#[cfg(feature = "auth")]
pub use kratos_http::KratosHttpAuth;
#[cfg(feature = "auth")]
pub use kratos_local::KratosLocalAuth;
pub use noop::NoOpAuth;
#[cfg(feature = "auth")]
pub use providers::{AwsIamAuth, AzureEntraIdAuth, CloudFlareAuth, OpaAgentAuth};
#[cfg(feature = "auth")]
pub use jwt::JwtOidcProvider;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("token validation failed: {0}")]
    Validation(String),
    #[error("provider communication error: {0}")]
    Transport(String),
    #[error("provider not configured: {0}")]
    NotConfigured(String),
}

// ── Sealed trait ──────────────────────────────────────────────────────────────

pub(crate) mod private {
    use std::sync::Arc;
    pub trait Sealed {}
    impl Sealed for Arc<dyn super::AuthProvider> {}
}

// ── AuthProvider ──────────────────────────────────────────────────────────────

/// Core contract every authentication/authorization provider must satisfy.
///
/// - `Ok(true)`  → allow the request
/// - `Ok(false)` → deny (HTTP 401)
/// - `Err(_)`    → hard provider failure (HTTP 502)
#[async_trait]
pub trait AuthProvider: private::Sealed + Send + Sync + 'static {
    async fn authenticate(&self, token: &str) -> Result<bool, AuthError>;
}

#[async_trait]
impl AuthProvider for Arc<dyn AuthProvider> {
    async fn authenticate(&self, token: &str) -> Result<bool, AuthError> {
        (**self).authenticate(token).await
    }
}
