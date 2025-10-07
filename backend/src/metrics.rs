use opentelemetry::metrics::{Counter, MeterProvider};
use opentelemetry_sdk::metrics::{Aggregation, MeterProvider, SdkMeterProvider, View};
use opentelemetry_sdk::{runtime, Resource};
use opentelemetry::{KeyValue};
use opentelemetry_sdk::Resource;
use opentelemetry::KeyValue;
use opentelemetry_otlp::HttpExporterBuilder;
use std::time::Duration;
use crate::config::MetricsConfig;

pub struct Metrics {
    pub request_counter: Option<Counter<u64>>,
    pub cache_hits: Option<Counter<u64>>,
    pub cache_misses: Option<Counter<u64>>,
}

impl Metrics {
    pub fn new(config: &MetricsConfig) -> Self {
        let meter_provider = if config.enabled {
            // Build OTLP HTTP exporter
            let exporter = HttpExporterBuilder::new()
                .with_endpoint(&config.otlp_endpoint)
                .with_timeout(Duration::from_secs(3))
                .build_metrics_exporter()
                .map_err(|e| {
                    tracing::error!("Failed to build OTLP exporter: {}", e);
                    None
                })
                .unwrap_or(None);

            if exporter.is_none() {
                tracing::warn!("Metrics disabled due to exporter initialization failure");
                None
            } else {
                let exporter = exporter.unwrap();

                // Custom temporality selector (default: Cumulative)
                struct SimpleTemporalitySelector;
                impl TemporalitySelector for SimpleTemporalitySelector {
                    fn temporality(&self, _kind: InstrumentKind) -> Temporality {
                        Temporality::Cumulative
                    }
                }

                // Custom aggregation selector
                struct SimpleAggregationSelector;
                impl AggregationSelector for SimpleAggregationSelector {
                    // Aggregation is the type imported from opentelemetry_sdk::metrics::data
                    fn aggregation(&self, kind: InstrumentKind) -> Aggregation {
                        match kind {
                            InstrumentKind::Counter
                            | InstrumentKind::UpDownCounter
                            | InstrumentKind::ObservableCounter
                            | InstrumentKind::ObservableUpDownCounter => {
                                Sum {
                                    temporality: Temporality::Cumulative,
                                    monotonic: true,
                                }.into()
                            }
                            InstrumentKind::Gauge | InstrumentKind::ObservableGauge => {
                                LastValue {}.into()
                            }
                            InstrumentKind::Histogram => {
                                ExplicitBucketHistogram {
                                    boundaries: vec![0.0, 5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 250.0, 500.0, 1000.0],
                                    record_min_max: true,
                                }.into()
                            }
                        }
                    }
                }

                let temporality_selector = Box::new(SimpleTemporalitySelector);
                let aggregation_selector = Box::new(SimpleAggregationSelector);
                let reader = PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio)
                    .build();
                let resource = Resource::new(vec![
                    KeyValue::new("service.name", "chainless-lb-backend"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    KeyValue::new("environment", std::env::var("APP_ENV").unwrap_or("production".to_string())),
                ]);
                Some(
                    SdkMeterProvider::builder()
                        .with_reader(reader)
                        .with_temporality_selector(temporality_selector)
                        .with_aggregation_selector(aggregation_selector)
                        .with_resource(resource)
                        .build(),
                )
            }
        } else {
            None
        };

        let meter = meter_provider.as_ref().map(|p| p.meter("chainless-lb-backend"));
        let request_counter = meter.as_ref().map(|m| {
            m.u64_counter("http_requests_total")
                .with_description("Total HTTP requests")
                .init()
        });
        let cache_hits = meter.as_ref().map(|m| {
            m.u64_counter("cache_hits_total")
                .with_description("Total cache hits")
                .init()
        });
        let cache_misses = meter.as_ref().map(|m| {
            m.u64_counter("cache_misses_total")
                .with_description("Total cache misses")
                .init()
        });

        Self {
            request_counter,
            cache_hits,
            cache_misses,
        }
    }
}