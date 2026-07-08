//! Services module for shared business logic.
//!
//! WHY-OODA83: Extracted services follow SRP and DRY principles.
//! Consolidates repeated logic into single, testable modules.

pub mod artifact_retrieval;
pub mod audit;
#[cfg(feature = "postgres")]
pub mod auth_bootstrap;
pub mod auth_memory_store;
pub mod auth_validation;
pub mod content_granularity;
pub mod content_hasher;
pub mod context_bundle_mapper;
pub mod cost_aggregation;
pub mod document_body_loader;
pub mod document_graph_cascade;
pub mod document_metadata_scan;
pub mod document_reingest;
pub mod document_task_cleanup;
pub mod document_vector_storage;
pub mod entity_graph_lookup;
pub mod entity_merge;
pub mod entity_name_normalize;
pub mod entity_neighborhood;
pub mod graph_community;
pub mod graph_materialization;
#[cfg(feature = "postgres")]
pub mod health_schema;
pub mod identity_storage;
pub mod ingest_admission;
pub mod ingestion_persist;
pub mod injection_list;
pub mod injection_process;
pub mod isolation_context;
pub mod job_registry;
pub mod large_document_profile;
pub mod list_pagination;
pub mod login_lockout;
pub mod message_context_mapper;
pub mod multimodal;
pub mod multimodal_admission;
pub mod multimodal_context;
pub mod multimodal_markdown;
pub mod oidc_flow;
pub mod oidc_pending;
pub mod pdf_admission_registry;
pub mod pdf_auto_routing;
pub mod pdf_lineage;
pub mod pdf_workspace_dedup;
pub mod query_context;
pub mod query_execution;
pub mod query_generation;
pub mod query_request_builder;
pub mod retrieval_id_cache;
pub mod route_registry;
pub mod session_storage;
pub mod source_reference_builder;
pub mod staging_admission;
pub mod task_scope;
pub mod tenant_guard;
pub mod tenant_isolation;
pub mod text_insert_content;
pub mod v1_rpc_migration;
pub mod vision_content;
pub mod vlm_limits;
pub mod vlm_provider_resolver;
pub mod workspace_content_hash_dedup;
pub mod workspace_document_index;

pub use audit::{
    record_audit, record_compliance_event, record_compliance_event_runtime, with_request_context,
};

