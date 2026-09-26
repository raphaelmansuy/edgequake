//! Lineage GIN pending-list bound (SPEC-149 / Migration 157).
//!
//! Indexes can be (re)created by paths that predate the storage clause
//! (Migration 038/137 SQL, boot reconcile). `ensure_indexes` calls this so
//! every graph converges on [`LINEAGE_GIN_PENDING_LIST_LIMIT_KB`].

use crate::error::{Result, StorageError};

use super::super::PostgresAGEGraphStorage;
use super::source_lineage_sql::{LINEAGE_GIN_INDEXES, LINEAGE_GIN_PENDING_LIST_LIMIT_KB};

impl PostgresAGEGraphStorage {
    /// Set `gin_pending_list_limit` on lineage GIN indexes that lack it.
    ///
    /// Returns the number of indexes altered (0 once converged).
    pub(in crate::adapters::postgres::graph) async fn ensure_lineage_gin_pending_limit(
        &self,
        conn: &mut sqlx::PgConnection,
    ) -> Result<usize> {
        let wanted = format!("gin_pending_list_limit={LINEAGE_GIN_PENDING_LIST_LIMIT_KB}");
        let names: Vec<String> = LINEAGE_GIN_INDEXES.iter().map(|s| s.to_string()).collect();
        let stale: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT c.relname::text
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = $1
              AND c.relkind = 'i'
              AND c.relname = ANY($2::text[])
              AND NOT (COALESCE(c.reloptions, '{}') @> ARRAY[$3::text])
            ORDER BY c.relname
            "#,
        )
        .bind(&self.graph_name)
        .bind(&names)
        .bind(&wanted)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| StorageError::Database(format!("Lineage GIN reloption probe failed: {e}")))?;

        for index in &stale {
            let sql = format!(
                r#"ALTER INDEX "{graph}"."{index}" SET (gin_pending_list_limit = {limit})"#,
                graph = self.graph_name,
                limit = LINEAGE_GIN_PENDING_LIST_LIMIT_KB,
            );
            sqlx::query(&sql).execute(&mut *conn).await.map_err(|e| {
                StorageError::Database(format!("ALTER INDEX {index} pending limit failed: {e}"))
            })?;
            tracing::info!(
                graph = %self.graph_name,
                index = %index,
                limit_kb = LINEAGE_GIN_PENDING_LIST_LIMIT_KB,
                "SPEC-149: bounded lineage GIN pending list"
            );
        }
        Ok(stale.len())
    }
}
