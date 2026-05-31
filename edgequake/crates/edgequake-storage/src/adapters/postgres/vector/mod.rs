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

use super::config::{PostgresConfig, VectorIndexType};
use super::connection::PostgresPool;
use super::row_count_stats::{self, RowCountStatsConfig};
use crate::error::{Result, StorageError};

/// PostgreSQL vector storage using pgvector.
///
/// Supports:
/// - IVFFlat index for approximate nearest neighbor search
/// - HNSW index for faster queries on large datasets  
/// - Cosine, L2, and inner product distance metrics
pub struct PgVectorStorage {
    pool: PostgresPool,
    table_name: String,
    /// Maintained-counter table for O(1) `count()` (SPEC-011 iter 02 Fix A).
    stats_table_name: String,
    namespace: String,
    dimension: usize,
    index_type: VectorIndexType,
    ivfflat_lists: u32,
    hnsw_m: u32,
    hnsw_ef_construction: u32,
    prefix: String,
    /// Lazily-detected capability cache: does the live pgvector support the
    /// iterative-scan GUCs (`hnsw.iterative_scan` / `ivfflat.iterative_scan`)?
    ///
    /// WHY: those GUCs only exist in pgvector >= 0.8.0. On 0.7.x the server
    /// rejects them with `invalid configuration parameter name`, which would
    /// abort every filtered query. We probe `pg_extension.extversion` once and
    /// cache the answer so the QW3 recall tuning degrades gracefully (it simply
    /// omits the optional iterative-scan hints) instead of breaking search.
    iterative_scan_supported: Arc<OnceCell<bool>>,
}

impl PgVectorStorage {
    /// Create a new pgvector storage.
    pub fn new(config: PostgresConfig) -> Self {
        Self::with_pool(PostgresPool::new(config.clone()), config, 1536)
    }

