//! No-op authentication provider — every request is unconditionally allowed.
//!
//! Use this in development / internal networks where authentication is handled
//! by an upstream service, or as the default when auth is disabled.

use async_trait::async_trait;
use super::{AuthError, AuthProvider};
use super::private::Sealed;

/// Zero-cost passthrough provider.
#[derive(Clone, Debug, Default)]
pub struct NoOpAuth;

impl Sealed for NoOpAuth {}

#[async_trait]
impl AuthProvider for NoOpAuth {
    #[inline]
    async fn authenticate(&self, _token: &str) -> Result<bool, AuthError> {
        tracing::trace!("NoOpAuth: unconditionally allowing request");
        Ok(true)
    }
}
