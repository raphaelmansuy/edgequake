//! SPEC-091 W2 — ingestion dedup store router (KV `doc:hash:`/`staging:hash:`
//! families → typed `public.ingestion_dedup`).
//!
//! Cutover contract (per-family flag, `EDGEQUAKE_KV_FAMILY_DOC_HASH`):
//! - Writes are dual whenever a PostgreSQL pool is available (typed write is
//!   warn-only — KV stays the rollback authority until the read flag flips).
//! - Reads route on the flag: `kv` (default) → KV keys; `relational` → typed
//!   table, with a KV fallback only when no pool exists (memory/test modes).

use edgequake_storage::kv_family_cutover::{
    kv_family_mode_from_env, KvFamilyMode, KV_FAMILY_DOC_HASH,
};
use edgequake_storage::kv_keys;
use serde_json::json;

use crate::error::ApiResult;
use crate::state::AppState;

#[cfg(feature = "postgres")]
use edgequake_storage::adapters::postgres::ingestion_dedup as dedup_rel;

fn reads_relational() -> bool {
    matches!(
        kv_family_mode_from_env(KV_FAMILY_DOC_HASH),
        KvFamilyMode::Relational
    )
}

/// Durable duplicate lookup (`doc:hash:{ws}:{sha}` ↔ version `v1`).
pub async fn lookup_durable(
    state: &AppState,
    workspace_id: &str,
    content_hash: &str,
) -> ApiResult<Option<String>> {
    #[cfg(feature = "postgres")]
    {
        if reads_relational() {
            if let Some(pool) = state.optional_pg_pool() {
                return dedup_rel::lookup_document(
                    pool,
                    workspace_id,
                    content_hash,
                    dedup_rel::DEDUP_VERSION_DURABLE,
                )
                .await
                .map_err(|e| crate::error::ApiError::Internal(e.to_string()));
            }
        }
    }
    let hash_key = super::ContentHasher::workspace_hash_key(workspace_id, content_hash);
    let value = state.storage.kv_storage.get_by_id(&hash_key).await?;
    Ok(value.and_then(|v| v.as_str().map(str::to_string)))
}

/// In-flight staging duplicate lookup (`staging:hash:{ws}:{sha}`).
pub async fn lookup_staging(
    state: &AppState,
    workspace_id: &str,
    content_hash: &str,
) -> ApiResult<Option<String>> {
    #[cfg(feature = "postgres")]
    {
        if reads_relational() {
            if let Some(pool) = state.optional_pg_pool() {
                return dedup_rel::lookup_document(
                    pool,
                    workspace_id,
                    content_hash,
                    dedup_rel::DEDUP_VERSION_STAGING,
                )
                .await
                .map_err(|e| crate::error::ApiError::Internal(e.to_string()));
            }
        }
    }
    let key = kv_keys::staging_workspace_hash(workspace_id, content_hash);
    let value = state.storage.kv_storage.get_by_id(&key).await?;
    Ok(value.and_then(|v| v.as_str().map(str::to_string)))
}

/// Admission reservation: KV upsert (authoritative until flag flip) + typed
/// staging row (warn-only dual write).
pub async fn reserve_staging(
    state: &AppState,
    workspace_id: &str,
    content_hash: &str,
    document_id: &str,
    tenant_id: Option<&str>,
) -> ApiResult<()> {
    // SPEC-091 Wave D: relational flag → the typed table is authoritative and
    // the adapter write-stop already drops the KV upsert — write typed only
    // and propagate failures.
    #[cfg(feature = "postgres")]
    if reads_relational() {
        if let Some(pool) = state.optional_pg_pool() {
            dedup_rel::upsert_reservation(
                pool,
                workspace_id,
                content_hash,
                dedup_rel::DEDUP_VERSION_STAGING,
                document_id,
                tenant_id,
            )
            .await
            .map_err(|e| {
                crate::error::ApiError::Internal(format!(
                    "dedup staging reserve (authoritative) failed: {e}"
                ))
            })?;
            return Ok(());
        }
    }

    let key = kv_keys::staging_workspace_hash(workspace_id, content_hash);
    state
        .storage
        .kv_storage
        .upsert(&[(key, json!(document_id))])
        .await?;
    #[cfg(feature = "postgres")]
    if let Some(pool) = state.optional_pg_pool() {
        if let Err(e) = dedup_rel::upsert_reservation(
            pool,
            workspace_id,
            content_hash,
            dedup_rel::DEDUP_VERSION_STAGING,
            document_id,
            tenant_id,
        )
        .await
        {
            warn_dedup("staging reserve", e);
        }
    }
    Ok(())
}

