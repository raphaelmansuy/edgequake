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
}
