use serde::{Deserialize, Serialize};
use config::{Config, File, Environment};
use std::env;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheStrategy {
    TTL,
    LRU,
    LFU,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancerStrategy {
    RoundRobin, // Only RoundRobin supported for now
    // Future: LeastConnections, ConsistentHashing for cross-zone balancing
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_capacity: u64,
    pub strategy: CacheStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBrakerConfig {
    pub enabled: bool,
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
    pub circuitBraker: CircuitBrakerConfig,
    pub rateLimiter: RateLimiterConfig,
    pub healthCheck: HealthCheckConfig,
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
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub jwt_secret: String,
    pub auth_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt: JwtConfig,
    // oidc: Option<OidcConfig>, // Future: OIDC configuration
    // mtls: Option<MtlsConfig>, // Future: mTLS configuration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub otlp_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub logging: LoggingConfig,
    pub cache: CacheConfig,
    pub lb: LoadBalancerConfig,
    pub ha: HaConfig,
    pub proxy: ProxyConfig,
    pub auth: AuthConfig,
    pub metrics: MetricsConfig,
}

pub fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    // Ensure config.default.yaml exists
    if !Path::new("config.default.yaml").exists() {
        return Err("Mandatory config.default.yaml file not found".into());
    }

    // Load default config and optional env-specific overrides
    let app_env = env::var("APP_ENV").unwrap_or_default();
    let mut builder = Config::builder()
        .add_source(File::with_name("config.default.yaml"));

    // Add environment-specific config if APP_ENV is set
    if !app_env.is_empty() {
        let env_config = format!("config.{}.yaml", app_env);
        if Path::new(&env_config).exists() {
            builder = builder.add_source(File::with_name(&env_config).required(false));
        } else {
            tracing::warn!("Environment-specific config {} not found, using defaults", env_config);
        }
    }

    // Add environment variable overrides
    let config = builder
        .add_source(Environment::with_prefix("APP").separator("__"))
        .build()?
        .try_deserialize::<AppConfig>()?;

    tracing::debug!("Loaded configuration: {:?}", config);
    Ok(config)
}
