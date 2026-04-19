use serde::{Deserialize, Serialize};
use config::{Config, File, FileFormat};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStrategy {
    TTL,
    LRU,
    LFU,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancerStrategy {
    RoundRobin, // Default
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_capacity: u64,
    pub strategy: CacheStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub threshold: u32,
    pub reset_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterConfig {
    pub enabled: bool,
    pub rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub period: u64,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub strategy: LoadBalancerStrategy,
    pub circuit_breaker: CircuitBreakerConfig,
    pub rate_limiter: RateLimiterConfig,
    pub health_check: HealthCheckConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaConfig {
    pub zones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub upstreams: Vec<String>,
    #[serde(default)]
    pub upstream_tls: bool,
    #[serde(default)]
    pub upstream_sni: String,
    #[serde(default)]
    pub upstream_alpn: String,
    pub tls: TlsConfig,
    pub tcp_recv_buf: Option<usize>,
    pub tcp_send_buf: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub http_addr: String,
    pub https_addr: String,
    pub tcp_recv_buf: Option<usize>,
    pub tcp_send_buf: Option<usize>,
}

// ── Auth provider configuration ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum AuthProviderConfig {
    Noop,
    #[cfg(feature = "auth")]
    KratosLocal {
        jwt_secret: String,
        issuer: String,
    },
    #[cfg(feature = "auth")]
    KratosHttp { endpoint: String },
    #[cfg(feature = "auth")]
    OpaAgent { endpoint: String },
    #[cfg(feature = "auth")]
    AwsIam { jwks_url: String },
    #[cfg(feature = "auth")]
    AzureEntraId { tenant_id: String },
    #[cfg(feature = "auth")]
    CloudFlare { team_domain: String },
    #[cfg(feature = "auth")]
    JwtOidc { endpoint: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuthConfig {
    Provider(AuthProviderConfig),
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig::Provider(AuthProviderConfig::Noop)
    }
}

impl AuthConfig {
    pub fn into_provider_config(self) -> AuthProviderConfig {
        match self {
            AuthConfig::Provider(p) => p,
        }
    }
}

// ── Top-level AppConfig ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OtlpProtocol {
    #[default]
    Grpc,
    Http,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub otlp_endpoint: String,
    #[serde(default)]
    pub protocol: OtlpProtocol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub logging: LoggingConfig,
    pub server: ServerConfig,
    pub lb: LoadBalancerConfig,
    pub proxy: ProxyConfig,
    
    // Feature-gated modules allow None (deserializing absent blocks to None securely)
    #[serde(default)]
    pub cache: Option<CacheConfig>,
    
    #[serde(default)]
    pub ha: Option<HaConfig>,
    
    #[serde(default)]
    pub auth: Option<AuthConfig>,
    
    #[serde(default)]
    pub metrics: Option<MetricsConfig>,
}

pub fn load_config(path: Option<&str>) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let builder = Config::builder();
    
    let config = if let Some(p) = path {
        builder.add_source(File::with_name(p)).build()?
    } else {
        let toml_str = include_str!("../config.default.toml");
        builder.add_source(File::from_str(toml_str, FileFormat::Toml)).build()?
    };
    
    let app_config: AppConfig = config.try_deserialize()?;

    tracing::debug!("Loaded configuration (path: {:?})", path);
    Ok(app_config)
}
