//! EdgeQuake observability — single init point for tracing, metrics, and correlation.
//!
//! ## SOLID
//! - **S**: This crate only configures observability; handlers use `tracing` macros.
//! - **D**: Application code depends on `RequestContext`, not OTEL types.

pub mod error_context;
pub mod http_span;
pub mod io_policy;
pub mod langfuse;
pub mod langfuse_attrs;
pub mod langfuse_context;
#[cfg(feature = "otel")]
pub mod langfuse_ingestion;
pub mod langfuse_meta;
pub mod propagation;
pub mod rag_span;
pub mod request_context;
pub mod subscriber;
pub mod utf8_truncate;

#[cfg(feature = "otel")]
pub mod baggage_span_processor;
#[cfg(feature = "otel")]
pub mod trace_context;

#[cfg(all(test, feature = "otel"))]
mod inmemory_otel_tests;
#[cfg(all(test, feature = "otel"))]
mod langfuse_otlp_e2e;

pub mod query_guard;

#[cfg(feature = "metrics")]
pub mod metrics;

pub use error_context::ErrorEvent;
pub use http_span::{record_http_error, record_http_status, with_http_span};
pub use io_policy::{
    format_llm_chat_turns_for_observation, langfuse_io_max_bytes, prepare_complete_io,
    prepare_observation_io, preview_bytes, redact_secrets, IoPolicy, PreparedIo,
    DEFAULT_LANGFUSE_IO_MAX_BYTES, INGEST_CONTENT_PREVIEW_BYTES, MARKER_TAIL_COMPLETE,
    OBSERVATION_IO_PREVIEW_CHARS, RETRIEVAL_PREVIEW_BYTES,
};
pub use query_guard::{QueryFailureGuard, QueryOutcomeGuard};
pub use rag_span::{
    enter_pipeline_stage, generation_span, instrument_generation_token_stream, query_preview,
    record_embedding_io, record_feature_tag, record_gen_ai_usage, record_ingest_document_input,
    record_ingest_document_output, record_observation_io, record_observation_io_with_policy,
    record_observation_type_span, record_pipeline_chunk_extraction_io, record_query_root_io,
    record_rag_retrieval_complete, record_rag_retrieval_io, record_rag_retrieval_outcome,
    record_structured_io, stamp_ingest_langfuse, stamp_query_langfuse,
    stamp_query_langfuse_identity, with_feature_root_span, with_ingest_document_span,
    with_ingest_task_span, with_llm_generation, with_pipeline_stage_span, with_rag_embedding_span,
    with_rag_generation_span, with_rag_retrieval_span, GenerationIoStream, LlmGenerationRecord,
    RagRetrievalAttrs,
};
pub use utf8_truncate::{
    utf8_clamp_span, utf8_prefix, utf8_prefix_at_sentence, utf8_prefix_ellipsis,
};

#[cfg(feature = "otel")]
pub use langfuse::probe_langfuse_api;
pub use langfuse::{
    basic_auth_token, langfuse_otlp_headers, langfuse_otlp_headers_from_env, normalize_base_url,
    record_resolved_langfuse_api, resolved_langfuse_api, unquote_env_value, LangfuseApi,
    LangfuseConfig, LangfuseConfigRequirement, DEFAULT_LANGFUSE_BASE_URL,
};
pub use langfuse_attrs::{
    is_forbidden_cost_attr, LangfuseTraceIdentity, COST_ATTR_DENYLIST, GEN_AI_COMPLETION,
    GEN_AI_CONVERSATION_ID, GEN_AI_PROMPT, GEN_AI_USAGE_INPUT_TOKENS, GEN_AI_USAGE_OUTPUT_TOKENS,
    LANGFUSE_BAGGAGE_ALLOWLIST, LANGFUSE_META_TENANT_ID, LANGFUSE_META_TENANT_SLUG,
    LANGFUSE_META_WORKSPACE_ID, LANGFUSE_META_WORKSPACE_SLUG, LANGFUSE_OBSERVATION_INPUT,
    LANGFUSE_OBSERVATION_METADATA_PREFIX, LANGFUSE_OBSERVATION_OUTPUT, LANGFUSE_OBSERVATION_TYPE,
    LANGFUSE_SESSION_ID, LANGFUSE_TRACE_METADATA_PREFIX, LANGFUSE_TRACE_TAGS, LANGFUSE_USER_ID,
    OBSERVATION_TYPE_CHAIN, OBSERVATION_TYPE_EMBEDDING, OBSERVATION_TYPE_GENERATION,
    OBSERVATION_TYPE_RETRIEVER, OBSERVATION_TYPE_SPAN, SESSION_ID, USER_ID,
};
pub use langfuse_context::{
    bind_langfuse_identity, bind_langfuse_trace_identity, bind_langfuse_trace_identity_async,
    with_langfuse_identity_async, LangfuseIdentityGuard,
};
pub use langfuse_meta::{
    record_ingest_kg_meta, record_ingest_parse_meta, record_observation_meta,
    record_query_pipeline_meta, record_trace_meta, IngestKgMeta, IngestParseMeta,
    QueryPipelineMeta,
};
pub use propagation::{harvest_propagation_headers, PropagationHeaders};
pub use request_context::{
    current_llm_provider, current_request_id, parse_trace_id_from_traceparent, resolve_request_id,
    scope_llm_provider, scope_request_id, synthesize_traceparent_from_request_id,
    trace_id_from_request_id, RequestContext, CORRELATION_ID_HEADER, REQUEST_ID_HEADER,
    TRACEPARENT_HEADER, TRACESTATE_HEADER,
};
pub use subscriber::{
    init_observability, log_format_label, otel_feature_built, LogFormat, ObservabilityConfig,
    ObservabilityGuard,
};
#[cfg(feature = "otel")]
pub use trace_context::{extract_from_headers, inject_current_context};

#[cfg(feature = "metrics")]
pub use metrics::{
    init_metrics, record_chunk_strategy_degraded, record_citation_rewrite,
    record_community_sampled, record_compensate_shared_entity_skipped,
    record_compensation_quarantine, record_db_pool_stats, record_db_pool_stats_for_role,
    record_document_processing, record_document_processing_with_labels, record_faithfulness_sample,
    record_graph_quality, record_http_request, record_ingest_stage_duration,
    record_ingestion_failure, record_llm_request, record_migration_progress,
    record_page_layout_persist_error, record_page_layout_persist_skipped,
    record_page_layout_persisted, record_pipeline_error, record_popular_node_fallback,
    record_query_arm_duration, record_query_completed, record_rate_limit_exceeded,
    record_retract_on_cancel, record_sparse_retrieval_outcome, record_storage_drift,
    record_storage_error, record_storage_op_duration, record_task_queue_stats,
    record_vector_dim_mismatch_rejected, render_prometheus_metrics, set_storage_drift_critical,
    set_vector_ann_index_missing,
};
