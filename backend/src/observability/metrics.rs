#[cfg(feature = "telemetry")]
use opentelemetry::metrics::{Counter, Meter, MeterProvider};
#[cfg(feature = "telemetry")]
use opentelemetry_sdk::metrics::{SdkMeterProvider, PeriodicReader};
#[cfg(feature = "telemetry")]
use opentelemetry_sdk::Resource;
#[cfg(feature = "telemetry")]
use opentelemetry::KeyValue;
#[cfg(feature = "telemetry")]
use opentelemetry_otlp::{MetricExporter, WithExportConfig};
#[cfg(feature = "telemetry")]
use std::time::Duration;
use crate::config::MetricsConfig;

pub struct Metrics {
    #[cfg(feature = "telemetry")]
    pub request_counter: Option<Counter<u64>>,
    #[cfg(feature = "telemetry")]
    pub cache_hits: Option<Counter<u64>>,
    #[cfg(feature = "telemetry")]
    pub cache_misses: Option<Counter<u64>>,
}

impl Metrics {
    pub fn new(_config: Option<&MetricsConfig>) -> Self {
        #[cfg(feature = "telemetry")]
        {
        let meter_provider: Option<SdkMeterProvider> = if let Some(cfg) = _config {
            if cfg.enabled {
                let build_http = || {
                    MetricExporter::builder()
                        .with_http()
                        .with_endpoint(&cfg.otlp_endpoint)
                        .with_timeout(Duration::from_secs(3))
                        .build()
                };

                let build_grpc = || {
                    MetricExporter::builder()
                        .with_tonic()
                        .with_endpoint(&cfg.otlp_endpoint)
                        .with_timeout(Duration::from_secs(3))
                        .build()
                };

                let mut provider_builder = SdkMeterProvider::builder()
                    .with_resource(Resource::builder()
                        .with_attribute(KeyValue::new("service.name", "chainless-lb-backend"))
                        .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
                        .with_attribute(KeyValue::new(
                            "environment",
                            std::env::var("APP_ENV").unwrap_or_else(|_| "production".into()),
                        ))
                        .build());

                use crate::config::OtlpProtocol;
                
                let (use_http, use_grpc) = match cfg.protocol {
                    OtlpProtocol::Http => (true, false),
                    OtlpProtocol::Grpc => (false, true),
                    OtlpProtocol::Both => (true, true),
                };

                if use_http {
                    if let Ok(exp) = build_http() {
                        provider_builder = provider_builder.with_reader(PeriodicReader::builder(exp).build());
                    } else {
                        tracing::warn!("Failed to init OTLP HTTP exporter");
                    }
                }

                if use_grpc {
                    if let Ok(exp) = build_grpc() {
                        provider_builder = provider_builder.with_reader(PeriodicReader::builder(exp).build());
                    } else {
                        tracing::warn!("Failed to init OTLP GRPC exporter");
                    }
                }

                Some(provider_builder.build())
            } else {
                None
            }
        } else {
            None
        };

        let meter = meter_provider
            .as_ref()
            .map(|p: &SdkMeterProvider| p.meter("chainless-lb-backend"));

        let request_counter = meter.as_ref().map(|m: &Meter| {
            m.u64_counter("http_requests_total")
                .with_description("Total HTTP requests")
                .build()
        });
        let cache_hits = meter.as_ref().map(|m: &Meter| {
            m.u64_counter("cache_hits_total")
                .with_description("Total cache hits")
                .build()
        });
        let cache_misses = meter.as_ref().map(|m: &Meter| {
            m.u64_counter("cache_misses_total")
                .with_description("Total cache misses")
                .build()
        });

        Self { request_counter, cache_hits, cache_misses }
        }
        #[cfg(not(feature = "telemetry"))]
        {
            Self {}
        }
    }

    pub fn record_request(&self) {
        #[cfg(feature = "telemetry")]
        if let Some(counter) = &self.request_counter {
            counter.add(1, &[]);
        }
    }

    pub fn record_cache_hit(&self) {
        #[cfg(feature = "telemetry")]
        if let Some(counter) = &self.cache_hits {
            counter.add(1, &[]);
        }
    }

    pub fn record_cache_miss(&self) {
        #[cfg(feature = "telemetry")]
        if let Some(counter) = &self.cache_misses {
            counter.add(1, &[]);
        }
    }
}