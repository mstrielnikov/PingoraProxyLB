#[cfg(feature = "telemetry")]
use tracing_subscriber;
use crate::config::LoggingConfig;

pub fn init_logging(_config: &LoggingConfig) {
    #[cfg(feature = "telemetry")]
    {
        let env_filter = tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| _config.level.clone())
        );

        if std::env::var("APP_ENV").unwrap_or_default() == "production" {
            let file_appender = tracing_appender::rolling::daily("/var/log/chainless-lb", "lb.log");
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
            // In a real application, the `_guard` must be kept alive globally
            // For now, we instantiate the non_blocking writer
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(non_blocking)
                .with_ansi(false)
                .init();
        } else {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .init();
        }
    }
}
