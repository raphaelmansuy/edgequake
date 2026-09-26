//! # EdgeQuake Storage
//!
//! Storage abstractions and adapters for the EdgeQuake RAG system.
//!
//! # Implements
//!
//! - **FEAT0201**: Vector Similarity Search
//! - **FEAT0202**: Graph Traversal  
//! - **FEAT0203**: Graph Mutation Operations
//! - **FEAT0204**: Graph Analytics
//! - **FEAT0205**: Community Detection
//! - **FEAT0010**: Document Metadata Storage
//!
//! # Enforces
//!
//! - **BR0201**: Tenant isolation (namespace-based scoping)
//! - **BR0008**: Entity names normalized before storage
//! - **BR0009**: Max 1000 nodes per query (paginated)
//!
//! This crate provides:
//! - Storage traits for key-value, vector, and graph operations
//! - In-memory implementations for testing
//! - Production adapters (PostgreSQL AGE + pgvector, SurrealDB)
//! - Community detection algorithms for graph clustering
//!
//! ## Storage Types
//!
//! | Trait | FEAT | Implementation |
//! |-------|------|----------------|
//! | [`KVStorage`] | FEAT0010 | Postgres, Memory |
//! | [`VectorStorage`] | FEAT0201 | pgvector, Memory |
//! | [`GraphStorage`] | FEAT0202-0204 | Apache AGE, Memory |
//!
//! ## Adapter Selection
//!
//! ```text
//! if DATABASE_URL set:
//!     → PostgreSQL adapters (production)
//! else:
//!     → Memory adapters (testing)
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use edgequake_storage::{KVStorage, MemoryKVStorage};
//!
//! let storage = MemoryKVStorage::new("documents");
//! storage.initialize().await?;
//! ```
//!
//! # See Also
//!
//! - [`crate::traits`] for storage trait definitions
//! - [`crate::adapters::memory`] for in-memory implementations
//! - [`crate::adapters::postgres`] for PostgreSQL adapters

/// Driver-free provider contracts. Re-exported for incremental caller migration.
pub use edgequake_storage_contracts as contracts;

pub mod adapters;
pub mod chunk_content;
pub mod chunk_text_authority;
pub mod community;
pub mod community_index_service;
pub mod community_persist;
pub mod community_reports;
pub mod compensation;
pub use compensation::{
    compensate_merge_failure, compensate_merge_failure_with_kv, compensate_orphan_graph_writes,
    compensate_orphan_kv, compensate_orphan_vectors, compensate_shared_entity_skipped_total,
    compensation_quarantine_total, record_compensate_shared_entity_skipped,
    record_vector_dim_mismatch_rejected, set_quarantine_sink, vector_dim_mismatch_rejected_total,
    QuarantineSink,
};
pub mod conversation_storage;
pub mod conversation_types;
pub mod dimension_policy;
pub use dimension_policy::{
    decide_dimension_action, DimensionAction, DimensionEnsureOutcome, DimensionReconcilePolicy,
};
pub mod document_metadata_integrity;
pub mod documents_column_status;
pub use documents_column_status::{
    normalize_documents_column_status, relational_documents_status_for_write,
};
pub mod entity_fuzzy;
pub mod entity_id;
pub mod entity_reconcile;
pub mod error;
pub mod failed_chunks;
pub mod filter_column_policy;
pub use filter_column_policy::{
    ann_exact_max_rows, prefer_denorm_filter_columns, DEFAULT_ANN_EXACT_MAX_ROWS,
};
pub mod chunk_fts;
pub mod dataop;
pub mod dataop_annotations;
pub mod drain_claim;
pub mod embedding_family;
pub use embedding_family::{
    classify_legacy_vector_id, entity_name_from_legacy_id, format_relationship_legacy_key,
    parse_relationship_legacy_key, parse_relationship_legacy_key_with_resolver, EmbeddingFamily,
};
pub mod graph_batch_dedupe;
pub mod graph_metrics;
pub mod kv_family_cutover;
pub mod kv_key_schema;
#[cfg(feature = "postgres")]
pub mod legacy_store_census;
pub mod lineage_canon;
pub mod metadata_filter_sql;
pub mod mm_asset_storage;
pub mod namespace_tables;
pub mod original_storage;
pub mod outbox;
#[cfg(feature = "postgres")]
pub mod outbox_drain;
pub mod page_layout_storage;
pub mod pdf_storage;
#[cfg(feature = "postgres")]
pub mod projection;
pub mod projection_manifest;
pub mod scorecard;
pub mod serving_fence;
pub mod storage_op_metrics;
pub mod traits;
pub mod vector_backend;
pub mod vector_id;

