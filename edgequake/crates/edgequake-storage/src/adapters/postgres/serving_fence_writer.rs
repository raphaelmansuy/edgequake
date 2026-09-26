//! Serving-fence open policy (SPEC-091): settle check + ready upsert.
//!
//! Separated from `ChunkRepository` so the projection worker can depend on a
//! port (`ServingFenceOpener`) instead of a raw `PgPool`.

use async_trait::async_trait;
use edgequake_storage_contracts::{AccessError, AccessResult};
use sqlx::PgPool;
use uuid::Uuid;

use super::serving_state_sql::{upsert_for_document, OPEN_SETTLED_FENCES_BOUNDED_SQL};
use crate::error::StorageError;
use crate::projection::serving_fence_port::ServingFenceOpener;

const DOCUMENT_BATCH_DELIVERIES_SETTLED_SQL: &str = r#"
SELECT EXISTS (
    SELECT 1
    FROM public.projection_events e
    WHERE e.object_id = $1 AND e.object_kind = 'document_batch'
)
AND NOT EXISTS (
    SELECT 1
    FROM public.projection_events e
    JOIN public.projection_deliveries d ON d.event_id = e.event_id
    WHERE e.object_id = $1
      AND e.object_kind = 'document_batch'
      AND d.state IN ('pending', 'leased', 'retry', 'quarantined')
)
"#;

const ACKED_DOCUMENT_BATCH_IDS_SQL: &str = r#"
SELECT DISTINCT e.object_id
FROM public.projection_events e
WHERE e.event_id = ANY($1::uuid[]) AND e.object_kind = 'document_batch'
ORDER BY e.object_id
"#;

/// Namespace for per-document fence locks (first key of the two-key advisory lock).
const SERVING_FENCE_LOCK_NAMESPACE: i32 = 149_091;

const LOCK_DOCUMENT_FENCE_SQL: &str = "SELECT pg_advisory_xact_lock($1, hashtext($2::text))";

/// Open the fence for every `document_batch` touched by `acked_event_ids`, inside
/// the caller's ack transaction. Returns chunk rows changed.
///
/// Two acks for the last graph and vector deliveries of one document can commit
/// concurrently; each would see the other's row as still leased and neither
/// would open. The per-document advisory lock is taken after the ack UPDATE, so
/// the second waiter's settle check (a fresh READ COMMITTED snapshot) sees the
/// first commit. Documents are locked in sorted order to rule out lock cycles.
pub(crate) async fn open_fences_for_acked_events(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    acked_event_ids: &[Uuid],
) -> Result<u64, StorageError> {
    if acked_event_ids.is_empty() {
        return Ok(0);
    }
    let document_ids: Vec<Uuid> = sqlx::query_scalar(ACKED_DOCUMENT_BATCH_IDS_SQL)
        .bind(acked_event_ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("acked document lookup failed: {e}")))?;
    let mut touched = 0u64;
    for document_id in document_ids {
        sqlx::query(LOCK_DOCUMENT_FENCE_SQL)
            .bind(SERVING_FENCE_LOCK_NAMESPACE)
            .bind(document_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| StorageError::Database(format!("serving fence lock failed: {e}")))?;
        if !document_batch_deliveries_settled(&mut **tx, document_id).await? {
            continue;
        }
        touched += upsert_for_document(
            &mut **tx,
            document_id,
            crate::serving_fence::SERVING_STATE_READY,
        )
        .await?;
    }
    Ok(touched)
}

/// Open the SPEC-091 serving fence once every `document_batch` delivery is applied.
///
/// WHY: Durable `commit_batch` returns before vectors/graph apply, so the sync
/// persister never reaches `set_serving_state(ready)`. Without this, list shows
/// Indexed · not queryable and retrieval hides every chunk.
///
/// Returns `Ok(true)` only when at least one `chunk_serving_state` row changed.
pub async fn open_serving_fence_when_deliveries_settled(
    pool: &PgPool,
    document_id: Uuid,
) -> Result<bool, StorageError> {
    if !document_batch_deliveries_settled(pool, document_id).await? {
        return Ok(false);
    }

    let touched =
        upsert_for_document(pool, document_id, crate::serving_fence::SERVING_STATE_READY).await?;

    if touched == 0 {
        return Ok(false);
    }
    crate::adapters::postgres::serving_fence_query::record_serving_fence_opened(touched);
    tracing::info!(
        document_id = %document_id,
        chunks_touched = touched,
        "SPEC-091: opened serving fence after document_batch deliveries settled"
    );
    Ok(true)
}