    /// Create pgvector storage with a shared connection pool (SPEC-011).
    pub fn with_pool(pool: PostgresPool, config: PostgresConfig, dimension: usize) -> Self {
        let prefix = config.table_prefix();
        let table_name = format!("public.eq_{}_vectors", prefix);
        let stats_table_name = format!("public.eq_{}_vectors_stats", prefix);
        let namespace = config.namespace.clone();
        let index_type = config.vector_index_type;
        let ivfflat_lists = config.ivfflat_lists;
        let hnsw_m = config.hnsw_m;
        let hnsw_ef_construction = config.hnsw_ef_construction;

        Self {
            pool,
            table_name,
            stats_table_name,
            namespace,
            dimension,
            index_type,
            ivfflat_lists,
            hnsw_m,
            hnsw_ef_construction,
            prefix,
            iterative_scan_supported: Arc::new(OnceCell::new()),
        }
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

    /// Create the vectors table.
    async fn create_table(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        // Ensure pgvector extension is available
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to create vector extension: {}", e))
            })?;

        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                embedding vector({}) NOT NULL,
                metadata JSONB DEFAULT '{{}}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name, self.dimension
        );

        sqlx::query(&sql).execute(&pool).await.map_err(|e| {
            StorageError::Database(format!("Failed to create vectors table: {}", e))
        })?;

        // Create vector index
        let index_sql = match self.index_type {
            VectorIndexType::IVFFlat => format!(
                "CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx ON {} USING ivfflat (embedding vector_cosine_ops) WITH (lists = {})",
                self.prefix, self.table_name, self.ivfflat_lists
            ),
            VectorIndexType::HNSW => format!(
                "CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx ON {} USING hnsw (embedding vector_cosine_ops) WITH (m = {}, ef_construction = {})",
                self.prefix, self.table_name, self.hnsw_m, self.hnsw_ef_construction
            ),
            VectorIndexType::None => String::new(),
        };

        // Index creation may fail if table is empty, that's OK
        if !index_sql.is_empty() {
            sqlx::query(&index_sql).execute(&pool).await.ok();
        }

        // GIN index on metadata JSONB for Tier 2 metadata pre-filtering (SPEC-007)
        let gin_sql = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_metadata_idx ON {} USING GIN (metadata jsonb_path_ops)",
            self.prefix, self.table_name
        );
        sqlx::query(&gin_sql).execute(&pool).await.ok();

        // Materialized columns + B-tree indexes for Tier 3 (SPEC-007)
        // ADD COLUMN IF NOT EXISTS is safe and instant (no table rewrite)
        let add_cols = format!(
            r#"
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS document_id TEXT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS tenant_id TEXT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS workspace_id TEXT
            "#,
            self.table_name, self.table_name, self.table_name
        );
        // Each ALTER must be executed separately
        for stmt in add_cols.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(stmt.trim()).execute(&pool).await.ok();
        }

        let doc_idx = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_doc_id_idx ON {} (document_id) WHERE document_id IS NOT NULL",
            self.prefix, self.table_name
        );
        sqlx::query(&doc_idx).execute(&pool).await.ok();

        let tenant_idx = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_tenant_ws_idx ON {} (tenant_id, workspace_id) WHERE tenant_id IS NOT NULL",
            self.prefix, self.table_name
        );
        sqlx::query(&tenant_idx).execute(&pool).await.ok();

        // SPEC-011 iter 02 Fix A: O(1) maintained counter for `count()`.
        self.ensure_row_count_stats(&pool).await?;

        Ok(())
    }

    /// O(1) row counter for `count()` — avoids `SELECT COUNT(*) FROM vectors`.
    ///
    /// SPEC-011 iter 02 Fix A: production polls vector `count()` every 30 s; while
    /// today it is fast (small table), it is O(N) and will degrade as embeddings grow.
    /// This mirrors the proven KV pattern: counter table + INSERT/DELETE triggers.
    /// `clear()` / `clear_workspace()` use `DELETE FROM` (not TRUNCATE), so row
    /// triggers fire naturally — no explicit reset needed unlike KV.
    async fn ensure_row_count_stats(&self, pool: &sqlx::PgPool) -> Result<()> {
        row_count_stats::ensure_row_count_stats(
            pool,
            &RowCountStatsConfig {
                prefix: &self.prefix,
                table_name: &self.table_name,
                stats_table_name: &self.stats_table_name,
                kind: "vectors",
            },
        )
        .await
    }

    /// Convert embedding vector to PostgreSQL format.
    fn format_embedding(embedding: &[f32]) -> String {
        let values: Vec<String> = embedding.iter().map(|v| v.to_string()).collect();
        format!("[{}]", values.join(","))
    }

    /// QW3: derive the transaction-scoped approximate-search GUCs for a query.
    ///
    /// Returns the list of `SET LOCAL ...` statements to run before the search.
    /// Kept as a pure function (no I/O) so its policy is unit-testable.
    ///
    /// # WHY tune recall per query
    /// pgvector's defaults (`hnsw.ef_search = 40`, `ivfflat.probes = 1`) favor
    /// latency over recall. For larger `top_k`, or when a metadata pre-filter
    /// discards candidates, the approximate scan can return fewer/worse rows
    /// than requested. We scale the search effort with `top_k` (clamped to a
    /// sane ceiling so a pathological `top_k` cannot melt the database) and, for
    /// filtered queries, enable `iterative_scan` (pgvector >= 0.8) so the scan
    /// keeps pulling candidates until the post-filter `LIMIT` is satisfied,
    /// bounded by `max_scan_tuples`.
    ///
    /// `iterative_scan_supported` gates the version-specific GUCs: on pgvector
    /// < 0.8.0 those parameters do not exist and the server would reject them,
    /// so the caller passes `false` and we emit only the always-available
    /// `ef_search` / `probes` hints.
    ///
    /// `SET LOCAL` is mandatory: it scopes the change to the current
    /// transaction and is reverted on commit/rollback, so it never leaks onto
    /// the shared pooled connection used by other requests.
    fn search_tuning_statements(
        index_type: VectorIndexType,
        top_k: usize,
        filtered: bool,
        iterative_scan_supported: bool,
    ) -> Vec<String> {
        let mut stmts = Vec::new();
        match index_type {
            VectorIndexType::HNSW => {
                let ef = (top_k.saturating_mul(4)).clamp(40, 1000);
                stmts.push(format!("SET LOCAL hnsw.ef_search = {}", ef));
                if filtered && iterative_scan_supported {
                    // strict_order preserves exact distance ordering while
                    // iterating; max_scan_tuples bounds worst-case work.
                    stmts.push("SET LOCAL hnsw.iterative_scan = strict_order".to_string());
                    stmts.push("SET LOCAL hnsw.max_scan_tuples = 20000".to_string());
                }
            }
            VectorIndexType::IVFFlat => {
                let probes = top_k.clamp(10, 200);
                stmts.push(format!("SET LOCAL ivfflat.probes = {}", probes));
                if filtered && iterative_scan_supported {
                    // IVFFlat only supports relaxed_order for iterative scan.
                    stmts.push("SET LOCAL ivfflat.iterative_scan = relaxed_order".to_string());
                }
            }
            VectorIndexType::None => {}
        }
        stmts
    }

    /// Detect (once, then cache) whether the live pgvector supports iterative
    /// scan GUCs, i.e. version >= 0.8.0.
    ///
    /// WHY cache: the version cannot change within a process lifetime, so a
    /// single `pg_extension` lookup amortizes across all queries. On any error
    /// (missing extension row, unparsable version, pool not ready) we default
    /// to `false` — the safe choice that keeps queries working by skipping the
    /// optional optimization rather than risking an invalid-GUC failure.
    async fn supports_iterative_scan(&self) -> bool {
        *self
            .iterative_scan_supported
            .get_or_init(|| async {
                let pool = match self.pool.get().await {
                    Ok(p) => p,
                    Err(_) => return false,
                };
                let version: Option<(String,)> =
                    sqlx::query_as("SELECT extversion FROM pg_extension WHERE extname = 'vector'")
                        .fetch_optional(&pool)
                        .await
                        .ok()
                        .flatten();
                match version {
                    Some((v,)) => {
                        let supported = pgvector_supports_iterative_scan(&v);
                        tracing::debug!(
                            pgvector_version = %v,
                            iterative_scan_supported = supported,
                            "Detected pgvector iterative-scan capability"
                        );
                        supported
                    }
                    None => false,
                }
            })
            .await
    }

    /// Parse embedding from PostgreSQL text format.
    fn parse_embedding(text: &str) -> Vec<f32> {
        let trimmed = text.trim_start_matches('[').trim_end_matches(']');
        trimmed
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect()
    }

    /// Get the dimension of the vector column in the database table.
    ///
    /// This queries the pg_attribute system catalog to get the vector column's
    /// dimension from atttypmod, which persists even when the table is empty.
    /// This is essential for detecting dimension mismatches after provider changes.
    ///
    /// @implements BR0320: Dimension consistency validation
    /// @implements OODA-228: Fix dimension detection for empty tables
    ///
    /// Returns `None` if the table doesn't exist or has no embedding column.
    pub async fn get_stored_dimension(&self) -> Result<Option<usize>> {
        let pool = match self.pool.get().await {
            Ok(p) => p,
            Err(_) => return Ok(None), // Pool not initialized yet
        };

        // Parse table name to extract schema and table
        let (schema, table) = if self.table_name.contains('.') {
            let parts: Vec<&str> = self.table_name.split('.').collect();
            (parts[0], parts[1])
        } else {
            ("public", self.table_name.as_str())
        };

        // Query the column's atttypmod from pg_attribute.
        // For pgvector, atttypmod stores the dimension directly.
        // This works even when the table is EMPTY, unlike querying stored vectors.
        //
        // WHY pg_attribute.atttypmod?
        // - pgvector stores dimension in atttypmod (type modifier)
        // - This is set when CREATE TABLE defines vector(N)
        // - Persists regardless of table contents
        let sql = r#"
            SELECT a.atttypmod
            FROM pg_attribute a
            JOIN pg_class c ON a.attrelid = c.oid
            JOIN pg_namespace n ON c.relnamespace = n.oid
            WHERE n.nspname = $1
              AND c.relname = $2
              AND a.attname = 'embedding'
              AND a.atttypmod > 0
        "#;

        let result: Option<(i32,)> = sqlx::query_as(sql)
            .bind(schema)
            .bind(table)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to get column dimension: {}", e))
            })?;

        match result {
            Some((dim,)) if dim > 0 => {
                tracing::debug!(
                    table = %self.table_name,
                    dimension = dim,
                    "Got column dimension from pg_attribute.atttypmod"
                );
                Ok(Some(dim as usize))
            }
            _ => {
                // Fallback: try to query stored vectors (works if table has data)
                // This covers cases where atttypmod might not be set correctly
                let fallback_sql = format!(
                    "SELECT vector_dims(embedding) as dim FROM {} LIMIT 1",
                    self.table_name
                );

                let fallback_result: Option<(i32,)> = sqlx::query_as(&fallback_sql)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten();

                match fallback_result {
                    Some((dim,)) if dim > 0 => {
                        tracing::debug!(
                            table = %self.table_name,
                            dimension = dim,
                            "Got dimension from stored vector (fallback)"
                        );
                        Ok(Some(dim as usize))
                    }
                    _ => Ok(None),
                }
            }
        }
    }

    /// Drop the vectors table if it exists.
    ///
    /// @implements OODA-228: Support dimension changes after provider switch
    ///
    /// # Warning
    ///
    /// This permanently deletes all vectors stored in this table.
    /// Use with caution and only when dimension migration is required.
    pub async fn drop_table(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!("DROP TABLE IF EXISTS {} CASCADE", self.table_name);

        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to drop vectors table: {}", e)))?;

        tracing::info!(
            table = %self.table_name,
            "Dropped vector table for dimension migration"
        );

        Ok(())
    }

    /// Ensure the table has the correct dimension, recreating if necessary.
    ///
    /// @implements OODA-228: Fix vector dimension mismatch after provider switch
    ///
    /// When an embedding provider is changed (e.g., OpenAI 1536 → Ollama 768),
    /// the PostgreSQL table's vector column dimension must be updated.
    /// Since PostgreSQL does not support ALTER COLUMN TYPE for vector columns,
    /// we must DROP and recreate the table.
    ///
    /// # Algorithm
    ///
    /// 1. Initialize pool connection if not already done
    /// 2. Check if table exists and get stored dimension
    /// 3. If table doesn't exist → create with required dimension (normal init)
    /// 4. If dimension matches → no-op (table is compatible)
    /// 5. If dimension differs → DROP TABLE and recreate with new dimension
    ///
    /// # Warning
    ///
    /// This may permanently delete stored vectors if dimension change is detected.
    /// The caller should ensure documents are re-embedded before calling queries.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if table was recreated due to dimension change
    /// - `Ok(false)` if no recreation was needed
    /// - `Err(_)` on database errors
    pub async fn ensure_dimension(&self, required_dimension: usize) -> Result<bool> {
        // Initialize pool connection first (required for database operations)
        // WHY: This method may be called before initialize(), so we need to
        // ensure the pool is ready before querying the database.
        self.pool.initialize().await?;

        // Now check if table exists by querying stored dimension
        let stored_dim = self.get_stored_dimension().await?;

        match stored_dim {
            Some(dim) if dim == required_dimension => {
                // Dimension matches, no action needed
                tracing::debug!(
                    table = %self.table_name,
                    dimension = required_dimension,
                    "Vector table dimension matches, no recreation needed"
                );
                Ok(false)
            }
            Some(dim) => {
                // Dimension mismatch - need to recreate table
                tracing::warn!(
                    table = %self.table_name,
                    old_dimension = dim,
                    new_dimension = required_dimension,
                    "Vector dimension mismatch detected, recreating table"
                );

                // Drop existing table
                self.drop_table().await?;

                // Recreate with new dimension
                self.create_table().await?;

                tracing::info!(
                    table = %self.table_name,
                    dimension = required_dimension,
                    "Vector table recreated with new dimension"
                );

                Ok(true)
            }
            None => {
                // Table is empty or doesn't exist - create_table handles this
                // (CREATE TABLE IF NOT EXISTS is idempotent for empty tables)
                tracing::debug!(
                    table = %self.table_name,
                    dimension = required_dimension,
                    "Vector table empty or not exists, will create on initialize"
                );
                Ok(false)
            }
        }
    }

    /// Check if the table exists in the database.
    ///
    /// @implements OODA-228: Dimension validation helper
    pub async fn table_exists(&self) -> Result<bool> {
        let pool = match self.pool.get().await {
            Ok(p) => p,
            Err(_) => return Ok(false), // Pool not initialized yet
        };

        // Parse table name to extract schema and table
        let (schema, table) = if self.table_name.contains('.') {
            let parts: Vec<&str> = self.table_name.split('.').collect();
            (parts[0], parts[1])
        } else {
            ("public", self.table_name.as_str())
        };

        let sql = r#"
            SELECT EXISTS (
                SELECT 1 FROM information_schema.tables 
                WHERE table_schema = $1 AND table_name = $2
            )
        "#;

        let exists: (bool,) = sqlx::query_as(sql)
            .bind(schema)
            .bind(table)
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to check table existence: {}", e))
            })?;

        Ok(exists.0)
    }
}