#[cfg(test)]
pub(crate) mod test_env_lock;

pub use vector_backend::{
    legacy_vector_writes_stopped, vector_backend_from_env, vector_backend_reads_typed,
    VectorBackend, VECTOR_BACKEND_ENV,
};

pub use dataop::{all_ref_ids, is_valid_ref_id, sql_comment};
pub use storage_op_metrics::TimedStorageOp;

// Re-export entity identity (RC-6 / P-G1): single normalization entry point.
pub use entity_id::{is_opaque_identifier, normalize_entity_name, EntityId};
// SPEC-083 X-17: optional fuzzy / blocking resolution (default off).
pub use entity_fuzzy::{
    blocking_key, entity_fuzzy_enabled, find_best_fuzzy_match, fuzzy_match_threshold,
    fuzzy_name_similarity, normalized_levenshtein_similarity, token_jaccard_similarity,
};

// Re-export community detection
pub use crate::community_index_service::{
    community_refresh_debounce_secs, pending_community_refresh_workspaces,
    refresh_community_index_now, refresh_community_index_now_with_extras,
    schedule_community_index_refresh, schedule_community_index_refresh_with_extras,
    CommunityRefreshExtras,
};
pub use chunk_content::{
    batch_fetch_chunk_contents, content_from_kv_value, content_from_metadata_or_kv,
};
pub use chunk_text_authority::{
    chunk_text_authority_from_env, chunk_text_authority_writes_kv,
    chunk_text_authority_writes_relational, ChunkTextAuthority, CHUNK_TEXT_AUTHORITY_ENV,
};
#[cfg(feature = "postgres")]
pub mod chunk_text_dual_read;
#[cfg(feature = "postgres")]
pub mod compensation_drain;
#[cfg(feature = "postgres")]
pub mod cutover_flag_guard;
pub mod migration_engine;
pub use community::{
    community_max_nodes_from_env, load_graph_bounded, louvain_hierarchy_enabled, BoundedGraphLoad,
    Community, CommunityAlgorithm, CommunityConfig, CommunityDetectionResult,
};
pub use community_persist::{
    backfill_communities_if_needed, community_auto_max_nodes, community_features_enabled,
    detect_and_persist_communities, needs_community_backfill, persist_community_labels,
    refresh_community_index, spawn_community_backfill_if_needed,
};
pub use community_reports::{
    build_community_report_records, build_extractive_community_report, community_report_vector_id,
    community_report_vector_metadata, community_reports_enabled,
    index_community_reports_with_embedder, pack_community_report_vectors,
    upsert_community_report_vectors, COMMUNITY_REPORT_VECTOR_TYPE,
};
pub use document_metadata_integrity::{
    canonical_document_id, document_id_from_metadata_key, metadata_id_drift,
    repair_document_metadata_in_place, DOCUMENT_METADATA_SUFFIX,
};
pub use failed_chunks::{FailedChunkInsert, FailedChunkRecord, InMemoryFailedChunkStore};
pub use graph_batch_dedupe::{
    dedupe_edges_by_endpoints, dedupe_nodes_by_id, graph_upsert_chunk_size, normalize_rel_type,
    normalize_relation_type_str, parse_graph_upsert_chunk, resolve_graph_upsert_chunk,
    sql_eq_rel_type_arbiter_expr, DEFAULT_GRAPH_UPSERT_CHUNK,
};
pub use graph_metrics::{
    collect_graph_quality_metrics, log_graph_quality, metrics_from_merge_delta, GraphQualityMetrics,
};
#[cfg(feature = "postgres")]
pub use migration_engine::PgVectorCutoverStore;
#[cfg(feature = "postgres")]
pub use migration_engine::{
    ensure_caught_up, run_drain_foreground, run_engine, spawn_for_serving, BackfillJob,
    BindingCompleteness, CutoverState, MigrationEngineConfig, MigrationJobProgress, MigrationMode,
    VectorCutoverStore, VectorProviderCutover, MIGRATION_MODE_ENV,
};
#[cfg(not(feature = "postgres"))]
pub use migration_engine::{
    ensure_caught_up, BindingCompleteness, CutoverState, MigrationJobProgress, MigrationMode,
    VectorCutoverStore, VectorProviderCutover, MIGRATION_MODE_ENV,
};

