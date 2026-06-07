use std::sync::atomic::Ordering;

use super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};

impl PostgresAGEGraphStorage {
    pub(super) fn pg_namespace(&self) -> &str {
        &self.namespace
    }

    pub(super) async fn pg_initialize(&self) -> Result<()> {
        if self.initialized.load(Ordering::Relaxed) {
            return Ok(());
        }

        self.pool.initialize().await?;
        self.create_graph().await?;

        // CRITICAL: Create indexes for query performance
        // Without these, Cypher queries like MATCH (n:Node {node_id: 'xxx'}) scan all vertices
        self.ensure_indexes().await?;

        self.initialized.store(true, Ordering::Relaxed);

        tracing::info!(
            "Initialized PostgresAGEGraphStorage with graph '{}' (indexes verified)",
            self.graph_name
        );

        Ok(())
    }

    pub(super) async fn pg_finalize(&self) -> Result<()> {
        Ok(())
    }
    pub(super) async fn pg_ping(&self) -> Result<()> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Probe the Node label table — succeeds even when empty (0 rows).
        let sql = format!(r#"SELECT 1 FROM {}."Node" LIMIT 1"#, self.graph_name);
        if sqlx::query(&sql).fetch_optional(&mut *conn).await.is_ok() {
            return Ok(());
        }

        // Graph may not be initialized yet — fall back to connection-level ping.
        sqlx::query("SELECT 1")
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Graph ping failed: {}", e)))?;

        Ok(())
    }
}
