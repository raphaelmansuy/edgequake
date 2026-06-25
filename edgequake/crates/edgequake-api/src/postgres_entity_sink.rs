//! PostgreSQL implementation of `RelationalEntitySink` (SPEC-021 P3-01/P3-02).
//!
//! This module provides the concrete sink that writes entity data to the
//! relational `entities` table, enabling the CQRS dual-write pattern when
//! `entity_sync_mode != "disabled"` in server_config.
//!
//! # WHY here (not in edgequake-pipeline or edgequake-storage)
//!
//! The `edgequake-pipeline` crate is sqlx-free (it only uses `edgequake-storage`
//! traits). The `RelationalEntitySink` trait lives in pipeline, but the sqlx
//! implementation lives here where `PgPool` is available.
//!
//! # Ascending Compatibility
//!
//! The sink checks `entity_sync_mode` at construction time. If the mode is
//! "disabled" or the server_config row is absent, the sink acts as a no-op.
//! This ensures zero regression on deployments that haven't run migration 039.

use std::sync::Arc;

use async_trait::async_trait;
use edgequake_pipeline::{NoopEntitySink, RelationalEntitySink};
use sqlx::PgPool;
use tracing::{debug, warn};

/// PostgreSQL-backed relational entity sink.
///
/// Writes entity upserts and source_chunk_ids updates to the `entities` table
/// when `entity_sync_mode` is `dual_write` or `full`.
pub struct PostgresEntitySink {
    pool: Arc<PgPool>,
}

impl PostgresEntitySink {
    /// Create a new sink. Call `create_if_enabled()` instead for feature-gated usage.
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Create the appropriate sink based on `entity_sync_mode` in server_config.
    ///
    /// Returns:
    /// - `PostgresEntitySink` when mode is `dual_write` or `full`
    /// - `NoopEntitySink` when mode is `disabled` or config is absent
    pub async fn create_if_enabled(pool: Arc<PgPool>) -> Arc<dyn RelationalEntitySink> {
        let mode: Option<String> = sqlx::query_scalar(
            "SELECT value::text FROM server_config WHERE key = 'entity_sync_mode'",
        )
        .fetch_optional(pool.as_ref())
        .await
        .unwrap_or(None);

        let mode_str = mode.as_deref().unwrap_or("\"disabled\"");
        let enabled = mode_str.contains("dual_write") || mode_str.contains("full");

        if enabled {
            tracing::info!(
                entity_sync_mode = %mode_str,
                "CQRS entity dual-write ENABLED (SPEC-021 P3-01)"
            );
            Arc::new(Self { pool })
        } else {
            tracing::info!(
                entity_sync_mode = %mode_str,
                "CQRS entity dual-write disabled (entity_sync_mode != dual_write|full)"
            );
            Arc::new(NoopEntitySink)
        }
    }
}

#[async_trait]
impl RelationalEntitySink for PostgresEntitySink {
    /// Upsert entity into relational CQRS read model.
    ///
    /// Uses ON CONFLICT DO UPDATE (last-write-wins) to stay idempotent.
    /// `source_chunk_ids` are merged with existing array via `||` operator.
    async fn upsert_entity(
        &self,
        name: &str,
        entity_type: &str,
        description: &str,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
        source_chunk_ids: &[String],
    ) -> edgequake_pipeline::Result<()> {
        let tenant_uuid: Option<uuid::Uuid> = tenant_id.and_then(|t| t.parse().ok());
        let workspace_uuid: Option<uuid::Uuid> = workspace_id.and_then(|w| w.parse().ok());

        let result = sqlx::query(
            r#"INSERT INTO entities
                   (name, entity_type, description, tenant_id, workspace_id,
                    source_chunk_ids, sync_status, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, 'synced', NOW(), NOW())
               ON CONFLICT (tenant_id, workspace_id, name) DO UPDATE SET
                   entity_type      = EXCLUDED.entity_type,
                   description      = EXCLUDED.description,
                   source_chunk_ids = (
                       SELECT array_agg(DISTINCT elem)
                       FROM unnest(entities.source_chunk_ids || EXCLUDED.source_chunk_ids) AS t(elem)
                   ),
                   sync_status = 'synced',
                   updated_at  = NOW()"#
        )
        .bind(name)
        .bind(entity_type)
        .bind(description)
        .bind(tenant_uuid)
        .bind(workspace_uuid)
        .bind(source_chunk_ids)
        .execute(self.pool.as_ref())
        .await;

        match result {
            Ok(_) => {
                debug!(entity = %name, "Relational entity sync OK");
                Ok(())
            }
            Err(e) => {
                // Schema column may not exist yet (migration 039 not applied)
                // Map to a warning + Ok(()) to never fail ingestion
                warn!(entity = %name, error = %e, "Relational entity upsert failed (best-effort)");
                Ok(())
            }
        }
    }

    /// Update source_chunk_ids when a document is deleted.
    ///
    /// If `remaining_sources` is empty: DELETE the entity row.
    /// If non-empty: UPDATE source_chunk_ids to the remaining list.
    async fn remove_entity_sources(
        &self,
        name: &str,
        workspace_id: Option<&str>,
        _sources_to_remove: &[String],
        remaining_sources: &[String],
    ) -> edgequake_pipeline::Result<()> {
        let workspace_uuid: Option<uuid::Uuid> = workspace_id.and_then(|w| w.parse().ok());

        let result = if remaining_sources.is_empty() {
            // Fully removed: delete the row
            sqlx::query("DELETE FROM entities WHERE name = $1 AND (workspace_id = $2 OR ($2 IS NULL AND workspace_id IS NULL))")
                .bind(name)
                .bind(workspace_uuid)
                .execute(self.pool.as_ref())
                .await
        } else {
            // Partially updated: keep entity but shrink source list
            sqlx::query("UPDATE entities SET source_chunk_ids = $1, sync_status = 'synced', updated_at = NOW() WHERE name = $2 AND (workspace_id = $3 OR ($3 IS NULL AND workspace_id IS NULL))")
                .bind(remaining_sources)
                .bind(name)
                .bind(workspace_uuid)
                .execute(self.pool.as_ref())
                .await
        };

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!(entity = %name, error = %e, "Relational entity delete/update failed (best-effort)");
                Ok(())
            }
        }
    }
}