// Re-export PDF storage types
/// Lineage SSOT helpers (no provider I/O) — available without the postgres feature.
pub use lineage_canon::{
    apply_retained_sources, canonicalize_source_lineage, document_ids_from_properties,
    insert_chunk_lineage_properties, retained_lineage, sources_from_contributions,
    union_source_properties, RetainedLineage, INDEXED_LINEAGE_ARRAY_KEYS,
};
pub use mm_asset_storage::{
    asset_id_from_path, classify_mm_asset_path, guess_mm_asset_content_type, normalize_mm_asset_id,
    normalize_mm_asset_path, validate_mm_asset_data, DocumentMmAsset, DocumentMmAssetStorage,
    DocumentMmAssetSummary, StoreMmAssetRequest, ASSET_KIND_EMBEDDED_FIGURE,
    ASSET_KIND_PAGE_CHART_CROP, ASSET_KIND_PAGE_FULL, ASSET_KIND_TABLE_CROP,
};
pub use original_storage::{
    validate_original_data, DocumentOriginal, DocumentOriginalStorage, StoreOriginalRequest,
};
pub use page_layout_storage::{
    bbox_norm_from_pdf, bbox_norm_iou, DocumentPage, DocumentPageLayoutStorage, LayoutBBoxNorm,
    LayoutBBoxPdf, PageLayoutBundle, PageLayoutRegion, ReplaceDocumentPagesRequest,
    UpsertDocumentPage, UpsertPageLayoutRegion,
};
pub use pdf_storage::{
    calculate_pdf_checksum, validate_pdf_data, CreatePdfRequest, DocumentStatsUpdate,
    ExtractionMethod, ListPdfFilter, PdfDocument, PdfDocumentStorage, PdfList, PdfProcessingStatus,
    UpdatePdfProcessingRequest,
};
#[cfg(feature = "postgres")]
pub use projection::{
    scoped_graph_node_id, AgeGraphProjectionApplier, GraphProjectionApplier, PgProjectionLedger,
    PgvectorProjectionApplier, ProjectionLedger, ProjectionRunReport, ProjectionTarget,
    ProjectionWorkLedger, ProjectionWorker, ProjectionWorkerConfig, ProjectionWorkerCounters,
    ProjectionWorkerRuntime, ServingFenceOpener, VectorProjectionApplier,
};
pub use projection_manifest::canonical_graph_node_id;

pub use conversation_storage::ConversationStorage;
pub use conversation_types::{ConversationRow, FolderRow, MessageRow};

