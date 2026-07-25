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
mod ann_exact_reorder_policy;
mod binary_quantize_policy;
mod capabilities;
mod config;
mod connection;
mod conversation;
mod diskann_runtime_policy;
mod filtered_diskann_label_policy;
mod graph;
mod hnsw_runtime_policy;
mod id_allocation;
mod kv;
mod mm_asset_storage_impl;
mod original_storage_impl;
mod pdf_list_query;
mod pdf_storage_impl;
pub mod rls;
mod row_count_stats;
mod schema;
mod vector;
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
    PostgresCapabilities, VectorStorageMode, HNSW_MAX_DIM_HALFVEC, HNSW_MAX_DIM_VECTOR,
    PGVECTOR_MIN_CVE_SAFE, PGVECTOR_MIN_ITERATIVE_SCAN, SUPPORTED_VECTOR_METRIC,
    VECTOR_COSINE_OPCLASS,
};
pub use config::{hnsw_ef_construction_from_env, PostgresConfig, VectorIndexType};
pub use connection::PostgresPool;
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
pub use graph::PostgresAGEGraphStorage;
pub use hnsw_runtime_policy::{
    filtered_ann_gucs_satisfy_contract, hnsw_partial_by_workspace_enabled,
    parse_hnsw_iterative_scan_mode, parse_partial_by_workspace_env, HnswRuntimePolicy,
};
pub use id_allocation::{allocate_document_id, is_uuidv7};
pub use kv::PostgresKVStorage;
pub use mm_asset_storage_impl::PostgresMmAssetStorage;
pub use original_storage_impl::PostgresOriginalStorage;
pub use pdf_storage_impl::PostgresPdfStorage;
#[allow(deprecated)]
pub use rls::{
    acquire_rls_connection, clear_tenant_context, clear_tenant_context_on_conn,
    release_rls_connection, set_tenant_context, set_tenant_context_on_conn,
    with_acquired_tenant_context, with_rls_transaction, RlsQueryBuilder, RlsTxFuture,
};

// SPEC-046 OPS-P2.16: `RlsContext` is no longer re-exported from `postgres::`.
// Use `acquire_rls_connection` / `with_acquired_tenant_context` (SEC-014 SSOT).
// The type remains in `rls` for transitional `#[deprecated]` compile errors.
pub use vector::{
    allow_vector_table_rebuild, fts_language_from_env, sanitize_fts_language, PgVectorStorage,
    DEFAULT_FTS_LANGUAGE, FTS_LANGUAGE_ENV,
};
pub use workspace_vector::PgWorkspaceVectorRegistry;
