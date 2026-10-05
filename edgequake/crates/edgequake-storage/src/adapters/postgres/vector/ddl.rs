//! DDL and schema maintenance for [`super::PgVectorStorage`].

use super::super::capabilities::{AnnIndexPolicy, HNSW_MAX_DIM_HALFVEC};
use super::super::config::VectorIndexType;
use super::super::hnsw_runtime_policy::HnswRuntimePolicy;
use super::super::row_count_stats::{self, RowCountStatsConfig};
use super::super::schema;
use super::PgVectorStorage;
use crate::error::{Result, StorageError};
use crate::vector_backend::vector_backend_reads_typed;

/// SPEC-070: sanitize `maintenance_work_mem` / lock_timeout env values (digits + unit only).
fn sanitize_pg_setting(raw: &str, fallback: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        return fallback.to_string();
    }
    if t.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        t.to_string()
    } else {
        fallback.to_string()
    }
}

impl PgVectorStorage {
    /// SPEC-070 / SPEC-090 F-090-07: DDL session GUCs for index builds.
    ///
    /// Prefer `SET LOCAL` inside an open transaction. `CREATE INDEX CONCURRENTLY`
    /// cannot run in a TX, so that path uses session `SET` on a dedicated
    /// connection that is `DISCARD ALL`'d on pool release.
    pub(crate) async fn setup_vector_ddl_session(conn: &mut sqlx::PgConnection) -> Result<()> {
        Self::apply_vector_ddl_gucs(conn, false).await
    }