// Re-export traits
#[cfg(feature = "postgres")]
pub use cutover_flag_guard::{
    detect_cutover_posture, validate_cutover_flags, CutoverSchemaPosture,
};
pub use error::StorageError;
pub use kv_family_cutover::{
    kv_family_mode_from_env, KvFamilyMode, KV_FAMILY_CHUNK, KV_FAMILY_COMPENSATION_QUARANTINE,
    KV_FAMILY_ENV_PREFIX, KV_FAMILY_METADATA, KV_FAMILY_WSDOC,
};
#[cfg(feature = "postgres")]
pub use legacy_store_census::{any_legacy_rows, legacy_store_census, LegacyStoreCensus};
pub use namespace_tables::{
    age_graph_name_for_namespace, bare_kv_table_for_namespace, bare_vectors_table_for_namespace,
    sanitize_namespace_segment, table_prefix_for_namespace,
};
#[cfg(feature = "postgres")]
pub use outbox::PostgresOutboxSink;
pub use outbox::{
    enqueue_outbox_best_effort, NoopOutboxSink, OutboxSink, OUTBOX_AGGREGATE_DOCUMENT,
    OUTBOX_EVENT_CHUNK_DECLARED, OUTBOX_EVENT_CHUNK_READY, OUTBOX_EVENT_COMPENSATE,
    OUTBOX_EVENT_MERGE_DONE,
};
#[cfg(feature = "postgres")]
pub use outbox_drain::{
    chaos_claim_without_ack, drain_once as outbox_drain_once, outbox_drain_claimed_total,
    outbox_drain_processed_total, outbox_lag_seconds, spawn_outbox_drain, OutboxDrainConfig,
    OutboxEvent, OUTBOX_DRAIN_ENV,
};
pub use scorecard::{
    AnnMetrics, FullTextMetrics, IngestionMetrics, Scorecard, ScorecardEnvironment,
    ScorecardRecorder,
};
pub use serving_fence::{
    chunk_visible_in_query, filter_ready_chunk_ids, serving_fence_enabled_from_env,
    SERVING_FENCE_ENV, SERVING_STATE_READY,
};
pub use traits::{
    kv_key_matches_like, vector_upsert_chunk_size, BoundedGraphTraversal, Chunk, ChunkCursor,
    ChunkId, ChunkRepository, ChunkText, DocumentId, DocumentRepository, EmbeddingIndex,
    FleetEmbeddingIndex, GraphEdge, GraphExpansionAuthorizer, GraphNode, GraphPropertyWriteMode,
    GraphReadView, GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps,
    GraphStorageReadOps, GraphTraversalBudget, GraphTruncationReason, InsertReport, KVStorage,
    KnowledgeGraph, MetadataFilter, MirrorLegacyReport, ModelId, TextEmbedder, UnitOfWork,
    VectorSearchResult, VectorStorage, WorkspaceId, WorkspaceVectorConfig, WorkspaceVectorRegistry,
    DEFAULT_VECTOR_UPSERT_CHUNK,
};

// Re-export adapters
pub use adapters::memory::{
    MemoryChunkRepository, MemoryConversationStorage, MemoryGraphStorage, MemoryKVStorage,
    MemoryMmAssetStorage, MemoryOriginalStorage, MemoryPageLayoutStorage, MemoryPdfStorage,
    MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
};

#[cfg(feature = "sqlite")]
pub use adapters::sqlite::{
    connect_sqlite, validate_sqlite_deployment, SqliteIngestionCommitter, SqliteProjectionLedger,
};

#[cfg(feature = "qdrant")]
pub use adapters::qdrant::{
    collection_name as qdrant_collection_name, compile_metadata_filter as compile_qdrant_filter,
    drop_qdrant_binding, provision_qdrant_binding, verify_qdrant_binding, CompiledFilter,
    QdrantClient, QdrantPointPayload, QdrantVectorPoint, REQUIRED_PAYLOAD_INDEXES,
};

