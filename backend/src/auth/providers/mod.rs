//! Provider stubs for future AuthN/AuthZ integrations.
//!
//! Each provider is a real, compiling type that returns `Ok(false)` with a
//! `tracing::warn!` until its implementation is filled in.

pub use opa::OpaAgentAuth;
pub use aws::AwsIamAuth;
pub use azure::AzureEntraIdAuth;
pub use cloudflare::CloudFlareAuth;

// ── OPA Agent ─────────────────────────────────────────────────────────────────

mod opa {
    //! OPA (Open Policy Agent) sidecar HTTP policy check.
    //!
    //! Calls `POST <endpoint>/v1/data/allow` with the token as input.

    use async_trait::async_trait;
    use crate::auth::{AuthError, AuthProvider};
    use crate::auth::private::Sealed;

    #[derive(Clone, Debug)]
    pub struct OpaAgentAuth {
        pub endpoint: String,
    }

    impl OpaAgentAuth {
        pub fn new(endpoint: impl Into<String>) -> Self {
            Self { endpoint: endpoint.into() }
        }
    }

    impl Sealed for OpaAgentAuth {}

    #[async_trait]
    impl AuthProvider for OpaAgentAuth {
        async fn authenticate(&self, _token: &str) -> Result<bool, AuthError> {
            tracing::warn!(
                "OpaAgentAuth: not yet implemented (endpoint={}); denying request",
                self.endpoint
            );
            Err(AuthError::NotConfigured("OpaAgentAuth".into()))
        }
    }
}

// ── AWS IAM ───────────────────────────────────────────────────────────────────

mod aws {
    //! AWS IAM / Cognito token validator.
    //!
    //! TODO: fetch JWKS from Cognito User Pool, verify RS256 JWT.

    use async_trait::async_trait;
    use crate::auth::{AuthError, AuthProvider};
    use crate::auth::private::Sealed;

    #[derive(Clone, Debug, Default)]
    pub struct AwsIamAuth {
        /// Cognito User Pool JWKS URL, e.g.
        /// `https://cognito-idp.<region>.amazonaws.com/<pool-id>/.well-known/jwks.json`
        pub jwks_url: String,
    }

    impl AwsIamAuth {
        pub fn new(jwks_url: impl Into<String>) -> Self {
            Self { jwks_url: jwks_url.into() }
        }
    }

    impl Sealed for AwsIamAuth {}

    #[async_trait]
    impl AuthProvider for AwsIamAuth {
        async fn authenticate(&self, _token: &str) -> Result<bool, AuthError> {
            tracing::warn!("AwsIamAuth: not yet implemented; denying request");
            Err(AuthError::NotConfigured("AwsIamAuth".into()))
        }
    }
}

// ── Azure Entra ID ────────────────────────────────────────────────────────────

mod azure {
    //! Azure Entra ID (formerly Azure AD) JWT validator.
    //!
    //! TODO: fetch JWKS from
    //! `https://login.microsoftonline.com/<tenant-id>/discovery/v2.0/keys`
    //! and verify RS256 JWT.

    use async_trait::async_trait;
    use crate::auth::{AuthError, AuthProvider};
    use crate::auth::private::Sealed;

    #[derive(Clone, Debug)]
    pub struct AzureEntraIdAuth {
        pub tenant_id: String,
    }

    impl AzureEntraIdAuth {
        pub fn new(tenant_id: impl Into<String>) -> Self {
            Self { tenant_id: tenant_id.into() }
        }
    }

    impl Sealed for AzureEntraIdAuth {}

    #[async_trait]
    impl AuthProvider for AzureEntraIdAuth {
        async fn authenticate(&self, _token: &str) -> Result<bool, AuthError> {
            tracing::warn!(
                "AzureEntraIdAuth: not yet implemented (tenant={}); denying request",
                self.tenant_id
            );
            Err(AuthError::NotConfigured("AzureEntraIdAuth".into()))
        }
    }
}

// ── Cloudflare Access ─────────────────────────────────────────────────────────

mod cloudflare {
    //! Cloudflare Access JWT validator.
    //!
    //! TODO: fetch JWKS from
    //! `https://<team-domain>.cloudflareaccess.com/cdn-cgi/access/certs`
    //! and verify RS256 `CF-Access-Jwt-Assertion` header.

    use async_trait::async_trait;
    use crate::auth::{AuthError, AuthProvider};
    use crate::auth::private::Sealed;

    #[derive(Clone, Debug)]
    pub struct CloudFlareAuth {
        pub team_domain: String,
    }

    impl CloudFlareAuth {
        pub fn new(team_domain: impl Into<String>) -> Self {
            Self { team_domain: team_domain.into() }
        }
    }

    impl Sealed for CloudFlareAuth {}

    #[async_trait]
    impl AuthProvider for CloudFlareAuth {
        async fn authenticate(&self, _token: &str) -> Result<bool, AuthError> {
            tracing::warn!(
                "CloudFlareAuth: not yet implemented (team={}); denying request",
                self.team_domain
            );
            Err(AuthError::NotConfigured("CloudFlareAuth".into()))
        }
    }
}
