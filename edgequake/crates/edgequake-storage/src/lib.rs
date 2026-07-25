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

pub mod adapters;
pub mod chunk_content;
pub mod community;
pub mod community_index_service;
pub mod community_persist;
pub mod community_reports;
pub mod compensation;
pub use compensation::{
    compensate_merge_failure, compensate_merge_failure_with_kv, compensate_orphan_graph_writes,
    compensate_orphan_kv, compensate_orphan_vectors, compensate_shared_entity_skipped_total,
    compensation_quarantine_total, record_compensate_shared_entity_skipped,
    record_vector_dim_mismatch_rejected, vector_dim_mismatch_rejected_total,
};
pub mod conversation_storage;
pub mod conversation_types;
pub mod dimension_policy;
pub use dimension_policy::{
    decide_dimension_action, DimensionAction, DimensionEnsureOutcome, DimensionReconcilePolicy,
};
pub mod document_metadata_integrity;
pub mod entity_fuzzy;
pub mod entity_id;
pub mod entity_reconcile;
pub mod error;
pub mod failed_chunks;
pub mod filter_column_policy;
pub use filter_column_policy::{
    ann_exact_max_rows, prefer_denorm_filter_columns, DEFAULT_ANN_EXACT_MAX_ROWS,
};
pub mod dataop;
pub mod dataop_annotations;
pub mod graph_batch_dedupe;
pub mod graph_metrics;
pub mod kv_key_schema;
pub mod metadata_filter_sql;
pub mod mm_asset_storage;
pub mod original_storage;
pub mod pdf_storage;
pub mod storage_op_metrics;
pub mod traits;
pub mod vector_id;

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
    parse_graph_upsert_chunk, resolve_graph_upsert_chunk, DEFAULT_GRAPH_UPSERT_CHUNK,
};
pub use graph_metrics::{
    collect_graph_quality_metrics, log_graph_quality, metrics_from_merge_delta, GraphQualityMetrics,
};

// Re-export PDF storage types
pub use mm_asset_storage::{
    asset_id_from_path, classify_mm_asset_path, guess_mm_asset_content_type, normalize_mm_asset_id,
    normalize_mm_asset_path, validate_mm_asset_data, DocumentMmAsset, DocumentMmAssetStorage,
    DocumentMmAssetSummary, StoreMmAssetRequest, ASSET_KIND_EMBEDDED_FIGURE,
    ASSET_KIND_PAGE_CHART_CROP, ASSET_KIND_PAGE_FULL, ASSET_KIND_TABLE_CROP,
};
pub use original_storage::{
    validate_original_data, DocumentOriginal, DocumentOriginalStorage, StoreOriginalRequest,
};
pub use pdf_storage::{
    calculate_pdf_checksum, validate_pdf_data, CreatePdfRequest, DocumentStatsUpdate,
    ExtractionMethod, ListPdfFilter, PdfDocument, PdfDocumentStorage, PdfList, PdfProcessingStatus,
    UpdatePdfProcessingRequest,
};

pub use conversation_storage::ConversationStorage;
pub use conversation_types::{ConversationRow, FolderRow, MessageRow};

// Re-export traits
pub use error::StorageError;
pub use traits::{
    kv_key_matches_like, vector_upsert_chunk_size, GraphEdge, GraphNode, GraphReadView,
    GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps, GraphStorageReadOps, KVStorage,
    KnowledgeGraph, MetadataFilter, TextEmbedder, VectorSearchResult, VectorStorage,
    WorkspaceVectorConfig, WorkspaceVectorRegistry, DEFAULT_VECTOR_UPSERT_CHUNK,
};

// Re-export adapters
pub use adapters::memory::{
    MemoryConversationStorage, MemoryGraphStorage, MemoryKVStorage, MemoryMmAssetStorage,
    MemoryOriginalStorage, MemoryPdfStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
};

// Conditionally export PostgreSQL adapters
#[cfg(feature = "postgres")]
pub use adapters::postgres::{
    allow_vector_table_rebuild, build_ann_select_sql, build_binary_hnsw_index_sql,
    build_binary_rerank_select_sql, build_diskann_embedding_only_index_sql,
    build_diskann_labels_index_sql, build_filtered_diskann_label_select_sql,
    build_postfilter_diskann_select_sql, diskann_optin_recipe_statements,
    diskann_query_tuning_statements, diskann_rescore_for_list, hnsw_ef_construction_from_env,
    hnsw_partial_by_workspace_enabled, parse_hnsw_iterative_scan_mode, AnnExactReorderPolicy,
    BinaryQuantizePolicy, FilteredDiskannLabelPolicy, HnswRuntimePolicy, PgVectorStorage,
    PgWorkspaceVectorRegistry, PostgresAGEGraphStorage, PostgresConfig,
    PostgresConversationStorage, PostgresKVStorage, PostgresMmAssetStorage,
    PostgresOriginalStorage, PostgresPdfStorage, PostgresPool, VectorIndexType, VectorStorageMode,
    WorkspaceLabelMap, DEFAULT_ANN_REORDER_CANDIDATE_K, DEFAULT_BINARY_CANDIDATE_K,
    DISKANN_OPTIN_RESCORE, DISKANN_OPTIN_SEARCH_LIST, MAX_WORKSPACE_LABELS,
};

// Re-export KV key schema for use across all crates
pub use kv_key_schema::kv_keys;
