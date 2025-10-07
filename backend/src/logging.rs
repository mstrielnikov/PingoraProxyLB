use tracing_subscriber;
use crate::config::LoggingConfig;

pub fn init_logging(config: &LoggingConfig) {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| config.level.clone())
        ))
        .init();
}