/// Post-success promote: typed durable upsert + staging row delete
/// (KV promote stays in `staging_admission::promote_staging_to_final`).
#[cfg(feature = "postgres")]
pub async fn dual_promote(
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
    content_hash: &str,
    document_id: &str,
    tenant_id: Option<&str>,
) {
    if let Some(pool) = pool {
        if let Err(e) =
            dedup_rel::promote_staging(pool, workspace_id, content_hash, document_id, tenant_id)
                .await
        {
            warn_dedup("promote", e);
        }
    }
}

/// Staging release/rollback: drop the typed staging row.
#[cfg(feature = "postgres")]
pub async fn dual_release_staging(
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
    content_hash: &str,
) {
    if let Some(pool) = pool {
        if let Err(e) = dedup_rel::delete_reservation(
            pool,
            workspace_id,
            content_hash,
            dedup_rel::DEDUP_VERSION_STAGING,
        )
        .await
        {
            warn_dedup("staging release", e);
        }
    }
}

/// Recycle / delete parity: drop every typed reservation for the hash.
#[cfg(feature = "postgres")]
pub async fn dual_delete_all(
    pool: crate::services::OptionalPgPool<'_>,
    workspace_id: &str,
    content_hash: &str,
) {
    if let Some(pool) = pool {
        if let Err(e) = dedup_rel::delete_all_versions(pool, workspace_id, content_hash).await {
            warn_dedup("delete-all", e);
        }
    }
}

#[cfg(feature = "postgres")]
fn warn_dedup(op: &str, e: edgequake_storage::StorageError) {
    if reads_relational() {
        // Wave D write-stop: the typed table is authoritative — escalate.
        tracing::error!(
            error = %e,
            op,
            "SPEC-091: authoritative typed ingestion_dedup write FAILED — \
             dedup data is not persisted; investigate or roll the family flag back to kv"
        );
    } else {
        tracing::warn!(
            error = %e,
            op,
            "SPEC-091: typed ingestion_dedup dual-write failed (KV authoritative)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Memory mode (no PG pool): the router must read/write KV exactly like
    /// the legacy path, whatever the family flag says.
    #[tokio::test]
    async fn contract_spec091_dedup_router_memory_fallback() {
        let state = crate::state::AppState::test_state();
        let ws = "ws-test";
        let hash = "deadbeef";
        let doc = "doc-1";

        assert!(lookup_durable(&state, ws, hash).await.unwrap().is_none());
        assert!(lookup_staging(&state, ws, hash).await.unwrap().is_none());

        reserve_staging(&state, ws, hash, doc, None).await.unwrap();
        assert_eq!(
            lookup_staging(&state, ws, hash).await.unwrap().as_deref(),
            Some(doc)
        );
        // Staging reservation does not leak into the durable lookup.
        assert!(lookup_durable(&state, ws, hash).await.unwrap().is_none());
    }

    /// Durable lookup reads the legacy `doc:hash:` KV key when the family
    /// flag is unset (default `kv`).
    #[tokio::test]
    async fn contract_spec091_dedup_router_kv_durable_read() {
        let state = crate::state::AppState::test_state();
        let ws = "ws-durable";
        let hash = "cafe";
        let key = super::super::ContentHasher::workspace_hash_key(ws, hash);
        state
            .storage
            .kv_storage
            .upsert(&[(key, serde_json::json!("doc-9"))])
            .await
            .unwrap();
        assert_eq!(
            lookup_durable(&state, ws, hash).await.unwrap().as_deref(),
            Some("doc-9")
        );
    }
}