mod storage_impl;

impl std::fmt::Debug for PgVectorStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgVectorStorage")
            .field("namespace", &self.namespace)
            .field("dimension", &self.dimension)
            .field("table_name", &self.table_name)
            .finish()
    }
}

/// Return true if a pgvector `extversion` string is >= 0.8.0, the first release
/// that ships the iterative-scan GUCs.
///
/// WHY a tolerant parser: pgvector reports versions like `"0.7.4"` or `"0.8.0"`,
/// but a packaged build may append suffixes (e.g. pre-release/build metadata).
/// We compare only the leading `major.minor` numerics and treat anything we
/// cannot parse as unsupported (the safe default that avoids invalid-GUC errors).
fn pgvector_supports_iterative_scan(version: &str) -> bool {
    let mut parts = version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<u32>().ok());
    let major = parts.next();
    let minor = parts.next().unwrap_or(0);
    match major {
        Some(0) => minor >= 8,
        Some(_) => true, // 1.x and beyond
        None => false,   // unparsable -> conservative
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_embedding() {
        let embedding = vec![1.0, 2.0, 3.0];
        let formatted = PgVectorStorage::format_embedding(&embedding);
        assert_eq!(formatted, "[1,2,3]");
    }

    #[test]
    fn test_parse_embedding() {
        let text = "[1,2,3]";
        let parsed = PgVectorStorage::parse_embedding(text);
        assert_eq!(parsed, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_search_tuning_hnsw_clamps_ef_search() {
        // QW3: ef_search scales with top_k but is clamped to [40, 1000].
        let small =
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 1, false, true);
        assert_eq!(small, vec!["SET LOCAL hnsw.ef_search = 40"]);
        let mid = PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 50, false, true);
        assert_eq!(mid, vec!["SET LOCAL hnsw.ef_search = 200"]);
        let huge =
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 100_000, false, true);
        assert_eq!(huge, vec!["SET LOCAL hnsw.ef_search = 1000"]);
    }

    #[test]
    fn test_search_tuning_hnsw_filtered_enables_iterative_scan() {
        let stmts =
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 10, true, true);
        assert!(stmts.iter().any(|s| s.contains("hnsw.ef_search")));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.iterative_scan = strict_order"));
        assert!(stmts
            .iter()
            .any(|s| s == "SET LOCAL hnsw.max_scan_tuples = 20000"));
    }

    #[test]
    fn test_search_tuning_hnsw_filtered_without_iterative_scan_support() {
        // Edge case (pgvector < 0.8): filtered query must NOT emit the
        // iterative_scan/max_scan_tuples GUCs the server would reject.
        let stmts =
            PgVectorStorage::search_tuning_statements(VectorIndexType::HNSW, 10, true, false);
        assert!(stmts.iter().any(|s| s.contains("hnsw.ef_search")));
        assert!(!stmts.iter().any(|s| s.contains("iterative_scan")));
        assert!(!stmts.iter().any(|s| s.contains("max_scan_tuples")));
    }

    #[test]
    fn test_search_tuning_ivfflat() {
        let plain =
            PgVectorStorage::search_tuning_statements(VectorIndexType::IVFFlat, 5, false, true);
        assert_eq!(plain, vec!["SET LOCAL ivfflat.probes = 10"]);
        let filtered =
            PgVectorStorage::search_tuning_statements(VectorIndexType::IVFFlat, 5, true, true);
        assert!(filtered
            .iter()
            .any(|s| s == "SET LOCAL ivfflat.iterative_scan = relaxed_order"));
    }

    #[test]
    fn test_search_tuning_ivfflat_without_iterative_scan_support() {
        // Edge case (pgvector < 0.8): only `probes` is emitted.
        let filtered =
            PgVectorStorage::search_tuning_statements(VectorIndexType::IVFFlat, 5, true, false);
        assert_eq!(filtered, vec!["SET LOCAL ivfflat.probes = 10"]);
    }

    #[test]
    fn test_search_tuning_none_is_empty() {
        // No index -> no GUCs (sequential scan is exact anyway).
        let stmts =
            PgVectorStorage::search_tuning_statements(VectorIndexType::None, 100, true, true);
        assert!(stmts.is_empty());
    }

    #[test]
    fn test_pgvector_version_gate() {
        // >= 0.8.0 supports iterative scan; older does not. Be tolerant of
        // build-metadata / pre-release suffixes pgvector may report.
        assert!(pgvector_supports_iterative_scan("0.8.0"));
        assert!(pgvector_supports_iterative_scan("0.8.2"));
        assert!(pgvector_supports_iterative_scan("1.0.0"));
        assert!(pgvector_supports_iterative_scan("0.9"));
        assert!(!pgvector_supports_iterative_scan("0.7.4"));
        assert!(!pgvector_supports_iterative_scan("0.7"));
        assert!(!pgvector_supports_iterative_scan("0.5.1"));
        // Unparsable -> conservative false.
        assert!(!pgvector_supports_iterative_scan(""));
        assert!(!pgvector_supports_iterative_scan("garbage"));
    }
}