#[cfg(all(feature = "neo4j", feature = "postgres"))]
pub use adapters::neo4j::{
    AgeRollbackReadiness, GraphDigestMismatch, GraphDigestReport, GraphReplayReport,
    PgNeo4jGraphMigration,
};
#[cfg(feature = "neo4j")]
pub use adapters::neo4j::{
    EdgeRevision as Neo4jEdgeRevision, EntityRevision as Neo4jEntityRevision, GraphObjectPayload,
    Neo4jClient, Neo4jConfig, Neo4jMutationReceipt, Neo4jProvisionReport, Neo4jTraversalRequest,
    Neo4jTraversalResult, RevisionDigest as Neo4jRevisionDigest,
};

// Conditionally export PostgreSQL adapters
#[cfg(feature = "postgres")]
pub use adapters::postgres::{
    allow_vector_table_rebuild, build_ann_select_sql, build_binary_hnsw_index_sql,
    build_binary_rerank_select_sql, build_diskann_embedding_only_index_sql,
    build_diskann_labels_index_sql, build_filtered_diskann_label_select_sql,
    build_postfilter_diskann_select_sql, check_hnsw_index_manifest, check_pool_budget,
    diskann_optin_recipe_statements, diskann_query_tuning_statements, diskann_rescore_for_list,
    document_batch_deliveries_settled, enforce_pool_budget, ensure_admission_document_row,
    ensure_admission_document_row_with_track, evaluate_pool_budget, hnsw_ef_construction_from_env,
    hnsw_partial_by_workspace_enabled, interactive_statement_timeout_ms,
    node_counts_by_source_prefixes_sql, open_serving_fence_when_deliveries_settled,
    open_settled_serving_fences_bounded, parse_hnsw_iterative_scan_mode, partition_allowed,
    pool_instance_count_from_env, pool_role_max_connections,
    pool_role_max_connections_with_queue_floor, quantization_allowed, resolve_pool_max_connections,
    serving_fence_filtered_total, serving_fence_open_changed, serving_fence_opened_total,
    session_application_name, with_session_hygiene, with_session_hygiene_labeled,
    AnnExactReorderPolicy, BinaryQuantizePolicy, BudgetMode, FilteredDiskannLabelPolicy,
    HnswIndexManifest, HnswRuntimePolicy, PgBindingRegistry, PgChunkEmbeddingIndex,
    PgFleetEmbeddingIndex, PgIngestionCommitter, PgPoolBundle, PgQuarantineSink,
    PgServingFenceOpener, PgStandaloneEmbeddingStore, PgVectorStorage, PgVisibilityRepository,
    PgWorkspaceVectorRegistry, PoolBudgetReport, PoolRole, PostgresAGEGraphStorage,
    PostgresChunkRepository, PostgresConfig, PostgresConversationStorage, PostgresKVStorage,
    PostgresMmAssetStorage, PostgresOriginalStorage, PostgresPageLayoutStorage, PostgresPdfStorage,
    PostgresPool, ScaleGateEvidence, StandaloneEmbeddingCapabilities, VectorIndexType,
    VectorStorageMode, WorkspaceLabelMap, DEFAULT_ANN_REORDER_CANDIDATE_K,
    DEFAULT_BINARY_CANDIDATE_K, DISKANN_OPTIN_RESCORE, DISKANN_OPTIN_SEARCH_LIST,
    LAST_SOURCE_PREFIX_COUNT_LEN, LINEAGE_GIN_INDEXES, LINEAGE_GIN_PENDING_LIST_LIMIT_KB,
    MAX_WORKSPACE_LABELS, SOURCE_COUNT_STATEMENT_TIMEOUT_MS, SOURCE_PREFIX_BATCH_LIMIT,
    SOURCE_PREFIX_DISCOVERY_CALLS,
};

// SPEC-091 W3 dual-read counters.
#[cfg(feature = "postgres")]
pub use adapters::postgres::vector::{
    vector_backend_fallback_total, vector_backend_typed_hit_total,
};

// Re-export KV key schema for use across all crates
pub use kv_key_schema::kv_keys;
