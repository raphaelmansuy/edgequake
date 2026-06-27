//! EdgeQuake observability — single init point for tracing, metrics, and correlation.
//!
//! ## SOLID
//! - **S**: This crate only configures observability; handlers use `tracing` macros.
//! - **D**: Application code depends on `RequestContext`, not OTEL types.

pub mod error_context;
pub mod http_span;
pub mod propagation;
pub mod request_context;
pub mod subscriber;

#[cfg(feature = "otel")]
pub mod trace_context;

pub mod query_guard;

#[cfg(feature = "metrics")]
pub mod metrics;

pub use error_context::ErrorEvent;
pub use http_span::{record_http_error, record_http_status, with_http_span};
pub use query_guard::{QueryFailureGuard, QueryOutcomeGuard};

pub use propagation::{harvest_propagation_headers, PropagationHeaders};
pub use request_context::{
    current_llm_provider, current_request_id, parse_trace_id_from_traceparent, resolve_request_id,
    scope_llm_provider, scope_request_id, synthesize_traceparent_from_request_id,
    trace_id_from_request_id, RequestContext, CORRELATION_ID_HEADER, REQUEST_ID_HEADER,
    TRACEPARENT_HEADER, TRACESTATE_HEADER,
};
pub use subscriber::{
    init_observability, log_format_label, LogFormat, ObservabilityConfig, ObservabilityGuard,
};
#[cfg(feature = "otel")]
pub use trace_context::{extract_from_headers, inject_current_context};

#[cfg(feature = "metrics")]
pub use metrics::{
    init_metrics, record_db_pool_stats, record_document_processing,
    record_document_processing_with_labels, record_http_request, record_llm_request,
    record_pipeline_error, record_query_completed, record_rate_limit_exceeded,
    record_storage_error, record_task_queue_stats, render_prometheus_metrics,
};
