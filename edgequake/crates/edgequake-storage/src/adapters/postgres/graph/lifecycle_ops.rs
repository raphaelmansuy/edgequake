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
        let pool = self.pool.get().await?;
        super::plain::refuse_cypher_opt_out(&pool).await?;
        let plain_tables = super::plain::tables_present(&pool).await?;
        if self.pool.graph_extension_available() {
            self.create_graph().await?;
            // SPEC-069: ensure_indexes is boot-owned + single-flight.
            self.ensure_indexes().await?;
            self.bootstrap_concurrent_indexes().await?;
            if !self.indexes_verified.load(Ordering::Acquire) {
                let mut conn = pool.acquire().await.map_err(|e| {
                    StorageError::Connection(format!("Failed to acquire connection: {}", e))
                })?;
                if self.eq_id_schema_ready(&mut conn).await? {
                    self.indexes_verified.store(true, Ordering::Release);
                }
            }
        } else if !plain_tables {
            return Err(StorageError::Database(
                "Apache AGE is unavailable and SPEC-164 plain graph tables are missing".into(),
            ));
        } else {
            self.indexes_verified.store(true, Ordering::Release);
        }
        if plain_tables {
            super::plain::register_graph(&pool, &self.graph_name, &self.namespace).await?;
        }

        self.initialized.store(true, Ordering::Relaxed);

        tracing::info!(
            graph = %self.graph_name,
            indexes_verified = self.indexes_verified.load(Ordering::Relaxed),
            "Initialized PostgresAGEGraphStorage (SPEC-069 boot-owned DDL)"
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

        if self.plain_reads_ready().await? {
            let ok =
                sqlx::query("SELECT 1 FROM public.eq_graph_nodes WHERE namespace = $1 LIMIT 1")
                    .bind(&self.namespace)
                    .fetch_optional(&mut *conn)
                    .await
                    .is_ok();
            if ok {
                return Ok(());
            }
        } else {
            let sql = format!(r#"SELECT 1 FROM {}."Node" LIMIT 1"#, self.graph_name);
            if sqlx::query(&sql).fetch_optional(&mut *conn).await.is_ok() {
                return Ok(());
            }
        }

        // Graph may not be initialized yet — fall back to connection-level ping.
        sqlx::query("SELECT 1")
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Graph ping failed: {}", e)))?;

        Ok(())
    }
}
