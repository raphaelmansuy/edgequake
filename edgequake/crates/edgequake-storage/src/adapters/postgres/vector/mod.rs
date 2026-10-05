//! PostgreSQL vector storage using pgvector extension.
//!
//! Provides high-performance vector similarity search using PostgreSQL's
//! pgvector extension with configurable indexing strategies.
//!
//! ## Implements
//!
//! - [`FEAT0203`]: PostgreSQL with pgvector adapter
//! - [`FEAT0320`]: IVFFlat index for approximate nearest neighbor
//! - [`FEAT0321`]: HNSW index for faster queries on large datasets
//! - [`FEAT0322`]: Distance metric — **cosine only** (SPEC-083 X-04).
//!   Indexes use `vector_cosine_ops` / `halfvec_cosine_ops`; L2/IP are not wired.
//!
//! ## Use Cases
//!
//! - [`UC0603`]: System performs vector similarity search
//! - [`UC0604`]: System retrieves similar chunks by embedding
//!
//! ## Enforces
//!
//! - [`BR0320`]: Dimension consistency validation
//! - [`BR0321`]: Index type selection based on dataset size

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::OnceCell;

use super::capabilities::{AnnIndexPolicy, VectorStorageMode};
use super::config::{
    qualified_kv_table_name, qualified_vectors_stats_table_name, qualified_vectors_table_name,
    PostgresConfig, VectorIndexType,
};
use super::connection::PostgresPool;
use super::schema;

mod ddl;
mod fts;
mod migration;
mod search_tuning;
mod storage_impl;
pub mod typed_read;

#[cfg(test)]
mod query_validation_tests;

pub use fts::{
    fts_language_from_env, sanitize_fts_language, DEFAULT_FTS_LANGUAGE, FTS_LANGUAGE_ENV,
};
pub use migration::allow_vector_table_rebuild;
pub use typed_read::{vector_backend_fallback_total, vector_backend_typed_hit_total};

/// PostgreSQL vector storage using pgvector.
///
/// Supports:
/// - IVFFlat index for approximate nearest neighbor search
/// - HNSW index for faster queries on large datasets
/// - **Cosine distance only** (`<=>` / `*_cosine_ops`) — X-04
pub struct PgVectorStorage {
    pub(crate) pool: PostgresPool,
    pub(crate) table_name: String,
    /// Maintained-counter table for O(1) `count()` (SPEC-011 iter 02 Fix A).
    pub(crate) stats_table_name: String,
    pub(crate) namespace: String,
    pub(crate) dimension: usize,
    pub(crate) index_type: VectorIndexType,
    pub(crate) ivfflat_lists: u32,
    pub(crate) hnsw_m: u32,
    pub(crate) hnsw_ef_construction: u32,
    pub(crate) prefix: String,
    pub(crate) storage_mode: VectorStorageMode,
    /// KV table holding chunk text for FTS joins (SPEC-024 2.5 SSOT).
    ///
    /// Defaults to the namespace-local KV table; workspace-scoped vector storage
    /// overrides this to the shared default KV store via [`Self::with_chunk_kv_table`].
    pub(crate) chunk_kv_table_name: String,
    pub(crate) chunk_kv_table_exists: Arc<OnceCell<bool>>,
    /// Cached existence of this instance's legacy `eq_*_vectors` relation (SPEC-383).
    pub(crate) legacy_vectors_relation_exists: Arc<OnceCell<bool>>,
    pub(crate) iterative_scan_supported: Arc<OnceCell<bool>>,
    /// Set by `ensure_ann_index` / partial HNSW so deferred `VectorIndexType::None`
    /// still applies HNSW search GUCs at query time (SPEC-062 / SPEC-064).
    pub(crate) deferred_ann_ready: Arc<AtomicBool>,
}

