//! PostgreSQL adapters using pgvector and Apache AGE.
//!
//! This module provides PostgreSQL-based storage implementations:
//! - `PgVectorStorage` - Vector storage using pgvector extension
//! - `PgWorkspaceVectorRegistry` - Per-workspace vector storage manager
//! - `PostgresAGEGraphStorage` - Graph storage using Apache AGE extension
//! - `PostgresKVStorage` - Key-value storage using JSONB
//! - `PostgresConversationStorage` - Conversation, message, and folder storage
//! - `rls` - Row-Level Security context management for multi-tenancy
//!
//! ## Implements
//!
//! - [`FEAT0202`]: PostgreSQL with pgvector adapter
//! - [`FEAT0203`]: Apache AGE graph storage
//! - [`FEAT0240`]: JSONB key-value storage
//! - [`FEAT0250`]: Conversation persistence
//! - [`FEAT0260`]: Row-Level Security for multi-tenancy
//! - [`FEAT0350`]: Per-workspace vector storage with independent dimensions
//!
//! ## Use Cases
//!
//! - [`UC0601`]: System stores documents in PostgreSQL
//! - [`UC0602`]: System stores entities in Apache AGE graph
//! - [`UC0603`]: System performs vector similarity search with pgvector
//! - [`UC0801`]: System manages conversation history
//!
//! ## Enforces
//!
//! - [`BR0202`]: ACID transactions for data integrity
//! - [`BR0240`]: Tenant isolation via RLS policies
//! - [`BR0350`]: Each workspace has isolated vector storage

mod age_csv_loader;
pub(crate) mod ann_abac;
mod ann_exact_reorder_policy;
mod binary_quantize_policy;
mod capabilities;
pub mod chunk_embedding_index;
pub(crate) mod chunk_repository;
mod config;
mod connection;
mod conversation;
mod diskann_runtime_policy;
pub mod document_shell;
mod filtered_diskann_label_policy;
pub mod fleet_embedding_index;
mod fleet_legacy_absorb;
mod graph;
mod hnsw_manifest;
mod hnsw_runtime_policy;
mod id_allocation;
pub mod ingestion_dedup;
mod kv;
mod kv_relation_state;
pub mod llm_cache;
mod mm_asset_storage_impl;
mod original_storage_impl;
mod page_layout_storage_impl;
mod pdf_list_query;
mod pdf_storage_impl;
mod pool_budget;
mod pool_bundle;
mod quarantine_sink;
pub mod rls;
mod row_count_stats;
mod scale_gates;
mod schema;
pub(crate) mod serving_fence_query;
mod statement_timeout;
pub(crate) mod typed_embedding_dims;
pub mod vector;
mod workspace_probe_cache;
mod workspace_table;
mod workspace_vector;

