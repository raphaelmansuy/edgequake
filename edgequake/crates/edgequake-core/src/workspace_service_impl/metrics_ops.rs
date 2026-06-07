#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::{
    error::{Error, Result},
    types::{MetricsSnapshot, MetricsTriggerType},
};

#[cfg(feature = "postgres")]
use super::WorkspaceServiceImpl;

#[cfg(feature = "postgres")]
impl WorkspaceServiceImpl {
    // ============ Metrics Operations ============

    pub(super) async fn pg_record_metrics_snapshot(
        &self,
        workspace_id: Uuid,
        trigger_type: MetricsTriggerType,
    ) -> Result<MetricsSnapshot> {
        // First get current stats
        let stats = self.pg_get_workspace_stats(workspace_id).await?;

        // WHY INSERT ... RETURNING: Single round-trip, atomic operation.
        // OODA-20: Records to workspace_metrics_history from migration 016.
        #[derive(sqlx::FromRow)]
        struct SnapshotRow {
            id: Uuid,
            #[allow(dead_code)]
            workspace_id: String,
            recorded_at: chrono::DateTime<chrono::Utc>,
            trigger_type: String,
            document_count: i64,
            chunk_count: i64,
            entity_count: i64,
            relationship_count: i64,
            embedding_count: i64,
            storage_bytes: i64,
        }

        let row: SnapshotRow = sqlx::query_as(
            r#"
            INSERT INTO workspace_metrics_history (
                workspace_id, trigger_type,
                document_count, chunk_count, entity_count, relationship_count,
                embedding_count, storage_bytes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, workspace_id, recorded_at, trigger_type,
                      document_count, chunk_count, entity_count, relationship_count,
                      embedding_count, storage_bytes
            "#,
        )
        .bind(workspace_id.to_string())
        .bind(trigger_type.as_str())
        .bind(stats.document_count as i64)
        .bind(stats.chunk_count as i64)
        .bind(stats.entity_count as i64)
        .bind(stats.relationship_count as i64)
        .bind(stats.embedding_count as i64)
        .bind(stats.storage_bytes as i64)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to record metrics snapshot: {}", e)))?;

        Ok(MetricsSnapshot {
            id: row.id,
            workspace_id,
            recorded_at: row.recorded_at,
            trigger_type: MetricsTriggerType::parse(&row.trigger_type)
                .unwrap_or(MetricsTriggerType::Event),
            document_count: row.document_count as usize,
            chunk_count: row.chunk_count as usize,
            entity_count: row.entity_count as usize,
            relationship_count: row.relationship_count as usize,
            embedding_count: row.embedding_count as usize,
            storage_bytes: row.storage_bytes as usize,
        })
    }

    pub(super) async fn pg_get_metrics_history(
        &self,
        workspace_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<MetricsSnapshot>> {
        // WHY ORDER BY DESC: Most recent snapshots first for trend analysis.
        // OODA-22: Query from workspace_metrics_history table.
        #[derive(sqlx::FromRow)]
        struct HistoryRow {
            id: Uuid,
            #[allow(dead_code)]
            workspace_id: String,
            recorded_at: chrono::DateTime<chrono::Utc>,
            trigger_type: String,
            document_count: i64,
            chunk_count: i64,
            entity_count: i64,
            relationship_count: i64,
            embedding_count: i64,
            storage_bytes: i64,
        }

        let rows: Vec<HistoryRow> = sqlx::query_as(
            r#"
            SELECT id, workspace_id, recorded_at, trigger_type,
                   document_count, chunk_count, entity_count, relationship_count,
                   embedding_count, storage_bytes
            FROM workspace_metrics_history
            WHERE workspace_id = $1
            ORDER BY recorded_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(workspace_id.to_string())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::internal(format!("Failed to get metrics history: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|row| MetricsSnapshot {
                id: row.id,
                workspace_id,
                recorded_at: row.recorded_at,
                trigger_type: MetricsTriggerType::parse(&row.trigger_type)
                    .unwrap_or(MetricsTriggerType::Event),
                document_count: row.document_count as usize,
                chunk_count: row.chunk_count as usize,
                entity_count: row.entity_count as usize,
                relationship_count: row.relationship_count as usize,
                embedding_count: row.embedding_count as usize,
                storage_bytes: row.storage_bytes as usize,
            })
            .collect())
    }
}
