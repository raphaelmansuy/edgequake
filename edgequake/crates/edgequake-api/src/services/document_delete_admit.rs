//! SPEC-098 LAW-098-9: dual-write lifecycle admit for document deletion.
//!
//! After a durable deletion (or batch deletion) job is enqueued, mark each
//! document `deleting` in **both** KV metadata and `public.documents.status`
//! so `GET /documents` list merge cannot resurface stale Completed/Ready.

use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::services::document_metadata_scan::metadata_key_for_document;
use crate::state::AppState;

const DELETING_STAGE_MESSAGE: &str = "Removing document data…";

/// Patch one metadata JSON object to lifecycle `deleting`.
fn patch_metadata_deleting(obj: &mut serde_json::Map<String, serde_json::Value>) {
    obj.insert("status".to_string(), serde_json::json!("deleting"));
    obj.insert("current_stage".to_string(), serde_json::json!("deleting"));
    obj.insert(
        "stage_message".to_string(),
        serde_json::json!(DELETING_STAGE_MESSAGE),
    );
    obj.insert("stage_progress".to_string(), serde_json::json!(0.0));
    for key in [
        "entity_count",
        "entities_count",
        "relationship_count",
        "relationships_count",
        "total_cost",
        "cost_usd",
    ] {
        if obj.contains_key(key) {
            obj.insert(key.to_string(), serde_json::json!(0));
        }
    }
}

/// Best-effort SQL status touch that **logs loudly** on failure (admit path).
///
/// Non-UUID ids cannot live in `documents.id` (uuid PK) — skip without error.
async fn touch_sql_deleting(document_id: &str) -> ApiResult<()> {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = crate::services::relational_sidecar_store::sidecar_pool() else {
            tracing::debug!(
                document_id = %document_id,
                "admit_documents_deleting: no sidecar pool — SQL status skipped"
            );
            return Ok(());
        };
        let Ok(doc_uuid) = Uuid::parse_str(document_id) else {
            tracing::debug!(
                document_id = %document_id,
                "admit_documents_deleting: non-UUID id — SQL status skipped"
            );
            return Ok(());
        };
        // SPEC-098 LAW-098-11 / W12: after durable enqueue, SQL CHECK miss must
        // not turn 202 into 500. KV deleting remains the hard admit path.
        match sqlx::query(
            "UPDATE public.documents SET status = 'deleting', updated_at = NOW() WHERE id = $1",
        )
        .bind(doc_uuid)
        .execute(pool.as_ref())
        .await
        {
            Ok(result) => {
                if result.rows_affected() == 0 {
                    tracing::debug!(
                        document_id = %document_id,
                        "admit_documents_deleting: no SQL row (KV-only / staging shell)"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    target: "edgequake.delete",
                    document_id = %document_id,
                    error = %e,
                    event = "spec098_sql_deleting_mirror_failed",
                    "SPEC-098: SQL status=deleting mirror failed (KV still deleting) — apply support/141 if CHECK rejects lifecycle statuses"
                );
            }
        }
        Ok(())
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = document_id;
        Ok(())
    }
}

/// Dual-write `deleting` for one document (KV metadata + SQL).
///
/// `key_prefix` is the KV identity prefix (may differ from `document_id` for
/// legacy key mismatches). Missing KV metadata is non-fatal (orphan SQL purge
/// paths still need SQL `deleting` when a row exists).
pub async fn admit_document_deleting(
    state: &AppState,
    document_id: &str,
    key_prefix: &str,
) -> ApiResult<()> {
    let metadata_key = metadata_key_for_document(key_prefix);
    if let Ok(Some(mut metadata)) = state.storage.kv_storage.get_by_id(&metadata_key).await {
        if let Some(obj) = metadata.as_object_mut() {
            patch_metadata_deleting(obj);
            crate::services::upsert_metadata_kv_with_index(
                state.storage.kv_storage.as_ref(),
                &metadata_key,
                metadata,
            )
            .await
            .map_err(|e| {
                ApiError::Internal(format!(
                    "SPEC-098: failed to write KV status=deleting for {document_id}: {e}"
                ))
            })?;
        }
    }

    // Prefer document_id for SQL PK; fall back to key_prefix if id is non-UUID.
    if Uuid::parse_str(document_id).is_ok() {
        touch_sql_deleting(document_id).await?;
    } else if Uuid::parse_str(key_prefix).is_ok() {
        touch_sql_deleting(key_prefix).await?;
    } else {
        let _ = touch_sql_deleting(document_id).await;
    }
    Ok(())
}

/// Dual-write `deleting` for many documents (batch admit).
///
/// Each entry is `(document_id, key_prefix)`. Continues on per-id errors and
/// returns the first error after attempting all (fail-closed summary).
pub async fn admit_documents_deleting(
    state: &AppState,
    docs: &[(String, String)],
) -> ApiResult<()> {
    let mut first_err: Option<ApiError> = None;
    for (document_id, key_prefix) in docs {
        if let Err(e) = admit_document_deleting(state, document_id, key_prefix).await {
            tracing::error!(
                document_id = %document_id,
                error = %e,
                "SPEC-098: admit_documents_deleting failed for one id"
            );
            if first_err.is_none() {
                first_err = Some(e);
            }
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Mirror `delete_failed` to SQL when resetting a stuck delete (LAW-098-9 parity).
pub async fn touch_sql_delete_failed(document_id: &str) {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = crate::services::relational_sidecar_store::sidecar_pool() else {
            return;
        };
        let Ok(doc_uuid) = Uuid::parse_str(document_id) else {
            return;
        };
        if let Err(e) = sqlx::query(
            "UPDATE public.documents SET status = 'delete_failed', updated_at = NOW() WHERE id = $1",
        )
        .bind(doc_uuid)
        .execute(pool.as_ref())
        .await
        {
            tracing::warn!(
                document_id = %document_id,
                error = %e,
                "SPEC-098: failed to mirror documents.status=delete_failed"
            );
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = document_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_metadata_deleting_sets_lifecycle_fields() {
        let mut obj = serde_json::Map::new();
        obj.insert("status".into(), serde_json::json!("completed"));
        obj.insert("entity_count".into(), serde_json::json!(42));
        patch_metadata_deleting(&mut obj);
        assert_eq!(obj.get("status").and_then(|v| v.as_str()), Some("deleting"));
        assert_eq!(
            obj.get("current_stage").and_then(|v| v.as_str()),
            Some("deleting")
        );
        assert_eq!(obj.get("entity_count").and_then(|v| v.as_i64()), Some(0));
    }
}