pub use age_csv_loader::{load_vertices_from_csv, should_use_copy_loader};
pub use ann_exact_reorder_policy::{
    build_ann_select_sql, AnnExactReorderPolicy, DEFAULT_ANN_REORDER_CANDIDATE_K,
};
pub use binary_quantize_policy::{
    build_binary_hnsw_index_sql, build_binary_rerank_select_sql, BinaryQuantizePolicy,
    DEFAULT_BINARY_CANDIDATE_K,
};
pub use capabilities::{
    age_copy_loader_min_rows, age_rls_requested, age_supports_copy_loader, age_supports_rls,
    extension_version_at_least, pgvector_meets_cve_floor, AnnIndexPolicy, DocumentIdGenerator,
    PostgresCapabilities, PostgresCapabilityProbe, VectorStorageMode, HNSW_MAX_DIM_HALFVEC,
    HNSW_MAX_DIM_VECTOR, PGVECTOR_MIN_CVE_SAFE, PGVECTOR_MIN_ITERATIVE_SCAN,
    SUPPORTED_VECTOR_METRIC, VECTOR_COSINE_OPCLASS,
};
pub use chunk_embedding_index::PgChunkEmbeddingIndex;
pub use chunk_repository::{
    ensure_admission_document_row, ensure_admission_document_row_with_track,
    PostgresChunkRepository,
};
pub use config::{
    hnsw_ef_construction_from_env, qualified_kv_table_name, resolve_pool_max_connections,
    PostgresConfig, VectorIndexType,
};
pub use connection::{
    apply_session_baseline, pool_idle_timeout, pool_max_lifetime, session_application_name,
    with_session_hygiene, with_session_hygiene_labeled, PostgresPool,
};
pub use conversation::PostgresConversationStorage;
pub use diskann_runtime_policy::{
    diskann_optin_recipe_statements, diskann_query_tuning_statements, diskann_rescore_for_list,
    DISKANN_OPTIN_RESCORE, DISKANN_OPTIN_SEARCH_LIST,
};
pub use filtered_diskann_label_policy::{
    build_diskann_embedding_only_index_sql, build_diskann_labels_index_sql,
    build_filtered_diskann_label_select_sql, build_postfilter_diskann_select_sql,
    FilteredDiskannLabelPolicy, WorkspaceLabelMap, MAX_WORKSPACE_LABELS,
};
pub use fleet_embedding_index::PgFleetEmbeddingIndex;
pub use graph::{
    interactive_statement_timeout_ms, PostgresAGEGraphStorage, LAST_SOURCE_PREFIX_COUNT_LEN,
    SOURCE_COUNT_STATEMENT_TIMEOUT_MS, SOURCE_PREFIX_BATCH_LIMIT, SOURCE_PREFIX_DISCOVERY_CALLS,
};
pub use hnsw_manifest::{check_hnsw_index_manifest, HnswIndexManifest, HnswManifestDrift};
pub use hnsw_runtime_policy::{
    filtered_ann_gucs_satisfy_contract, hnsw_partial_by_workspace_enabled,
    parse_hnsw_iterative_scan_mode, parse_partial_by_workspace_env, HnswRuntimePolicy,
};
pub use id_allocation::{allocate_document_id, is_uuidv7};
pub use kv::PostgresKVStorage;
pub use kv_relation_state::{KvRelationPresence, KvRelationState};
pub use mm_asset_storage_impl::PostgresMmAssetStorage;
pub use original_storage_impl::PostgresOriginalStorage;
pub use page_layout_storage_impl::PostgresPageLayoutStorage;
pub use pdf_storage_impl::PostgresPdfStorage;
pub use pool_budget::{
    check_pool_budget, enforce_pool_budget, evaluate_pool_budget, pool_instance_count_from_env,
    BudgetMode, PoolBudgetReport, DEFAULT_TOOLS_HEADROOM,
};
pub use pool_bundle::{
    pool_role_max_connections, pool_role_max_connections_with_queue_floor, PgPoolBundle, PoolRole,
};
pub use quarantine_sink::PgQuarantineSink;
#[allow(deprecated)]
pub use rls::{
    acquire_rls_connection, clear_tenant_context, clear_tenant_context_on_conn,
    release_rls_connection, set_tenant_context, set_tenant_context_on_conn,
    with_acquired_tenant_context, with_rls_transaction, RlsQueryBuilder, RlsTxFuture,
};

// SPEC-046 OPS-P2.16: `RlsContext` is no longer re-exported from `postgres::`.
// Use `acquire_rls_connection` / `with_acquired_tenant_context` (SEC-014 SSOT).
// The type remains in `rls` for transitional `#[deprecated]` compile errors.
pub use scale_gates::{partition_allowed, quantization_allowed, ScaleGateEvidence};
pub use serving_fence_query::{apply_serving_fence, serving_fence_filtered_total};
pub use vector::{
    allow_vector_table_rebuild, fts_language_from_env, sanitize_fts_language, PgVectorStorage,
    DEFAULT_FTS_LANGUAGE, FTS_LANGUAGE_ENV,
};
pub use workspace_vector::PgWorkspaceVectorRegistry;
