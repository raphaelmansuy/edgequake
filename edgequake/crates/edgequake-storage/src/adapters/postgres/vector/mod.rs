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
//! - [`FEAT0322`]: Configurable distance metrics (cosine, L2, inner product)
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

use std::sync::Arc;

use tokio::sync::OnceCell;

use super::config::{
    qualified_kv_table_name, qualified_vectors_stats_table_name, qualified_vectors_table_name,
    PostgresConfig, VectorIndexType,
};
use super::connection::PostgresPool;

mod ddl;
mod fts;
mod migration;
mod search_tuning;
mod storage_impl;

/// PostgreSQL vector storage using pgvector.
///
/// Supports:
/// - IVFFlat index for approximate nearest neighbor search
/// - HNSW index for faster queries on large datasets  
/// - Cosine, L2, and inner product distance metrics
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
    /// KV table holding chunk text for FTS joins (SPEC-024 2.5 SSOT).
    ///
    /// Defaults to the namespace-local KV table; workspace-scoped vector storage
    /// overrides this to the shared default KV store via [`Self::with_chunk_kv_table`].
    pub(crate) chunk_kv_table_name: String,
    pub(crate) chunk_kv_table_exists: Arc<OnceCell<bool>>,
    pub(crate) iterative_scan_supported: Arc<OnceCell<bool>>,
}

impl PgVectorStorage {
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
            chunk_kv_table_name,
            chunk_kv_table_exists: Arc::new(OnceCell::new()),
            iterative_scan_supported: Arc::new(OnceCell::new()),
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