/// True when this document has ≥1 `document_batch` event and zero open deliveries.
///
/// Single statement — the SSOT for “projection settled” (worker, promote, heal).
pub async fn document_batch_deliveries_settled<'e, E>(
    executor: E,
    document_id: Uuid,
) -> Result<bool, StorageError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let settled: bool = sqlx::query_scalar(DOCUMENT_BATCH_DELIVERIES_SETTLED_SQL)
        .bind(document_id)
        .fetch_one(executor)
        .await
        .map_err(|e| StorageError::Database(format!("serving fence settled check failed: {e}")))?;
    Ok(settled)
}

/// One bounded statement: open the fence for settled docs that still have non-ready chunks.
///
/// Used by SPEC-054 reconcile only — list and promote must not write serving state.
/// Returns the number of `chunk_serving_state` rows touched.
pub async fn open_settled_serving_fences_bounded(
    pool: &PgPool,
    limit: i64,
) -> Result<u64, StorageError> {
    let limit = limit.max(0);
    if limit == 0 {
        return Ok(0);
    }
    let result = sqlx::query(OPEN_SETTLED_FENCES_BOUNDED_SQL)
        .bind(crate::serving_fence::SERVING_STATE_READY)
        .bind(limit)
        .execute(pool)
        .await
        .map_err(|e| StorageError::Database(format!("bounded serving fence open failed: {e}")))?;
    let touched = result.rows_affected();
    crate::adapters::postgres::serving_fence_query::record_serving_fence_opened(touched);
    if touched > 0 {
        tracing::info!(
            chunks_touched = touched,
            limit,
            "SPEC-091: bounded reconcile opened serving fences for settled document_batch docs"
        );
    }
    Ok(touched)
}

/// Pure helper for tests: `rows_affected > 0` means the opener reported a write.
pub fn serving_fence_open_changed(rows_affected: u64) -> bool {
    rows_affected > 0
}

/// Postgres adapter for [`ServingFenceOpener`] — worker depends on the trait.
pub struct PgServingFenceOpener {
    pool: PgPool,
}

impl PgServingFenceOpener {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServingFenceOpener for PgServingFenceOpener {
    async fn open_when_settled(&self, document_id: Uuid) -> AccessResult<bool> {
        open_serving_fence_when_deliveries_settled(&self.pool, document_id)
            .await
            .map_err(AccessError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settled_predicate_is_one_statement() {
        assert!(DOCUMENT_BATCH_DELIVERIES_SETTLED_SQL.contains("document_batch"));
        assert!(DOCUMENT_BATCH_DELIVERIES_SETTLED_SQL.contains("EXISTS"));
        assert!(DOCUMENT_BATCH_DELIVERIES_SETTLED_SQL.contains("NOT EXISTS"));
        assert!(
            !DOCUMENT_BATCH_DELIVERIES_SETTLED_SQL.contains("COUNT(*)"),
            "settled check must be one EXISTS/NOT EXISTS statement, not two COUNTs"
        );
    }

    #[test]
    fn ack_fence_open_locks_documents_in_sorted_order() {
        assert!(ACKED_DOCUMENT_BATCH_IDS_SQL.contains("ORDER BY e.object_id"));
        assert!(ACKED_DOCUMENT_BATCH_IDS_SQL.contains("document_batch"));
        assert!(LOCK_DOCUMENT_FENCE_SQL.contains("pg_advisory_xact_lock"));
    }

    #[test]
    fn bounded_fence_open_sql_has_limit() {
        assert!(OPEN_SETTLED_FENCES_BOUNDED_SQL.contains("LIMIT $2"));
        assert!(OPEN_SETTLED_FENCES_BOUNDED_SQL.contains("document_batch"));
        assert!(OPEN_SETTLED_FENCES_BOUNDED_SQL.contains("chunk_serving_state"));
    }

    #[test]
    fn serving_fence_open_changed_only_when_rows_written() {
        assert!(!serving_fence_open_changed(0));
        assert!(serving_fence_open_changed(1));
        assert!(serving_fence_open_changed(19));
    }
}