impl PgVectorStorage {
    /// Reject incompatible embeddings before any typed or legacy query shortcut.
    pub(crate) fn validate_query_dimension(&self, embedding: &[f32]) -> crate::error::Result<()> {
        if embedding.len() != self.dimension {
            return Err(crate::error::StorageError::InvalidQuery(format!(
                "Query dimension {} does not match storage dimension {}",
                embedding.len(),
                self.dimension
            )));
        }
        Ok(())
    }

    /// Single constructor path (STORE-P3-15): all public factories delegate here.
    fn from_parts(pool: PostgresPool, config: PostgresConfig, dimension: usize) -> Self {
        let prefix = config.table_prefix();
        let table_name = qualified_vectors_table_name(&prefix);
        let stats_table_name = qualified_vectors_stats_table_name(&prefix);
        let chunk_kv_table_name = qualified_kv_table_name(&prefix);

        Self {
            pool,
            table_name,
            stats_table_name,
            namespace: config.namespace.clone(),
            dimension,
            index_type: config.vector_index_type,
            ivfflat_lists: config.ivfflat_lists,
            hnsw_m: config.hnsw_m,
            hnsw_ef_construction: config.hnsw_ef_construction,
            prefix,
            storage_mode: VectorStorageMode::from_env(),
            chunk_kv_table_name,
            chunk_kv_table_exists: Arc::new(OnceCell::new()),
            legacy_vectors_relation_exists: Arc::new(OnceCell::new()),
            iterative_scan_supported: Arc::new(OnceCell::new()),
            deferred_ann_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Override the KV table used for native FTS chunk-text joins.
    ///
    /// Workspace-scoped vector tables keep embeddings isolated but chunk bodies
    /// remain in the shared default KV store.
    pub fn with_chunk_kv_table(mut self, table_name: impl Into<String>) -> Self {
        self.chunk_kv_table_name = table_name.into();
        self.chunk_kv_table_exists = Arc::new(OnceCell::new());
        self
    }

    /// Create a new pgvector storage (default 1536-dim embeddings).
    pub fn new(config: PostgresConfig) -> Self {
        Self::from_parts(PostgresPool::new(config.clone()), config, 1536)
    }

    /// Create pgvector storage with a shared connection pool (SPEC-011).
    pub fn with_pool(pool: PostgresPool, config: PostgresConfig, dimension: usize) -> Self {
        Self::from_parts(pool, config, dimension)
    }

    /// Create a new pgvector storage with a specific dimension.
    pub fn with_dimension(config: PostgresConfig, dimension: usize) -> Self {
        Self::with_pool(PostgresPool::new(config.clone()), config, dimension)
    }

    /// Create pgvector storage with shared pool and explicit dimension (SPEC-011).
    pub fn with_pool_and_dimension(
        pool: PostgresPool,
        config: PostgresConfig,
        dimension: usize,
    ) -> Self {
        Self::with_pool(pool, config, dimension)
    }

    /// Override column storage mode (SPEC-064 A/B — ignore `EDGEQUAKE_VECTOR_STORAGE`).
    pub fn with_storage_mode(mut self, mode: VectorStorageMode) -> Self {
        self.storage_mode = mode;
        self
    }

    /// Qualified vectors table name (EXPLAIN / battle harness).
    pub fn vectors_table_name(&self) -> &str {
        &self.table_name
    }

    /// SQL embedding type (`vector` / `halfvec`) for this storage.
    pub fn embedding_sql_type(&self) -> &'static str {
        self.embedding_pg_type()
    }

    /// Active storage mode (`full` / `halfvec`).
    pub fn storage_mode(&self) -> VectorStorageMode {
        self.storage_mode
    }

    /// Index type used for search GUC tuning (promotes deferred None → HNSW when ready).
    pub(crate) fn effective_index_type(&self) -> VectorIndexType {
        match self.index_type {
            VectorIndexType::None if self.deferred_ann_ready.load(Ordering::Acquire) => {
                VectorIndexType::HNSW
            }
            other => other,
        }
    }

    pub(crate) fn mark_deferred_ann_ready(&self) {
        self.deferred_ann_ready.store(true, Ordering::Release);
    }

    /// Cached `information_schema` probe for this instance's legacy vectors table.
    ///
    /// SPEC-383: skip mutate SQL when the relation is gone so Postgres never
    /// logs 42P01. SPEC-111: when the relation exists, lifecycle DELETE still runs.
    pub(crate) async fn legacy_vectors_relation_exists_cached(&self) -> crate::error::Result<bool> {
        if let Some(exists) = self.legacy_vectors_relation_exists.get() {
            return Ok(*exists);
        }
        let pool = self.pool.get().await?;
        let exists = schema::relation_exists(&pool, &self.table_name).await?;
        let _ = self.legacy_vectors_relation_exists.set(exists);
        Ok(exists)
    }

    /// Skip a legacy mutate when `eq_*_vectors` is absent (probe-before-DELETE).
    ///
    /// Returns `true` when the caller must no-op. Distinct from
    /// [`Self::map_legacy_mutate_err`] (TOCTOU 42P01 swallow).
    pub(crate) async fn skip_legacy_mutate_if_absent(
        &self,
        op: &str,
    ) -> crate::error::Result<bool> {
        if self.legacy_vectors_relation_exists_cached().await? {
            return Ok(false);
        }
        tracing::debug!(
            table = %self.table_name,
            op = %op,
            "SPEC-383: skip mutate — legacy vectors relation absent"
        );
        Ok(true)
    }

    /// Skip a legacy SELECT when `eq_*_vectors` is absent (probe-before-read).
    ///
    /// Returns `true` when the caller must soft-empty. Independent of
    /// `EDGEQUAKE_VECTOR_BACKEND` so misconfig / mid-upgrade never 42P01.
    pub(crate) async fn skip_legacy_read_if_absent(&self, op: &str) -> crate::error::Result<bool> {
        if self.legacy_vectors_relation_exists_cached().await? {
            return Ok(false);
        }
        tracing::debug!(
            table = %self.table_name,
            op = %op,
            "SPEC-405: skip legacy read — vectors relation absent"
        );
        Ok(true)
    }

    /// SPEC-091: soft-treat missing legacy `eq_*_vectors` (42P01) as write-stop success.
    /// TOCTOU fallback when the relation is dropped between probe and execute.
    pub(crate) fn map_legacy_mutate_err(
        e: sqlx::Error,
        op: &str,
        table: &str,
    ) -> crate::error::Result<()> {
        if let sqlx::Error::Database(ref db) = e {
            if db.code().as_deref() == Some("42P01") {
                tracing::debug!(
                    table = %table,
                    op = %op,
                    "SPEC-091: legacy vectors relation gone — mutate write-stop"
                );
                return Ok(());
            }
        }
        Err(crate::error::StorageError::Database(format!(
            "{op} failed: {e}"
        )))
    }

    /// Contract/e2e probe for `legacy_chunk_ddl_retired` (SPEC-091 hardening).
    pub async fn probe_legacy_chunk_ddl_retired(&self) -> bool {
        self.legacy_chunk_ddl_retired().await
    }

    /// Contract/e2e probe for `legacy_vector_ddl_retired` (migration 131).
    pub async fn probe_legacy_vector_ddl_retired(&self) -> bool {
        self.legacy_vector_ddl_retired().await
    }
}

impl PgVectorStorage {
    pub(crate) fn embedding_pg_type(&self) -> &'static str {
        AnnIndexPolicy::resolve(self.dimension, self.storage_mode).column_type
    }

    pub(crate) fn embedding_opclass(&self) -> &'static str {
        AnnIndexPolicy::resolve(self.dimension, self.storage_mode).opclass
    }
}

impl std::fmt::Debug for PgVectorStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgVectorStorage")
            .field("namespace", &self.namespace)
            .field("dimension", &self.dimension)
            .field("table_name", &self.table_name)
            .finish()
    }
}
