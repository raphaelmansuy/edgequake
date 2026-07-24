//! Services module for shared business logic.
//!
//! WHY-OODA83: Extracted services follow SRP and DRY principles.
//! Consolidates repeated logic into single, testable modules.

#[cfg(feature = "postgres")]
pub mod ann_readiness;
pub mod artifact_retrieval;
pub mod audit;
#[cfg(feature = "postgres")]
pub mod auth_bootstrap;
pub mod auth_memory_store;
pub mod auth_validation;
pub mod cancel_facade;
pub mod cancel_retract;
pub mod content_granularity;
pub mod content_hasher;
pub mod context_bundle_mapper;
pub mod converting_subprogress;
pub mod cost_aggregation;
pub mod document_assets;
pub mod document_body_loader;
pub mod document_deletion;
pub mod document_graph_cascade;
pub mod document_graph_lineage;
pub mod document_metadata_repair;
pub mod document_metadata_scan;
#[cfg(feature = "postgres")]
pub mod document_mm_asset_persist;
#[cfg(feature = "postgres")]
pub mod document_original_persist;
pub mod document_quota;
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
pub mod include_pdf_assets;
pub mod ingest_admission;
pub mod ingestion_persist;
pub mod ingestion_status;
pub mod ingestion_status_mapper;
pub mod injection_list;
pub mod injection_process;
pub mod interrupted_restart;
pub mod isolation_context;
pub mod job_registry;
pub mod knowledge_rebuild;
pub mod large_document_profile;
pub mod list_pagination;
pub mod llm_text_embedder;
pub mod login_lockout;
pub mod message_context_mapper;
pub mod multimodal;
pub mod multimodal_admission;
pub mod multimodal_context;
pub mod multimodal_markdown;
pub mod oidc_flow;
pub mod oidc_pending;
pub mod orphan_index_retract;
pub mod orphan_staging_recovery;
pub mod orphan_task_recovery;
pub mod pdf_admission_registry;
pub mod pdf_auto_routing;
pub mod pdf_lineage;
pub mod pdf_workspace_dedup;
pub mod pending_doc_task_reconcile;
pub mod pipeline_failure_classify;
pub mod pipeline_ws_bridge;
#[cfg(feature = "postgres")]
pub mod postgres_chunk_lineage;
pub mod process_fingerprint;
pub mod progress_facade;
pub mod query_context;
pub mod query_execution;
pub mod query_generation;
pub mod query_request_builder;
pub mod query_stats_mapper;
pub mod reprocess_admission;
pub mod reprocess_stage_reset;
pub mod retract_document_indexes;
pub mod retrieval_id_cache;
pub mod route_registry;
pub mod session_storage;
pub mod source_reference_builder;
pub mod staging_admission;
pub mod startup_task_hydrate;
pub mod summary_role;
pub mod task_cancel;
pub mod task_document_sync;
pub mod task_scope;
pub mod tenant_guard;
pub mod tenant_isolation;
pub mod text_insert_content;
pub mod v1_rpc_migration;
pub mod vision_content;
pub mod vision_stall_watchdog;
pub mod vlm_limits;
pub mod vlm_provider_resolver;
pub mod workspace_content_hash_dedup;
pub mod workspace_document_index;
pub mod workspace_document_wipe;
pub mod workspace_wipe_admission;

pub use audit::{
    record_audit, record_compliance_event, record_compliance_event_runtime, with_request_context,
};

