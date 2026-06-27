//! Tracing subscriber initialization (JSON/plain + optional OTLP).

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::Layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Configuration for observability bootstrap.
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub log_format: LogFormat,
    pub default_filter: String,
    pub otel_enabled: bool,
    pub service_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Plain,
    Json,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_format: LogFormat::Plain,
            default_filter: "edgequake=info,edgequake_api=info,edgequake_query=info,edgequake_pipeline=info,edgequake_storage=warn,tower_http=warn,sqlx=warn".into(),
            otel_enabled: false,
            service_name: "edgequake-api".into(),
        }
    }
}

impl ObservabilityConfig {
    pub fn from_env() -> Self {
        let log_format = match std::env::var("EDGEQUAKE_LOG_FORMAT")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "json" => LogFormat::Json,
            _ => LogFormat::Plain,
        };

        let otel_enabled = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
            || std::env::var("EDGEQUAKE_OTEL_ENABLED")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

        let service_name =
            std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "edgequake-api".to_string());

        let default_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
            "edgequake=info,edgequake_api=info,edgequake_query=info,edgequake_pipeline=info,edgequake_storage=warn,tower_http=warn,sqlx=warn".into()
        });

        Self {
            log_format,
            default_filter,
            otel_enabled,
            service_name,
        }
    }
}

/// Guard that shuts down OTEL providers on drop.
pub struct ObservabilityGuard {
    #[cfg(feature = "otel")]
    tracer_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl ObservabilityGuard {
    #[cfg(feature = "otel")]
    fn shutdown(&mut self) {
        if let Some(provider) = self.tracer_provider.take() {
            if let Err(e) = provider.shutdown() {
                eprintln!("OTEL tracer shutdown error: {e}");
            }
        }
    }
}

impl Drop for ObservabilityGuard {
    fn drop(&mut self) {
        #[cfg(feature = "otel")]
        self.shutdown();
    }
}

/// Initialize global tracing subscriber. Call once at process start.
pub fn init_observability(config: ObservabilityConfig) -> ObservabilityGuard {
    warn_on_otel_misconfiguration(&config);

    #[cfg(feature = "metrics")]
    crate::metrics::init_metrics();

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.default_filter));

    #[cfg(feature = "otel")]
    let (tracer_provider, otel_layer) = if config.otel_enabled {
        init_otel_layers(&config.service_name)
    } else {
        (None, None)
    };

    let log_span_events = std::env::var("EDGEQUAKE_LOG_SPAN_EVENTS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let span_events = if log_span_events {
        FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    // Layer order: OTLP bridge on registry first, then filter → stdout.
    #[cfg(feature = "otel")]
    {
        match (config.log_format, otel_layer) {
            (LogFormat::Json, Some(otel)) => {
                tracing_subscriber::registry()
                    .with(otel)
                    .with(env_filter)
                    .with(json_log_layer(span_events).boxed())
                    .init();
            }
            (LogFormat::Json, None) => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(json_log_layer(span_events).boxed())
                    .init();
            }
            (LogFormat::Plain, Some(otel)) => {
                tracing_subscriber::registry()
                    .with(otel)
                    .with(env_filter)
                    .with(plain_log_layer(span_events).boxed())
                    .init();
            }
            (LogFormat::Plain, None) => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(plain_log_layer(span_events).boxed())
                    .init();
            }
        }
    }

    #[cfg(not(feature = "otel"))]
    match config.log_format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(json_log_layer(span_events).boxed())
                .init();
        }
        LogFormat::Plain => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(plain_log_layer(span_events).boxed())
                .init();
        }
    }

    let otel_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|v| !v.trim().is_empty());
    tracing::info!(
        log_format = ?config.log_format,
        otel_enabled = config.otel_enabled,
        otel_endpoint = ?otel_endpoint,
        service_name = %config.service_name,
        log_span_events = log_span_events,
        "Observability initialized"
    );

    ObservabilityGuard {
        #[cfg(feature = "otel")]
        tracer_provider,
    }
}

#[cfg(feature = "otel")]
type OtelLayer = tracing_opentelemetry::OpenTelemetryLayer<
    tracing_subscriber::Registry,
    opentelemetry_sdk::trace::SdkTracer,
>;

#[cfg(feature = "otel")]
fn init_otel_layers(
    service_name: &str,
) -> (
    Option<opentelemetry_sdk::trace::SdkTracerProvider>,
    Option<OtelLayer>,
) {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();
    let mut builder = opentelemetry_otlp::SpanExporter::builder().with_tonic();
    if let Some(ref ep) = endpoint {
        builder = builder.with_endpoint(ep.clone());
    }

    let exporter = match builder.build() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("OTEL exporter build failed: {e}; continuing without OTLP");
            return (None, None);
        }
    };

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", service_name.to_string()),
            KeyValue::new(
                "service.version",
                std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".into()),
            ),
        ])
        .build();

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(service_name.to_string());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    (Some(provider), Some(otel_layer))
}

fn json_log_layer<S>(span_events: FmtSpan) -> impl Layer<S> + Send + Sync + 'static
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_span_events(span_events)
}

fn plain_log_layer<S>(span_events: FmtSpan) -> impl Layer<S> + Send + Sync + 'static
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(false)
        .with_span_events(span_events)
}

/// Resolve the active log format label for operator dashboards (`"plain"` | `"json"`).
pub fn log_format_label(format: LogFormat) -> &'static str {
    match format {
        LogFormat::Plain => "plain",
        LogFormat::Json => "json",
    }
}

fn warn_on_otel_misconfiguration(config: &ObservabilityConfig) {
    let endpoint_set = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    #[cfg(not(feature = "otel"))]
    if endpoint_set || config.otel_enabled {
        eprintln!(
            "WARNING: OTLP export requested (OTEL_EXPORTER_OTLP_ENDPOINT or EDGEQUAKE_OTEL_ENABLED) \
             but this binary was built without the `otel` feature — traces will not leave the process. \
             Rebuild with: cargo build -p edgequake --features otel"
        );
    }

    #[cfg(feature = "otel")]
    if config.otel_enabled && !endpoint_set {
        eprintln!(
            "WARNING: EDGEQUAKE_OTEL_ENABLED is set but OTEL_EXPORTER_OTLP_ENDPOINT is empty — \
             OTLP exporter will use library defaults (may fail to connect)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_env_var<F: FnOnce()>(key: &str, value: Option<&str>, f: F) {
        let previous = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        f();
        match previous {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn contract_edgequake_log_format_from_env() {
        with_env_var("EDGEQUAKE_LOG_FORMAT", Some("json"), || {
            let cfg = ObservabilityConfig::from_env();
            assert_eq!(cfg.log_format, LogFormat::Json);
            assert_eq!(log_format_label(cfg.log_format), "json");
        });
        with_env_var("EDGEQUAKE_LOG_FORMAT", Some("plain"), || {
            let cfg = ObservabilityConfig::from_env();
            assert_eq!(cfg.log_format, LogFormat::Plain);
        });
        with_env_var("EDGEQUAKE_LOG_FORMAT", None, || {
            let cfg = ObservabilityConfig::from_env();
            assert_eq!(cfg.log_format, LogFormat::Plain);
            assert_eq!(log_format_label(cfg.log_format), "plain");
        });
    }
}
