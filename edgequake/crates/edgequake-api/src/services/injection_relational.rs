//! SPEC-091 Wave B6 — injection metadata on `public.documents`
//! (`metadata->>'source_type' = 'injection'`).
//!
//! The legacy `injection::{ws}::{id}-metadata` KV record holds the whole
//! injection document; the typed cutover stores the same JSON in
//! `documents.metadata` (SSOT shape — readers keep working unchanged) with
//! title/content/status promoted to columns. The row id IS the injection id
//! (already a UUIDv4 at creation).
//!
//! Cutover pattern mirrors B1/B4/B5: writes dual (KV authoritative + typed
//! warn-only), reads flag-gated typed-first (`EDGEQUAKE_KV_FAMILY_INJECTION=
//! relational`) with KV fallback on any gap.

use serde_json::Value;

use edgequake_storage::kv_family_cutover::{
    kv_family_mode_from_env, KvFamilyMode, KV_FAMILY_INJECTION,
};

pub fn injections_prefer_relational() -> bool {
    kv_family_mode_from_env(KV_FAMILY_INJECTION) == KvFamilyMode::Relational
}

/// Injection status → documents.status (CHECK: pending/processing/indexed/failed).
fn doc_status(injection_status: &str) -> &'static str {
    match injection_status {
        "completed" | "indexed" => "indexed",
        "failed" | "cancelled" => "failed",
        "pending" => "pending",
        _ => "processing",
    }
}

/// Typed upsert from the canonical metadata JSON (warn-only).
/// Skips rows with non-UUID injection/workspace ids — KV stays authoritative.
pub async fn typed_injection_upsert(meta: &Value, pool: crate::services::OptionalPgPool<'_>) {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = pool else {
            return;
        };
        let (Some(id), Some(ws)) = (
            meta.get("id")
                .and_then(|v| v.as_str())
                .and_then(|s| uuid::Uuid::parse_str(s).ok()),
            meta.get("workspace_id")
                .and_then(|v| v.as_str())
                .and_then(|s| uuid::Uuid::parse_str(s).ok()),
        ) else {
            return;
        };
        let title = meta.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let content = meta.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let status = doc_status(meta.get("status").and_then(|v| v.as_str()).unwrap_or(""));
        // Marker lets list/read discriminate injection rows from documents.
        let mut stored = meta.clone();
        stored["source_type"] = Value::String("injection".to_string());

        let result = sqlx::query(
            r#"
            INSERT INTO public.documents (id, workspace_id, title, content, status, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                workspace_id = EXCLUDED.workspace_id,
                title = EXCLUDED.title,
                content = EXCLUDED.content,
                status = EXCLUDED.status,
                metadata = EXCLUDED.metadata,
                updated_at = now()
            "#,
        )
        .bind(id)
        .bind(ws)
        .bind(title)
        .bind(content)
        .bind(status)
        .bind(stored)
        .execute(pool)
        .await;
        if let Err(e) = result {
            if injections_prefer_relational() {
                tracing::error!(injection_id = %id, error = %e, "SPEC-091: authoritative typed injection upsert FAILED");
            } else {
                tracing::warn!(injection_id = %id, error = %e, "typed injection upsert failed (KV remains)");
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (meta, pool);
    }
}

/// Typed read of one injection's metadata JSON (None → KV fallback).
pub async fn typed_injection_get(
    injection_id: &str,
    pool: crate::services::OptionalPgPool<'_>,
) -> Option<Value> {
    #[cfg(feature = "postgres")]
    {
        let pool = pool?;
        let id = uuid::Uuid::parse_str(injection_id).ok()?;
        match sqlx::query_scalar::<_, Value>(
            "SELECT metadata FROM public.documents \
             WHERE id = $1 AND metadata->>'source_type' = 'injection'",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(injection_id, error = %e, "typed injection read failed");
                None
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (injection_id, pool);
        None
    }
}

/// Typed list for a workspace: (metadata JSONs, total). None → KV fallback.
pub async fn typed_injection_list(
    workspace_id: &str,
    pool: crate::services::OptionalPgPool<'_>,
    limit: i64,
    offset: i64,
) -> Option<(Vec<Value>, i64)> {
    #[cfg(feature = "postgres")]
    {
        let pool = pool?;
        let ws = uuid::Uuid::parse_str(workspace_id).ok()?;
        let rows = sqlx::query_scalar::<_, Value>(
            "SELECT metadata FROM public.documents \
             WHERE workspace_id = $1 AND metadata->>'source_type' = 'injection' \
             ORDER BY created_at DESC, id LIMIT $2 OFFSET $3",
        )
        .bind(ws)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await;
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM public.documents \
             WHERE workspace_id = $1 AND metadata->>'source_type' = 'injection'",
        )
        .bind(ws)
        .fetch_one(pool)
        .await;
        match (rows, total) {
            (Ok(items), Ok(total)) => Some((items, total)),
            (Err(e), _) | (_, Err(e)) => {
                tracing::warn!(workspace_id, error = %e, "typed injection list failed");
                None
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (workspace_id, pool, limit, offset);
        None
    }
}

/// Typed delete (paired with the caller's KV sweep).
pub async fn typed_injection_delete(injection_id: &str, pool: crate::services::OptionalPgPool<'_>) {
    #[cfg(feature = "postgres")]
    {
        let (Some(pool), Ok(id)) = (pool, uuid::Uuid::parse_str(injection_id)) else {
            return;
        };
        if let Err(e) = sqlx::query(
            "DELETE FROM public.documents WHERE id = $1 AND metadata->>'source_type' = 'injection'",
        )
        .bind(id)
        .execute(pool)
        .await
        {
            if injections_prefer_relational() {
                tracing::error!(injection_id = %id, error = %e, "SPEC-091: authoritative typed injection delete FAILED");
            } else {
                tracing::warn!(injection_id = %id, error = %e, "typed injection delete failed");
            }
        }
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (injection_id, pool);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_status_maps_to_documents_check_constraint() {
        assert_eq!(doc_status("completed"), "indexed");
        assert_eq!(doc_status("failed"), "failed");
        assert_eq!(doc_status("pending"), "pending");
        assert_eq!(doc_status("processing"), "processing");
        assert_eq!(doc_status("anything-else"), "processing");
    }

    /// SPEC-091 Wave D: the flag defaults to RELATIONAL; typed accessors stay
    /// inert without a pool, and the `kv` rollback env keeps working.
    #[tokio::test]
    async fn typed_accessors_inert_without_pool() {
        std::env::remove_var("EDGEQUAKE_KV_FAMILY_INJECTION");
        assert!(injections_prefer_relational());
        typed_injection_upsert(&serde_json::json!({"id": "x", "workspace_id": "y"}), None).await;
        assert!(typed_injection_get("not-a-uuid", None).await.is_none());
        assert!(typed_injection_list("ws", None, 10, 0).await.is_none());
        typed_injection_delete("not-a-uuid", None).await;

        std::env::set_var("EDGEQUAKE_KV_FAMILY_INJECTION", "kv");
        assert!(!injections_prefer_relational());
        std::env::remove_var("EDGEQUAKE_KV_FAMILY_INJECTION");
    }
}