    /// Apply DDL GUCs; `local=true` requires an open transaction (`SET LOCAL`).
    pub(crate) async fn apply_vector_ddl_gucs(
        conn: &mut sqlx::PgConnection,
        local: bool,
    ) -> Result<()> {
        let lock_timeout = sanitize_pg_setting(
            &std::env::var("EDGEQUAKE_GRAPH_DDL_LOCK_TIMEOUT").unwrap_or_default(),
            "5s",
        );
        let maint_mem = sanitize_pg_setting(
            &std::env::var("EDGEQUAKE_INDEX_MAINTENANCE_WORK_MEM").unwrap_or_default(),
            "256MB",
        );
        let set = if local { "SET LOCAL" } else { "SET" };
        sqlx::query(&format!("{set} statement_timeout = 0"))
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to clear statement_timeout for vector DDL: {e}"
                ))
            })?;
        sqlx::query(&format!("{set} lock_timeout = '{lock_timeout}'"))
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to set vector DDL lock_timeout: {e}"))
            })?;
        sqlx::query(&format!("{set} maintenance_work_mem = '{maint_mem}'"))
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to set maintenance_work_mem for vector DDL: {e}"
                ))
            })?;
        Ok(())
    }

    /// Run index/DDL SQL on a dedicated connection with vector DDL session GUCs.
    ///
    /// SPEC-090 F-090-08: non-empty tables use `CREATE INDEX CONCURRENTLY` outside
    /// a transaction; empty tables may use blocking `CREATE INDEX`. INVALID leftovers
    /// from interrupted builds are dropped before retry.
    async fn execute_index_ddl(pool: &sqlx::PgPool, sql: &str) -> Result<()> {
        let (index_name, table_name) = parse_create_index_target(sql).ok_or_else(|| {
            StorageError::Database(format!("Cannot parse vector index DDL: {sql}"))
        })?;
        let table_only = table_name
            .split('.')
            .next_back()
            .unwrap_or(table_name)
            .trim_matches('"');

        let index_state = Self::vector_index_validity(pool, index_name).await;
        match index_state {
            Some(true) => {
                tracing::debug!(index = %index_name, "Vector index already valid, skip");
                return Ok(());
            }
            Some(false) => {
                tracing::warn!(
                    index = %index_name,
                    table = %table_name,
                    "Vector index INVALID (interrupted build), dropping before rebuild"
                );
                let mut conn = pool.acquire().await.map_err(|e| {
                    StorageError::Connection(format!(
                        "Failed to acquire connection for INVALID index drop: {e}"
                    ))
                })?;
                Self::setup_vector_ddl_session(&mut conn).await?;
                let drop_sql = format!("DROP INDEX CONCURRENTLY IF EXISTS {index_name}");
                if let Err(e) = sqlx::query(&drop_sql).execute(&mut *conn).await {
                    tracing::warn!(
                        index = %index_name,
                        error = %e,
                        "Failed to drop INVALID vector index"
                    );
                    return Err(StorageError::Database(format!(
                        "Failed to drop INVALID vector index {index_name}: {e}"
                    )));
                }
            }
            None => {}
        }

        let row_estimate = Self::table_row_estimate(pool, table_only).await?;
        let use_concurrent = row_estimate > 0;
        let final_sql = if use_concurrent {
            sql.replace(
                "CREATE INDEX IF NOT EXISTS",
                "CREATE INDEX CONCURRENTLY IF NOT EXISTS",
            )
        } else {
            sql.to_string()
        };

        if use_concurrent {
            // CIC cannot run inside a transaction — session SET + DISCARD ALL on release.
            let mut conn = pool.acquire().await.map_err(|e| {
                StorageError::Connection(format!(
                    "Failed to acquire connection for vector DDL: {e}"
                ))
            })?;
            Self::apply_vector_ddl_gucs(&mut conn, false).await?;
            sqlx::query(&final_sql)
                .execute(&mut *conn)
                .await
                .map_err(|e| StorageError::Database(format!("Vector index DDL failed: {e}")))?;
        } else {
            let mut tx = pool.begin().await.map_err(|e| {
                StorageError::Connection(format!("Failed to begin TX for vector DDL: {e}"))
            })?;
            Self::apply_vector_ddl_gucs(&mut tx, true).await?;
            sqlx::query(&final_sql)
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("Vector index DDL failed: {e}")))?;
            tx.commit()
                .await
                .map_err(|e| StorageError::Database(format!("Vector DDL commit failed: {e}")))?;
        }
        Ok(())
    }

    /// `Some(true)` = valid index, `Some(false)` = INVALID, `None` = missing.
    async fn vector_index_validity(pool: &sqlx::PgPool, index_name: &str) -> Option<bool> {
        sqlx::query_scalar(
            "SELECT i.indisvalid FROM pg_index i
             JOIN pg_class c ON c.oid = i.indexrelid
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE c.relname = $1 AND n.nspname = 'public'",
        )
        .bind(index_name)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
    }

    /// Estimated row count from pg_class (0 for empty / new tables).
    async fn table_row_estimate(pool: &sqlx::PgPool, table_name: &str) -> Result<i64> {
        let estimate: i64 = sqlx::query_scalar(
            "SELECT COALESCE(c.reltuples::bigint, 0)
             FROM pg_class c
             JOIN pg_namespace n ON n.oid = c.relnamespace
             WHERE n.nspname = 'public' AND c.relname = $1",
        )
        .bind(table_name)
        .fetch_optional(pool)
        .await
        .map_err(|e| StorageError::Database(format!("table_row_estimate failed: {e}")))?
        .unwrap_or(0);
        Ok(estimate.max(0))
    }
    /// Create the vectors table and indexes.
    /// SPEC-091 IW2 (LD-03): whether runtime DDL for the entire legacy vector
    /// fleet is retired (migration 131 applied + typed read backend).
    pub(crate) async fn legacy_vector_ddl_retired(&self) -> bool {
        if !vector_backend_reads_typed(crate::vector_backend_from_env()) {
            return false;
        }
        let Ok(pool) = self.pool.get().await else {
            return false;
        };
        sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 131 AND success)",
        )
        .fetch_one(&pool)
        .await
        .unwrap_or(false)
    }

    /// SPEC-091 W4 (LD-03): whether runtime DDL may (re)create this legacy
    /// `eq_*_vectors` table. Once the chunk fleet is retired (read backend
    /// authoritative on typed `chunk_embeddings` AND migration 126 applied),
    /// a chunk-dedicated legacy table must NOT be recreated. Migration 131
    /// retires runtime DDL for the entire fleet.
    ///
    /// Fail-safe: any probe error / missing table ⇒ **allow** creation
    /// (pre-retirement behavior). Only retire when the table **exists** and
    /// has zero non-chunk rows (or fleet DDL is already retired via 131).
    pub(crate) async fn legacy_chunk_ddl_retired(&self) -> bool {
        if self.legacy_vector_ddl_retired().await {
            return true;
        }
        // Only the typed backend retires chunk runtime DDL.
        if !vector_backend_reads_typed(crate::vector_backend_from_env()) {
            return false;
        }
        let Ok(pool) = self.pool.get().await else {
            return false;
        };
        // Migration 126 applied? (its version row in sqlx's ledger)
        let applied_126: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 126 AND success)",
        )
        .fetch_one(&pool)
        .await
        .unwrap_or(false);
        if !applied_126 {
            return false;
        }
        // Retire DDL only for chunk-dedicated tables that still exist.
        // Missing table ⇒ not retired — allow CREATE on legacy_tables rollback
        // / pre-write-stop paths (never treat never-created workspace tables
        // as dropped). Probe first so Postgres never logs 42P01 (#405).
        if !self
            .legacy_vectors_relation_exists_cached()
            .await
            .unwrap_or(false)
        {
            return false;
        }
        let non_chunk: std::result::Result<i64, sqlx::Error> = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {} WHERE id NOT LIKE '%-chunk-%'",
            self.table_name
        ))
        .fetch_one(&pool)
        .await;
        match non_chunk {
            Ok(0) => true,
            Ok(_) => false,
            Err(_) => false,
        }
    }

    pub(crate) async fn create_table(&self) -> Result<()> {
        // SPEC-091 RM1 (RM-AC-04): typed default write-stops legacy CREATE even
        // before migration 131 is recorded — never recreate eq_*_vectors under
        // typed authority.
        if crate::legacy_vector_writes_stopped() || self.legacy_vector_ddl_retired().await {
            tracing::debug!(
                table = %self.table_name,
                "SPEC-091 RM1: typed/legacy-retired — skipping runtime create_table (LD-03)"
            );
            return Ok(());
        }
        let pool = self.pool.get().await?;

        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Failed to create vector extension: {}", e))
            })?;

        let policy = AnnIndexPolicy::resolve(self.dimension, self.storage_mode);
        let emb_type = self.embedding_pg_type();
        let opclass = self.embedding_opclass();
        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                embedding {}({}) NOT NULL,
                metadata JSONB DEFAULT '{{}}',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name, emb_type, self.dimension
        );

        sqlx::query(&sql).execute(&pool).await.map_err(|e| {
            StorageError::Database(format!("Failed to create vectors table: {}", e))
        })?;

        let index_sql = match self.index_type {
            VectorIndexType::IVFFlat if policy.hnsw_viable => format!(
                "CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx ON {} USING ivfflat (embedding {}) WITH (lists = {})",
                self.prefix, self.table_name, opclass, self.ivfflat_lists
            ),
            VectorIndexType::HNSW if policy.hnsw_viable => format!(
                "CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx ON {} USING hnsw (embedding {}) WITH (m = {}, ef_construction = {})",
                self.prefix, self.table_name, opclass, self.hnsw_m, self.hnsw_ef_construction
            ),
            VectorIndexType::IVFFlat | VectorIndexType::HNSW => {
                tracing::warn!(
                    table = %self.table_name,
                    dimension = self.dimension,
                    max_hnsw_dim = HNSW_MAX_DIM_HALFVEC,
                    "Skipping ANN index — embedding dimension exceeds pgvector HNSW limits (issue #275)"
                );
                String::new()
            }
            VectorIndexType::None => String::new(),
        };

        if !index_sql.is_empty() {
            // SPEC-046 OPS-P0.3: fail-closed — never swallow ANN index DDL errors.
            // Missing HNSW silently degrades to seq-scan (latency/recall cliff).
            // SPEC-070: DDL session GUCs on dedicated connection.
            Self::execute_index_ddl(&pool, &index_sql)
                .await
                .map_err(|e| {
                    StorageError::Database(format!(
                        "Failed to create ANN index on {}: {}",
                        self.table_name, e
                    ))
                })?;
        }

        // SPEC-034 IMP-08: Vector metadata GIN index removed.
        // WHY: 0 query scans — all metadata lookups use metadata->>'key' = value
        // (equality on extracted text), served by doc_id_idx / tenant_ws_idx btrees.
        // This was 13 MB per workspace with zero benefit.
        // To restore: uncomment the line below.
        // sqlx::query(&format!(
        //     "CREATE INDEX IF NOT EXISTS eq_{}_vectors_metadata_idx ON {} USING GIN (metadata jsonb_path_ops)",
        //     self.prefix, self.table_name
        // )).execute(&pool).await.ok();

        let add_cols = format!(
            r#"
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS document_id TEXT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS tenant_id TEXT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS workspace_id TEXT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS embedding_model TEXT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS embedding_dim INT;
            ALTER TABLE {} ADD COLUMN IF NOT EXISTS embedding_norm TEXT
            "#,
            self.table_name,
            self.table_name,
            self.table_name,
            self.table_name,
            self.table_name,
            self.table_name
        );
        for stmt in add_cols.split(';').filter(|s| !s.trim().is_empty()) {
            sqlx::query(stmt.trim()).execute(&pool).await.ok();
        }

        let doc_idx = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_doc_id_idx ON {} (document_id) WHERE document_id IS NOT NULL",
            self.prefix, self.table_name
        );
        let _ = Self::execute_index_ddl(&pool, &doc_idx).await;

        let tenant_idx = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_tenant_ws_idx ON {} (tenant_id, workspace_id) WHERE tenant_id IS NOT NULL",
            self.prefix, self.table_name
        );
        let _ = Self::execute_index_ddl(&pool, &tenant_idx).await;

        self.ensure_content_fts(&pool).await?;

        self.ensure_row_count_stats(&pool).await?;

        Ok(())
    }

    pub(crate) async fn ensure_row_count_stats(&self, pool: &sqlx::PgPool) -> Result<()> {
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

    /// Drop the vectors table if it exists.
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

    /// Check if the table exists in the database.
    pub async fn table_exists(&self) -> Result<bool> {
        let pool = match self.pool.get().await {
            Ok(p) => p,
            Err(_) => return Ok(false),
        };

        schema::relation_exists(&pool, &self.table_name).await
    }

    /// Name of the ANN embedding index for this table (SPEC-046 OPS-P0.3).
    pub fn ann_index_name(&self) -> String {
        format!("eq_{}_vectors_embedding_idx", self.prefix)
    }

    /// SPEC-062: build HNSW/IVFFlat after a deferred bulk load (`VectorIndexType::None`).
    ///
    /// Cold ingest pattern: create table without ANN → upsert heap rows → `ensure_ann_index`.
    /// Online ingest still creates HNSW at `initialize()` time (pays insert tax).
    pub async fn ensure_ann_index(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let policy = AnnIndexPolicy::resolve(self.dimension, self.storage_mode);
        if !policy.hnsw_viable {
            return Ok(());
        }
        let opclass = self.embedding_opclass();
        let index_sql = match self.index_type {
            VectorIndexType::None | VectorIndexType::HNSW => format!(
                "CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx ON {} USING hnsw (embedding {}) WITH (m = {}, ef_construction = {})",
                self.prefix, self.table_name, opclass, self.hnsw_m, self.hnsw_ef_construction
            ),
            VectorIndexType::IVFFlat => format!(
                "CREATE INDEX IF NOT EXISTS eq_{}_vectors_embedding_idx ON {} USING ivfflat (embedding {}) WITH (lists = {})",
                self.prefix, self.table_name, opclass, self.ivfflat_lists
            ),
        };
        Self::execute_index_ddl(&pool, &index_sql)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to ensure ANN index on {}: {}",
                    self.table_name, e
                ))
            })?;
        self.mark_deferred_ann_ready();
        Ok(())
    }

    /// SPEC-064 Wave 2: workspace-scoped partial HNSW (opt-in / explicit hot-workspace path).
    ///
    /// Builds `WHERE workspace_id = $ws` so filtered ANN walks a smaller graph instead of
    /// over-filtering a global HNSW via `hnsw.iterative_scan`. Prefer for hot workspaces;
    /// keep global `ensure_ann_index` for sparse/small workspaces.
    ///
    /// Gate: call after heap load. Battle harness drops the global ANN first so the planner
    /// cannot prefer the larger index. Production callers should set
    /// `EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE=1` before creating hot-workspace partials.
    pub async fn ensure_partial_hnsw_for_workspace(&self, workspace_id: &str) -> Result<()> {
        if workspace_id.is_empty() {
            return Err(StorageError::Database(
                "ensure_partial_hnsw_for_workspace: empty workspace_id".into(),
            ));
        }
        let pool = self.pool.get().await?;
        let policy = AnnIndexPolicy::resolve(self.dimension, self.storage_mode);
        if !policy.hnsw_viable {
            return Ok(());
        }
        let opclass = self.embedding_opclass();
        let index_name = self.partial_ann_index_name(workspace_id);
        let lit = sql_string_literal(workspace_id);
        let index_sql = format!(
            "CREATE INDEX IF NOT EXISTS {index_name} ON {} USING hnsw (embedding {}) WITH (m = {}, ef_construction = {}) WHERE workspace_id = {lit}",
            self.table_name, opclass, self.hnsw_m, self.hnsw_ef_construction
        );
        Self::execute_index_ddl(&pool, &index_sql)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to create partial HNSW {index_name} on {}: {}",
                    self.table_name, e
                ))
            })?;
        self.mark_deferred_ann_ready();
        tracing::info!(
            table = %self.table_name,
            index = %index_name,
            workspace_id,
            "Ensured workspace partial HNSW (SPEC-064)"
        );
        Ok(())
    }

    /// Drop the global ANN index (battle / rebuild helper). Partial indexes are left intact.
    pub async fn drop_global_ann_index(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let index_name = self.ann_index_name();
        let sql = format!("DROP INDEX IF EXISTS {index_name}");
        sqlx::query(&sql).execute(&pool).await.map_err(|e| {
            StorageError::Database(format!("Failed to drop ANN index {index_name}: {e}"))
        })?;
        Ok(())
    }

    /// Name of a workspace partial HNSW index.
    pub fn partial_ann_index_name(&self, workspace_id: &str) -> String {
        format!(
            "eq_{}_vectors_hnsw_ws_{}",
            self.prefix,
            workspace_index_slug(workspace_id)
        )
    }

    /// True when HNSW/IVFFlat index exists (fail-closed readiness probe).
    ///
    /// SPEC-065: accepts **global** ANN **or** any workspace partial HNSW on this table.
    /// `VectorIndexType::None` (deferred create) is not a readiness failure until/unless
    /// callers expect an index — then catalog probe still reports truth.
    pub async fn ann_index_exists(&self) -> Result<bool> {
        let policy = AnnIndexPolicy::resolve(self.dimension, self.storage_mode);
        if !policy.hnsw_viable {
            return Ok(true); // ANN not expected
        }
        if matches!(self.index_type, VectorIndexType::None)
            && !self
                .deferred_ann_ready
                .load(std::sync::atomic::Ordering::Acquire)
        {
            // Deferred heap load — not a readiness failure yet.
            return Ok(true);
        }
        let pool = self.pool.get().await?;
        let global = self.ann_index_name();
        let partial_prefix = format!("eq_{}_vectors_hnsw_ws_", self.prefix);
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM pg_indexes
                WHERE schemaname = 'public'
                  AND (indexname = $1 OR indexname LIKE $2)
            )",
        )
        .bind(&global)
        .bind(format!("{partial_prefix}%"))
        .fetch_one(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("ann_index_exists probe failed: {e}")))?;
        Ok(exists)
    }

    /// True when this table is a dedicated per-workspace table (`*_ws_*` namespace).
    pub fn is_dedicated_workspace_table(&self) -> bool {
        self.prefix.contains("_ws_") || self.table_name.contains("_ws_")
    }

    /// Count rows for a workspace (denorm column).
    pub async fn count_workspace_rows(&self, workspace_id: &str) -> Result<u64> {
        if self
            .skip_legacy_read_if_absent("count_workspace_rows")
            .await?
        {
            return Ok(0);
        }
        let pool = self.pool.get().await?;
        let sql = format!(
            "SELECT COUNT(*)::bigint FROM {} WHERE workspace_id = $1",
            self.table_name
        );
        let n: i64 = sqlx::query_scalar(&sql)
            .bind(workspace_id)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("count_workspace_rows failed: {e}")))?;
        Ok(n.max(0) as u64)
    }

    /// True when the workspace partial HNSW exists in the catalog.
    pub async fn partial_ann_index_exists(&self, workspace_id: &str) -> Result<bool> {
        let pool = self.pool.get().await?;
        let index_name = self.partial_ann_index_name(workspace_id);
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM pg_indexes
                WHERE schemaname = 'public' AND indexname = $1
            )",
        )
        .bind(&index_name)
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            StorageError::Database(format!("partial_ann_index_exists probe failed: {e}"))
        })?;
        Ok(exists)
    }

    /// SPEC-065: productized Wave-2 path — create partial HNSW for a hot workspace
    /// when opt-in is on, table is shared (multi-WS), and row count >= threshold.
    ///
    /// Dedicated per-workspace tables are a no-op (already isolated by table).
    /// Mutual exclusivity: shared tables use partial OR global HNSW per workspace —
    /// when partial exists, query path prefers partial only (no dual-index writes).
    pub async fn ensure_hot_workspace_ann(&self, workspace_id: &str) -> Result<bool> {
        let runtime = HnswRuntimePolicy::from_env();
        if !runtime.partial_by_workspace {
            return Ok(false);
        }
        if workspace_id.is_empty() {
            return Err(StorageError::Database(
                "ensure_hot_workspace_ann: empty workspace_id".into(),
            ));
        }
        if self.is_dedicated_workspace_table() {
            tracing::debug!(
                table = %self.table_name,
                "Skipping partial HNSW — dedicated workspace table"
            );
            return Ok(false);
        }
        if self.partial_ann_index_exists(workspace_id).await? {
            self.mark_deferred_ann_ready();
            return Ok(false);
        }
        let rows = self.count_workspace_rows(workspace_id).await?;
        if rows < runtime.partial_min_rows {
            tracing::debug!(
                workspace_id,
                rows,
                min = runtime.partial_min_rows,
                "Skipping partial HNSW — below row threshold"
            );
            return Ok(false);
        }
        let policy = AnnIndexPolicy::resolve(self.dimension, self.storage_mode);
        if !policy.hnsw_viable {
            return Ok(false);
        }
        self.ensure_partial_hnsw_for_workspace(workspace_id).await?;
        // Fail-closed: catalog must show the partial after DDL (no silent seq-scan path).
        if !self.partial_ann_index_exists(workspace_id).await? {
            return Err(StorageError::Database(format!(
                "ensure_hot_workspace_ann: partial HNSW missing after CREATE for workspace {workspace_id}"
            )));
        }
        // SPEC-090 F-090-25: register hot WS and exclude it from the global HNSW.
        self.register_hot_ann_workspace(workspace_id).await?;
        self.rebuild_global_ann_excluding_hot_workspaces().await?;
        Ok(true)
    }

    /// Record workspace as mutual-exclusive with the global HNSW (F-090-25).
    async fn register_hot_ann_workspace(&self, workspace_id: &str) -> Result<()> {
        // SPEC-091 RM1: never runtime-CREATE hot ANN registry under typed authority.
        if crate::legacy_vector_writes_stopped() || self.legacy_vector_ddl_retired().await {
            return Ok(());
        }
        let pool = self.pool.get().await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS eq_hot_ann_workspaces (
              table_prefix TEXT NOT NULL,
              workspace_id TEXT NOT NULL,
              created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
              PRIMARY KEY (table_prefix, workspace_id)
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("create hot ANN registry failed: {e}")))?;
        sqlx::query(
            r#"
            INSERT INTO eq_hot_ann_workspaces (table_prefix, workspace_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&self.prefix)
        .bind(workspace_id)
        .execute(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("register_hot_ann_workspace failed: {e}")))?;
        Ok(())
    }

    /// Rebuild global HNSW as `WHERE workspace_id NOT IN (hot set)` (F-090-25).
    async fn rebuild_global_ann_excluding_hot_workspaces(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let policy = AnnIndexPolicy::resolve(self.dimension, self.storage_mode);
        if !policy.hnsw_viable {
            return Ok(());
        }
        let hot: Vec<String> = sqlx::query_scalar(
            "SELECT workspace_id FROM eq_hot_ann_workspaces WHERE table_prefix = $1",
        )
        .bind(&self.prefix)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("read hot ANN registry failed: {e}")))?;
        if hot.is_empty() {
            return Ok(());
        }
        let global = self.ann_index_name();
        // Drop then recreate with exclusion predicate (CIC path via execute_index_ddl).
        let drop_sql = format!("DROP INDEX CONCURRENTLY IF EXISTS {global}");
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("acquire for global HNSW rebuild: {e}"))
        })?;
        Self::apply_vector_ddl_gucs(&mut conn, false).await?;
        let _ = sqlx::query(&drop_sql).execute(&mut *conn).await;
        drop(conn);

        let opclass = self.embedding_opclass();
        let lit_list: Vec<String> = hot.iter().map(|w| sql_string_literal(w)).collect();
        let not_in = lit_list.join(", ");
        let index_sql = format!(
            "CREATE INDEX IF NOT EXISTS {global} ON {} USING hnsw (embedding {}) \
             WITH (m = {}, ef_construction = {}) \
             WHERE workspace_id IS NULL OR workspace_id NOT IN ({not_in})",
            self.table_name, opclass, self.hnsw_m, self.hnsw_ef_construction
        );
        Self::execute_index_ddl(&pool, &index_sql)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "rebuild global HNSW excluding hot workspaces failed: {e}"
                ))
            })?;
        tracing::info!(
            table = %self.table_name,
            hot_workspaces = hot.len(),
            "SPEC-090: global HNSW rebuilt excluding hot workspaces (mutual exclusive)"
        );
        Ok(())
    }

    /// Count vector tables that have no HNSW/IVFFlat index (bootstrap readiness).
    pub async fn count_vector_tables_missing_ann_index(pool: &sqlx::PgPool) -> Result<usize> {
        let missing: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)::bigint
            FROM pg_tables t
            WHERE t.schemaname = 'public'
              AND t.tablename LIKE 'eq\_%\_vectors' ESCAPE '\'
              AND t.tablename NOT LIKE '%\_stats'
              AND NOT EXISTS (
                SELECT 1
                FROM pg_indexes i
                WHERE i.schemaname = 'public'
                  AND i.tablename = t.tablename
                  AND (
                    i.indexname = (t.tablename || '_embedding_idx')
                    OR i.indexdef ILIKE '% USING hnsw %'
                    OR i.indexdef ILIKE '% USING ivfflat %'
                  )
              )
            "#,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| StorageError::Database(format!("missing ANN index scan failed: {e}")))?;
        Ok(missing.max(0) as usize)
    }

    /// Add writable GIN-backed `content_tsv` for native Postgres FTS (SPEC-023 I10 / SPEC-058).
    ///
    /// WHY writable (not GENERATED): chunk SSOT is KV via `content_ref`; generated
    /// columns from `metadata->>'content'` stay empty and block coalesce fallthrough.
    pub(crate) async fn ensure_content_fts(&self, pool: &sqlx::PgPool) -> Result<()> {
        let table_only = self
            .table_name
            .split('.')
            .next_back()
            .unwrap_or(&self.table_name);

        let ensure_col = format!(
            r#"
            DO $$
            DECLARE
                gen text;
            BEGIN
                SELECT a.attgenerated INTO gen
                FROM pg_attribute a
                JOIN pg_class c ON c.oid = a.attrelid
                JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE n.nspname = 'public'
                  AND c.relname = '{table_only}'
                  AND a.attname = 'content_tsv'
                  AND a.attnum > 0
                  AND NOT a.attisdropped;

                IF gen IS NULL THEN
                    ALTER TABLE {table}
                    ADD COLUMN content_tsv TSVECTOR;
                ELSIF gen <> '' THEN
                    ALTER TABLE {table} DROP COLUMN content_tsv;
                    ALTER TABLE {table}
                    ADD COLUMN content_tsv TSVECTOR;
                END IF;
            END $$;
            "#,
            table_only = table_only,
            table = self.table_name
        );

        sqlx::query(&ensure_col).execute(pool).await.ok();

        let fts_idx = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_vectors_content_tsv_idx ON {} USING GIN (content_tsv)",
            self.prefix, self.table_name
        );
        // SPEC-070: GIN build also uses DDL session (timeout=0 + maintenance_work_mem).
        let _ = Self::execute_index_ddl(pool, &fts_idx).await;

        Ok(())
    }
}

/// Parse `CREATE INDEX IF NOT EXISTS {name} ON {table} ...` for F-090-08 routing.
fn parse_create_index_target(sql: &str) -> Option<(&str, &str)> {
    let rest = sql.trim().strip_prefix("CREATE INDEX IF NOT EXISTS ")?;
    let (index_name, rest) = rest.split_once(" ON ")?;
    let table = rest.split_whitespace().next()?;
    Some((index_name.trim(), table.trim()))
}

/// Stable, index-safe slug for workspace_id (alnum/_ + short hash).
fn workspace_index_slug(workspace_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let safe: String = workspace_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .take(24)
        .collect();
    let mut hasher = DefaultHasher::new();
    workspace_id.hash(&mut hasher);
    format!("{safe}_{:04x}", hasher.finish() as u16)
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_index_slug_is_stable_and_safe() {
        let a = workspace_index_slug("ws-a");
        let b = workspace_index_slug("ws-a");
        assert_eq!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        assert!(!workspace_index_slug("ws';a").contains('\''));
    }

    #[test]
    fn sql_string_literal_escapes_quotes() {
        assert_eq!(sql_string_literal("o'reilly"), "'o''reilly'");
    }
}
