//! AGE graph creation and index management (SPEC-017 P1-12).

use crate::error::{Result, StorageError};

use super::super::PostgresAGEGraphStorage;

impl PostgresAGEGraphStorage {
    pub(in crate::adapters::postgres::graph) async fn create_graph(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session(&mut conn).await?;

        let check_sql = format!(
            "SELECT 1 FROM ag_catalog.ag_graph WHERE name = '{}'",
            self.graph_name
        );

        let exists = sqlx::query(&check_sql)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Graph check failed: {}", e)))?;

        if exists.is_none() {
            let create_sql = format!(
                "SELECT * FROM ag_catalog.create_graph('{}')",
                self.graph_name
            );

            sqlx::query(&create_sql)
                .execute(&mut *conn)
                .await
                .map_err(|e| {
                    StorageError::Database(format!("Failed to create AGE graph: {}", e))
                })?;

            tracing::info!("Created AGE graph: {}", self.graph_name);
        }

        Ok(())
    }

    pub(in crate::adapters::postgres::graph) async fn ensure_indexes(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        Self::setup_age_session(&mut conn).await?;

        let index_queries = [
            (
                "idx_node_prop_node_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_node_prop_node_id 
                       ON {}."Node" (ag_catalog.agtype_access_operator(properties, '"node_id"'::agtype))"#,
                    self.graph_name
                ),
            ),
            (
                "idx_node_props_gin",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_node_props_gin 
                       ON {}."Node" USING gin(properties)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_node_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_node_id 
                       ON {}."Node" (id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_start_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_start_id 
                       ON {}."EDGE" (start_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_end_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_end_id 
                       ON {}."EDGE" (end_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_start_end",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_start_end 
                       ON {}."EDGE" (start_id, end_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_props_gin",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_props_gin 
                       ON {}."EDGE" USING gin(properties)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_vertex_props_gin",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_vertex_props_gin 
                       ON {}."_ag_label_vertex" USING gin(properties)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_edge_start_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_edge_start_id 
                       ON {}."_ag_label_edge" (start_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_edge_end_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_edge_end_id 
                       ON {}."_ag_label_edge" (end_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_edge_start_end",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_edge_start_end 
                       ON {}."_ag_label_edge" (start_id, end_id)"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_vertex_tenant_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_vertex_tenant_id 
                       ON {}."_ag_label_vertex" (
                         (ag_catalog.agtype_to_json(properties)->>'tenant_id')
                       )"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_vertex_workspace_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_vertex_workspace_id 
                       ON {}."_ag_label_vertex" (
                         (ag_catalog.agtype_to_json(properties)->>'workspace_id')
                       )"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_edge_source_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_edge_source_id 
                       ON {}."_ag_label_edge" (
                         (ag_catalog.agtype_to_json(properties)->>'source_id')
                       )"#,
                    self.graph_name
                ),
            ),
            (
                "idx_ag_edge_target_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_ag_edge_target_id 
                       ON {}."_ag_label_edge" (
                         (ag_catalog.agtype_to_json(properties)->>'target_id')
                       )"#,
                    self.graph_name
                ),
            ),
            (
                "idx_node_tenant_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_node_tenant_id 
                       ON {}."Node" (
                         (ag_catalog.agtype_to_json(properties)->>'tenant_id')
                       )"#,
                    self.graph_name
                ),
            ),
            (
                "idx_node_workspace_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_node_workspace_id 
                       ON {}."Node" (
                         (ag_catalog.agtype_to_json(properties)->>'workspace_id')
                       )"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_source_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_source_id 
                       ON {}."EDGE" (
                         (ag_catalog.agtype_to_json(properties)->>'source_id')
                       )"#,
                    self.graph_name
                ),
            ),
            (
                "idx_edge_target_id",
                format!(
                    r#"CREATE INDEX IF NOT EXISTS idx_edge_target_id 
                       ON {}."EDGE" (
                         (ag_catalog.agtype_to_json(properties)->>'target_id')
                       )"#,
                    self.graph_name
                ),
            ),
        ];

        let mut indexes_created = 0;
        let mut indexes_skipped = 0;

        for (name, sql) in &index_queries {
            match sqlx::query(sql).execute(&mut *conn).await {
                Ok(_) => {
                    indexes_created += 1;
                    tracing::debug!("Created/verified index: {}", name);
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("does not exist")
                        || err_str.contains("undefined_table")
                        || err_str.contains("relation")
                    {
                        indexes_skipped += 1;
                        tracing::debug!(
                            "Skipped index {} (table not yet created): {}",
                            name,
                            err_str
                        );
                    } else {
                        tracing::warn!(
                            error.source = "postgres_graph",
                            error.action = "create_index",
                            index = %name,
                            error.message = %e,
                            "Failed to create graph index"
                        );
                    }
                }
            }
        }

        if indexes_created > 0 {
            tracing::info!(
                "AGE graph indexes: {} created/verified, {} skipped (tables pending)",
                indexes_created,
                indexes_skipped
            );
        }

        Ok(())
    }

    /// Bootstrap critical indexes CONCURRENTLY for existing databases (SPEC-032 W-01).
    ///
    /// # First Principle: Non-Blocking Index Creation
    ///
    /// `ensure_indexes()` uses `CREATE INDEX IF NOT EXISTS` which acquires a
    /// `ShareLock` on the label table for the duration of the build. For a graph
    /// with 100K+ nodes this blocks all concurrent writes for minutes.
    ///
    /// `CONCURRENTLY` builds the index without holding a table lock: reads and
    /// writes continue normally while the index is built in the background
    /// (PostgreSQL §CREATE INDEX §CONCURRENTLY).
    ///
    /// # Constraint
    ///
    /// `CREATE INDEX CONCURRENTLY` cannot run inside a transaction block.
    /// This function acquires a pool connection and sends commands directly,
    /// NOT inside a `BEGIN ... COMMIT` envelope.
    ///
    /// # When to Call
    ///
    /// Called ONCE at startup after `pg_initialize()` when the graph already
    /// has rows (existing databases). The `concurrent_indexes_bootstrapped`
    /// atomic flag prevents repeated runs.
    ///
    /// # Edge Cases Handled
    ///
    /// - Graph does not exist yet → returns Ok(()) silently (no label tables)
    /// - Index already exists → `IF NOT EXISTS` is a no-op
    /// - Index is INVALID (partial build interrupted) → detected via pg_index,
    ///   dropped and rebuilt
    /// - AGE not installed → returns Ok(()) silently
    /// - Timeout during build → index is left INVALID, next startup retries
    pub(in crate::adapters::postgres::graph) async fn bootstrap_concurrent_indexes(
        &self,
    ) -> Result<()> {
        // Quick check: is AGE available?
        let pool = self.pool.get().await?;

        let age_available: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')",
        )
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

        if !age_available {
            tracing::debug!("AGE not installed — skipping concurrent index bootstrap");
            return Ok(());
        }

        // Check if the Node label table exists (graph may not have been used yet)
        let node_table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
               SELECT 1 FROM pg_class c
               JOIN pg_namespace n ON n.oid = c.relnamespace
               WHERE n.nspname = $1 AND c.relname = 'Node'
             )",
        )
        .bind(&self.graph_name)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

        if !node_table_exists {
            tracing::debug!(
                graph = %self.graph_name,
                "Node label table not yet created — skipping concurrent index bootstrap"
            );
            return Ok(());
        }

        // Check row count: if small, regular CREATE INDEX is fine; skip CONCURRENT
        let node_count: i64 = sqlx::query_scalar(&format!(
            r#"SELECT reltuples::bigint FROM pg_class c
               JOIN pg_namespace n ON n.oid = c.relnamespace
               WHERE n.nspname = $1 AND c.relname = 'Node'"#
        ))
        .bind(&self.graph_name)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

        // Below 10K rows: regular CREATE INDEX is fast enough (< 1 second).
        // Above 10K rows: use CONCURRENT to avoid blocking writes.
        const CONCURRENT_THRESHOLD: i64 = 10_000;
        let use_concurrent = node_count >= CONCURRENT_THRESHOLD;

        tracing::info!(
            graph = %self.graph_name,
            node_count,
            use_concurrent,
            "AGE graph bootstrap: checking critical indexes"
        );

        // ── Critical indexes for MERGE (n:Node {node_id: 'X'}) performance ──
        // These directly impact the cost of every entity upsert.
        let node_idx_name = "idx_node_prop_node_id_btree";
        let node_idx_sql = format!(
            r#"CREATE INDEX {concurrent} IF NOT EXISTS {name}
               ON {graph}."Node"
               ((ag_catalog.agtype_to_json(properties)->>'node_id'))"#,
            concurrent = if use_concurrent { "CONCURRENTLY" } else { "" },
            name = node_idx_name,
            graph = self.graph_name,
        );
        self.ensure_critical_index_concurrent(&pool, node_idx_name, &node_idx_sql, use_concurrent)
            .await;

        // EDGE source_id + target_id composite index
        let edge_table_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
               SELECT 1 FROM pg_class c
               JOIN pg_namespace n ON n.oid = c.relnamespace
               WHERE n.nspname = $1 AND c.relname = 'EDGE'
             )",
        )
        .bind(&self.graph_name)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

        if edge_table_exists {
            self.ensure_critical_index_concurrent(
                &pool,
                "idx_edge_source_target_btree",
                &format!(
                    r#"CREATE INDEX {concurrent} IF NOT EXISTS idx_edge_source_target_btree
                       ON {graph}."EDGE"
                       (
                         (ag_catalog.agtype_to_json(properties)->>'source_id'),
                         (ag_catalog.agtype_to_json(properties)->>'target_id')
                       )"#,
                    concurrent = if use_concurrent { "CONCURRENTLY" } else { "" },
                    graph = self.graph_name,
                ),
                use_concurrent,
            )
            .await;
        }

        Ok(())
    }

    /// Create or repair one critical index, handling INVALID state.
    async fn ensure_critical_index_concurrent(
        &self,
        pool: &sqlx::PgPool,
        index_name: &str,
        create_sql: &str,
        is_concurrent: bool,
    ) {
        // Check if index exists and is valid
        let index_state: Option<bool> = sqlx::query_scalar(
            "SELECT indisvalid FROM pg_index i
             JOIN pg_class c ON c.oid = i.indexrelid
             WHERE c.relname = $1",
        )
        .bind(index_name)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        match index_state {
            Some(true) => {
                tracing::debug!(index = %index_name, "Bootstrap: index already valid, skip");
                return;
            }
            Some(false) => {
                // INVALID index: drop it so we can rebuild
                tracing::warn!(
                    index = %index_name,
                    "Bootstrap: found INVALID index (interrupted build), dropping and rebuilding"
                );
                let drop_sql = format!("DROP INDEX CONCURRENTLY IF EXISTS {}", index_name);
                if let Err(e) = sqlx::query(&drop_sql).execute(pool).await {
                    tracing::warn!(
                        index = %index_name,
                        error = %e,
                        "Bootstrap: failed to drop INVALID index"
                    );
                    return;
                }
            }
            None => {
                tracing::debug!(index = %index_name, "Bootstrap: index missing, creating");
            }
        }

        // Run the CREATE INDEX [CONCURRENTLY] statement
        // NOTE: CONCURRENTLY requires NOT being in a transaction block.
        // We execute directly on the pool (not inside BEGIN/COMMIT).
        match sqlx::query(create_sql).execute(pool).await {
            Ok(_) => {
                tracing::info!(
                    index = %index_name,
                    concurrent = is_concurrent,
                    "Bootstrap: critical index created successfully"
                );
            }
            Err(e) => {
                tracing::warn!(
                    index = %index_name,
                    error = %e,
                    concurrent = is_concurrent,
                    "Bootstrap: failed to create critical index (non-fatal, will retry next startup)"
                );
            }
        }
    }
}