pub use crate::handlers::documents::upload::document_admission::{
    admit_document_for_processing, chunk_fields_from_metadata, parse_upload_chunk_fields,
    DocumentAdmissionAccepted, DocumentAdmissionDuplicateProcessing, DocumentAdmissionInput,
    DocumentAdmissionOutcome, GleaningAdmissionOptions,
};
pub use cancel_facade::{cancel_track_with_doc_and_pdf_chain, retract_indexes_for_document_id};
pub use cancel_retract::{retract_indexes_for_document, retract_indexes_for_task};
pub use content_granularity::{
    ensure_debug_granularity_allowed, truncate_for_granularity, SNIPPET_LEN,
};
pub use content_hasher::ContentHasher;
pub use converting_subprogress::{
    report_vision_figure_analyze, report_vision_figure_analyze_ex, vision_figure_analyze_message,
    vision_figure_analyze_message_local, vision_figure_analyze_progress_01,
    ConvertingSubstepReporter, VisionFigureProgressOpts,
};
pub use document_assets::{
    document_mm_assets_root, mm_assets_base_dir, multimodal_asset_base_dir,
    multimodal_images_requested, page_drawing_assets_config, page_drawing_assets_config_for_vision,
};
pub use document_deletion::{
    find_active_deletion_track_id, perform_document_deletion, reconcile_stuck_deleting_documents,
    reset_deleting_status, DocumentDeletionResult,
};
pub use document_graph_cascade::{
    analyze_deletion_impact_stats, cascade_remove_document_sources,
    cascade_remove_document_sources_with_progress, cleanup_document_graph_data,
    find_document_edges, find_document_nodes, find_relationships_for_document_lineage,
    sources_for_document, CascadeStats, CleanupStats, DocumentSourceScope,
};
pub use document_graph_lineage::{
    build_document_graph_lineage, entity_summary_from_node, relationship_summary_from_edge,
    DocumentGraphLineageBuild,
};
#[cfg(feature = "postgres")]
pub use document_mm_asset_persist::{
    delete_document_mm_assets, list_mm_asset_summaries_for_document, load_mm_asset_bytes,
    load_mm_asset_bytes_by_id, materialize_mm_assets_to_dir, persist_document_mm_assets_from_dir,
    persist_mm_assets_with_storage, persist_uploaded_mm_assets, store_requests_from_dir,
};
#[cfg(feature = "postgres")]
pub use document_original_persist::{persist_uploaded_original, should_store_original};
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
pub use include_pdf_assets::{include_extracted_pdf_assets, IncludePdfAssetsResult};
pub use ingest_admission::{
    admit_pdf_processing_enqueue, persist_pdf_task_document_id,
    provision_queued_pdf_document_shell, resolve_pdf_ingest_document_id,
    resolve_worker_pdf_document_id, QueuedPdfDocumentShell, WorkerPdfDocumentIdRequest,
};
pub use ingestion_persist::{
    build_chunk_kv_records, persist_ingestion_result, persist_with_providers,
    persist_with_providers_and_progress, persist_with_providers_progress_and_embedder,
    resolve_relational_sink, tag_injection_sources, PersistIngestionParams,
};
pub use ingestion_status::{apply_doc_cancelled_fields, pdf_status_for_cancel};
pub use ingestion_status_mapper::{
    enrich_document_summaries, enrich_document_summaries_with_cancel,
    enrich_document_summary_status, legacy_status_to_unified_stage, map_ingestion_status,
    IngestionStatusInputs, IngestionStatusView,
};
pub use injection_list::{
    list_injections_paged, summary_from_meta, InjectionListPage, DEFAULT_INJECTION_LIST_LIMIT,
    MAX_INJECTION_LIST_LIMIT,
};
pub use injection_process::{
    build_injection_metadata, injection_doc_id, injection_list_prefix, injection_meta_key,
    run_injection_pipeline, write_injection_status,
};
pub use interrupted_restart::{
    is_interrupted_restart_metadata, FAILURE_CODE_SERVER_RESTART_INTERRUPTED,
};
pub use large_document_profile::{
    classify_ingestion_failure, is_provider_misconfig_message, IngestionEstimate,
    IngestionFailureClass, LargeDocumentProfile,
};
pub use llm_text_embedder::LlmTextEmbedder;
pub use message_context_mapper::{
    build_message_context_from_engine, message_context_from_subgraph,
};
pub use multimodal::{
    analysis_cache_enabled, analyze_multimodal_images, analyze_multimodal_images_with_substep,
    analyze_standalone_image, append_mm_chunks_to_text, apply_process_options_to_metadata,
    build_mm_chunks_from_manifest, build_surrounding, collect_mm_chunks_from_manifest,
    enrich_markdown_with_vlm, enrich_processed_text_with_mm_chunks, extract_json_object,
    find_target_span, load_chunk_separators, load_content_rows_by_blockid_jsonl, load_manifest,
    load_mm_chunks, manifest_item_status_views, manifest_key, maybe_attach_cache_key,
    metadata_multimodal_patch, mm_chunks_enabled, mm_chunks_key, parse_json_object,
    persist_manifest, persist_mm_chunks, reanalyze_document_multimodal, render_mm_chunk,
    resolve_process_options_from_metadata, run_multimodal_analyze_stage,
    run_multimodal_analyze_stage_outcome, run_multimodal_analyze_stage_outcome_with_cancel,
    run_multimodal_analyze_stage_outcome_with_substep, scan_manifest_items,
    should_run_image_analysis, summary_from_metadata, table_analysis_messages, vlm_process_enabled,
    AnalyzeOutcome, LocalMmProfile, ManifestItem, MmChunkBuildError, MultimodalChunk,
    MultimodalHeading, MultimodalItemRecord, MultimodalItemStatusView, MultimodalManifest,
    MultimodalProviders, MultimodalReanalyzeOutcome, MultimodalReanalyzeParams, MultimodalSummary,
    PromptContext, SurroundingContext, SurroundingKind, SurroundingTokenCounter, METADATA_FIELD,
};
pub use multimodal_admission::{
    resolve_upload_content, MultimodalAdmissionMeta, ResolvedUploadContent,
};
pub use orphan_index_retract::{
    is_post_graph_incomplete_stage, orphan_retract_on_recover_enabled,
    retract_indexes_for_orphan_docs,
};
pub use orphan_staging_recovery::{
    task_is_live,
    recover_orphaned_staging_admissions, OrphanStagingRecoveryReport,
};
pub use orphan_task_recovery::{recover_orphaned_tasks, OrphanTaskRecoveryReport};
pub use pdf_admission_registry::PdfAdmissionRegistry;
pub use pdf_auto_routing::{should_try_edgeparse_before_vision, try_edgeparse_fast_path};
pub use pdf_workspace_dedup::{
    find_kv_document_id_for_pdf, recycle_orphan_workspace_pdf,
    workspace_has_visible_document_for_pdf,
};
pub use pending_doc_task_reconcile::{ensure_task_for_pending_document, EnsureTaskOutcome};
pub use pipeline_failure_classify::{classify_from_llm_error, classify_from_pipeline_error};
pub use query_context::{
    build_legacy_query_response, build_legacy_query_sources, fetch_context_by_id,
    resolve_query_llm_override, retrieve_context, search_context, FetchContextOptions,
};
pub use query_execution::{
    execute_sota_query, execute_sota_query_stream, execute_sota_query_stream_with_auth_fallback,
    execute_sota_query_with_auth_fallback, is_llm_auth_failure, llm_override_from_request,
    resolve_workspace_query_resources, validate_llm_override_pair, WorkspaceQueryResources,
};
pub use query_generation::{execute_full_query, execute_legacy_query_response};
pub use query_request_builder::{build_engine_request, QueryExecutionParams};
pub use query_stats_mapper::from_engine_stats as map_engine_query_stats;
pub use reprocess_admission::{
    evaluate_reprocess_admission, is_reprocess_completed_status, is_reprocess_inflight_status,
    is_reprocess_lifecycle_exclusive, is_reprocess_orphan_waiting_status,
    is_reprocess_terminal_recoverable, ReprocessAdmitContext, ReprocessAdmitDecision,
    ReprocessSkipReason,
};
pub use retract_document_indexes::{retract_document_indexes, retract_on_cancel_total};
pub use retrieval_id_cache::{global_retrieval_cache, new_retrieval_id, RetrievalIdCache};
pub use source_reference_builder::{build_sources_from_context, is_injection_source};
pub use staging_admission::{
    promote_staging_to_final, release_staging_reservation, rollback_staging,
};
pub use summary_role::resolve_summary_llm_or_fallback;
pub use task_cancel::{
    apply_cancel_all_active, apply_cancel_pdf_pipeline_tasks, apply_task_row_cancel,
    is_cancel_error_message, TaskCancelApplyResult,
};
pub use task_document_sync::{
    extract_document_id_from_task, sync_doc_cancelled_by_document_id, sync_doc_cancelled_for_task,
    sync_document_failed_on_orphan_heartbeat,
};
pub use text_insert_content::{
    patch_document_metadata, resolve_document_metadata_key, resolve_text_insert_content,
};
pub use vision_content::{
    describe_image, describe_image_as_markdown, image_analysis_to_markdown,
    image_analysis_to_markdown_with_asset, parse_image_analysis_json, ImageAnalysisResult,
    MultimodalProcessOptions, IMAGE_TYPE_FALLBACK,
};
pub use vision_stall_watchdog::{
    annotate_timeout_progress, durable_vision_checkpoint_dir, evaluate_vision_watchdog,
    run_with_vision_stall_watchdog, vision_stall_timeout_secs, HeartbeatProgressCallback,
    VisionProgressHeartbeat, VisionWatchdogAbort, DEFAULT_VISION_STALL_TIMEOUT_SECS,
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
pub use workspace_document_wipe::{
    broadcast_wipe_failed, count_planned_wipe_documents, new_wipe_task_data,
    run_workspace_wipe_phases,
};
pub use workspace_wipe_admission::{
    find_active_workspace_wipe_track_id, workspace_wipe_in_flight, WorkspaceWipeAdmissionRegistry,
};