pub use crate::handlers::documents::upload::document_admission::{
    admit_document_for_processing, chunk_fields_from_metadata, parse_upload_chunk_fields,
    DocumentAdmissionAccepted, DocumentAdmissionDuplicateProcessing, DocumentAdmissionInput,
    DocumentAdmissionOutcome, GleaningAdmissionOptions,
};
pub use content_granularity::{
    ensure_debug_granularity_allowed, truncate_for_granularity, SNIPPET_LEN,
};
pub use content_hasher::ContentHasher;
pub use document_graph_cascade::{
    analyze_deletion_impact_stats, cascade_remove_document_sources, cleanup_document_graph_data,
    find_document_edges, find_document_nodes, sources_for_document, CascadeStats, CleanupStats,
    DocumentSourceScope,
};
pub use document_reingest::{
    delete_document_for_reingestion, resolve_workspace_duplicate_for_reingestion,
    DuplicateReingestAction,
};
pub use document_vector_storage::{
    get_workspace_vector_storage_for_delete, get_workspace_vector_storage_strict,
    get_workspace_vector_storage_with_fallback,
};
pub use entity_graph_lookup::{entity_lookup_candidates, lookup_entity_node_for_context};
pub use graph_community::detect_communities_guarded;
pub use graph_materialization::{
    admit_graph_materialization, graph_query_timeout, run_timed_graph_query,
    GraphMaterializationGuard,
};
pub use ingest_admission::{
    admit_pdf_processing_enqueue, persist_pdf_task_document_id,
    provision_queued_pdf_document_shell, resolve_pdf_ingest_document_id,
    resolve_worker_pdf_document_id, QueuedPdfDocumentShell, WorkerPdfDocumentIdRequest,
};
pub use ingestion_persist::{
    build_chunk_kv_records, persist_ingestion_result, persist_with_providers,
    persist_with_providers_and_progress, resolve_relational_sink, tag_injection_sources,
    PersistIngestionParams,
};
pub use injection_list::{
    list_injections_paged, summary_from_meta, InjectionListPage, DEFAULT_INJECTION_LIST_LIMIT,
    MAX_INJECTION_LIST_LIMIT,
};
pub use injection_process::{
    build_injection_metadata, injection_doc_id, injection_list_prefix, injection_meta_key,
    run_injection_pipeline, write_injection_status,
};
pub use large_document_profile::{
    classify_ingestion_failure, IngestionEstimate, IngestionFailureClass, LargeDocumentProfile,
};
pub use message_context_mapper::{
    build_message_context_from_engine, message_context_from_subgraph,
};
pub use multimodal::{
    analysis_cache_enabled, analyze_multimodal_images, analyze_standalone_image,
    append_mm_chunks_to_text, apply_process_options_to_metadata, build_mm_chunks_from_manifest,
    build_surrounding, collect_mm_chunks_from_manifest, enrich_markdown_with_vlm,
    enrich_processed_text_with_mm_chunks, extract_json_object, find_target_span,
    load_chunk_separators, load_content_rows_by_blockid_jsonl, load_manifest, load_mm_chunks,
    manifest_item_status_views, manifest_key, maybe_attach_cache_key, metadata_multimodal_patch,
    mm_chunks_enabled, mm_chunks_key, parse_json_object, persist_manifest, persist_mm_chunks,
    reanalyze_document_multimodal, render_mm_chunk, resolve_process_options_from_metadata,
    run_multimodal_analyze_stage, run_multimodal_analyze_stage_outcome, scan_manifest_items,
    should_run_image_analysis, summary_from_metadata, table_analysis_messages, vlm_process_enabled,
    AnalyzeOutcome, ManifestItem, MmChunkBuildError, MultimodalChunk, MultimodalHeading,
    MultimodalItemRecord, MultimodalItemStatusView, MultimodalManifest, MultimodalProviders,
    MultimodalReanalyzeOutcome, MultimodalReanalyzeParams, MultimodalSummary, PromptContext,
    SurroundingContext, SurroundingKind, SurroundingTokenCounter, METADATA_FIELD,
};
pub use multimodal_admission::{
    resolve_upload_content, MultimodalAdmissionMeta, ResolvedUploadContent,
};
pub use pdf_admission_registry::PdfAdmissionRegistry;
pub use pdf_auto_routing::{should_try_edgeparse_before_vision, try_edgeparse_fast_path};
pub use pdf_workspace_dedup::{
    find_kv_document_id_for_pdf, recycle_orphan_workspace_pdf,
    workspace_has_visible_document_for_pdf,
};
pub use query_context::{
    build_legacy_query_response, build_legacy_query_sources, fetch_context_by_id, retrieve_context,
    search_context, FetchContextOptions,
};
pub use query_execution::{
    execute_sota_query, execute_sota_query_stream, execute_sota_query_stream_with_auth_fallback,
    execute_sota_query_with_auth_fallback, is_llm_auth_failure, llm_override_from_request,
    resolve_workspace_query_resources, validate_llm_override_pair, WorkspaceQueryResources,
};
pub use query_generation::{execute_full_query, execute_legacy_query_response};
pub use query_request_builder::{build_engine_request, QueryExecutionParams};
pub use retrieval_id_cache::{global_retrieval_cache, new_retrieval_id, RetrievalIdCache};
pub use source_reference_builder::{build_sources_from_context, is_injection_source};
pub use staging_admission::{promote_staging_to_final, rollback_staging};
pub use text_insert_content::{
    patch_document_metadata, resolve_document_metadata_key, resolve_text_insert_content,
};
pub use vision_content::{
    describe_image, describe_image_as_markdown, image_analysis_to_markdown,
    parse_image_analysis_json, ImageAnalysisResult, MultimodalProcessOptions, IMAGE_TYPE_FALLBACK,
};
pub use vlm_provider_resolver::{
    resolve_extract_provider_for_workspace, resolve_vlm_provider,
    resolve_vlm_provider_for_workspace, resolve_workspace_vlm_config,
};
pub use workspace_content_hash_dedup::{
    recycle_orphan_workspace_hash, workspace_has_visible_document_for_hash,
};
pub use workspace_document_index::{
    list_workspace_document_ids, list_workspace_metadata_keys, remove_workspace_document_index,
    sync_after_metadata_upsert, sync_workspace_document_index, upsert_final_document_metadata,
    upsert_metadata_kv_with_index,
};
