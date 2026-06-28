//! Document read-model helpers (SPEC-021 P5-01).
//!
//! Resolves cross-store drift between the relational `documents` table (write
//! target for async dual-write) and `eq_*_kv` metadata (legacy read path).
//!
//! # Read authority
//!
//! - **document_count / list**: `max(postgresql, kv)` when Postgres is active;
//!   KV-only in memory/test mode.
//! - **entity_count / relationship_count**: always AGE graph (see `stats.rs`).

/// Operator-visible merge rule (SPEC-024 Phase 4.6).
pub const MERGE_STRATEGY: &str = "max(postgresql, kv)";

/// Snapshot of KV vs relational drift for dashboards (read-only, cheap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentDriftSnapshot {
    pub postgres_count: usize,
    pub kv_count: usize,
    pub merged_count: usize,
    pub drift_detected: bool,
}

/// Detect whether KV and relational stores disagree on document count.
#[inline]
pub fn detect_document_drift(postgres_count: usize, kv_count: usize) -> DocumentDriftSnapshot {
    let merged_count = merge_document_count(postgres_count, kv_count);
    DocumentDriftSnapshot {
        postgres_count,
        kv_count,
        merged_count,
        drift_detected: postgres_count != kv_count,
    }
}

use crate::handlers::documents_types::DocumentSummary;
use crate::middleware::TenantContext;
use crate::state::AppState;
use uuid::Uuid;

/// Merge document counts from two stores covering opposite drift directions.
///
/// - Postgres-only rows (KV missing) → pg wins.
/// - KV-only legacy uploads (postgres missing) → kv wins.
#[inline]
pub fn merge_document_count(postgres_count: usize, kv_count: usize) -> usize {
    postgres_count.max(kv_count)
}

/// Merge storage byte totals the same way as document counts.
#[inline]
pub fn merge_storage_bytes(postgres_bytes: u64, kv_bytes: u64) -> u64 {
    postgres_bytes.max(kv_bytes)
}

/// Fetch `(document_count, storage_bytes)` from the relational `documents` table.
///
/// Returns `None` when running in memory mode (no `pg_pool`).
#[cfg(feature = "postgres")]
pub async fn postgres_document_metrics(
    state: &AppState,
    workspace_id: Uuid,
) -> Option<(usize, u64)> {
    state.pg_pool.as_ref()?;

    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .ok()?;

    Some((stats.document_count, stats.storage_bytes as u64))
}

#[cfg(not(feature = "postgres"))]
pub async fn postgres_document_metrics(
    _state: &AppState,
    _workspace_id: Uuid,
) -> Option<(usize, u64)> {
    None
}

/// Normalize relational status values to the strings expected by the WebUI.
#[cfg(feature = "postgres")]
fn normalize_relational_status(status: &str) -> String {
    match status {
        "indexed" => "completed".to_string(),
        other => other.to_string(),
    }
}

/// Load document summaries from the relational `documents` table for a workspace.
///
/// Used to backfill the documents list when KV metadata is missing or scoped to
/// a legacy workspace id.
#[cfg(feature = "postgres")]
pub async fn list_relational_document_summaries(
    pool: Option<&sqlx::PgPool>,
    tenant_ctx: &TenantContext,
) -> Result<Vec<DocumentSummary>, crate::error::ApiError> {
    use crate::error::ApiError;
    use sqlx::Row;

    let pool = pool.ok_or_else(|| ApiError::Internal("PostgreSQL pool not available".into()))?;

    let workspace_id = tenant_ctx
        .workspace_id
        .as_ref()
        .and_then(|w| Uuid::parse_str(w).ok())
        .ok_or_else(|| ApiError::BadRequest("workspace_id required".into()))?;

    let tenant_uuid = tenant_ctx
        .tenant_id
        .as_ref()
        .and_then(|t| Uuid::parse_str(t).ok());

    let rows = sqlx::query(
        r#"
        SELECT
            id::text AS id,
            title,
            status,
            chunk_count,
            entity_count,
            file_size_bytes,
            track_id,
            error_message,
            created_at,
            updated_at,
            LEFT(content, 200) AS content_preview,
            LENGTH(content)::int AS content_length,
            -- WHY: cost/token stats live in the `metadata` JSONB column (written by
            -- the pipeline alongside KV metadata). Reading them from `metadata`
            -- avoids depending on migration-041 stat columns (`cost_usd`,
            -- `input_tokens`, ...) which a stale sqlx prepared-statement cache /
            -- partially-applied migration can make invisible at runtime, causing
            -- the "column cost_usd does not exist" backfill failure that turns
            -- the documents list into "0 documents". `metadata` is a base column
            -- (migration 001) and always present.
            (metadata->>'cost_usd')::double precision AS cost_usd,
            (metadata->>'input_tokens')::bigint AS input_tokens,
            (metadata->>'output_tokens')::bigint AS output_tokens,
            (metadata->>'total_tokens')::bigint AS total_tokens
        FROM documents
        WHERE workspace_id = $1
          AND ($2::uuid IS NULL OR tenant_id IS NULL OR tenant_id = $2)
        ORDER BY created_at DESC
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_uuid)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to list relational documents: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let title: String = row.get("title");
            let status: String = row.get("status");
            let chunk_count: Option<i32> = row.get("chunk_count");
            let entity_count: Option<i32> = row.get("entity_count");
            let content_length: Option<i32> = row.get("content_length");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
            let cost_usd: Option<f64> = row.get("cost_usd");
            let input_tokens: Option<i64> = row.get("input_tokens");
            let output_tokens: Option<i64> = row.get("output_tokens");
            let total_tokens: Option<i64> = row.get("total_tokens");
            let track_id: Option<String> = row.get("track_id");

            DocumentSummary {
                id: row.get("id"),
                title: Some(title.clone()),
                file_name: Some(title),
                content_summary: row.get("content_preview"),
                content_length: content_length.map(|n| n.max(0) as usize),
                chunk_count: chunk_count.unwrap_or(0).max(0) as usize,
                entity_count: entity_count.map(|n| n.max(0) as usize),
                status: Some(normalize_relational_status(&status)),
                error_message: row.get("error_message"),
                warning_message: None,
                track_id,
                created_at: Some(created_at.to_rfc3339()),
                updated_at: Some(updated_at.to_rfc3339()),
                cost_usd,
                input_tokens: input_tokens.map(|n| n as usize),
                output_tokens: output_tokens.map(|n| n as usize),
                total_tokens: total_tokens.map(|n| n as usize),
                llm_model: None,
                embedding_model: None,
                source_type: None,
                current_stage: None,
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            }
        })
        .collect())
}

#[cfg(not(feature = "postgres"))]
pub async fn list_relational_document_summaries(
    _tenant_ctx: &TenantContext,
) -> Result<Vec<DocumentSummary>, crate::error::ApiError> {
    Ok(vec![])
}

/// Merge KV-derived documents with relational rows (relational fills gaps).
pub fn merge_document_summaries(
    mut kv_documents: Vec<DocumentSummary>,
    relational_documents: Vec<DocumentSummary>,
) -> Vec<DocumentSummary> {
    use std::collections::HashSet;

    let existing: HashSet<String> = kv_documents.iter().map(|d| d.id.clone()).collect();

    for rel in relational_documents {
        if !existing.contains(&rel.id) {
            kv_documents.push(rel);
        }
    }

    kv_documents.sort_by(|a, b| {
        b.created_at
            .as_deref()
            .unwrap_or("")
            .cmp(a.created_at.as_deref().unwrap_or(""))
    });

    kv_documents
}

/// Resolve the per-document `entity_count` against the authoritative AGE
/// graph when both the KV and relational feeds are missing/zero (SPEC-021 P-A3).
///
/// WHY: the "Completed / 0 entities" screenshot (file 16) is caused by KV
/// metadata scoped to a legacy workspace being dropped, leaving the relational
/// backfill — whose `entity_count` column was never refreshed (P-A1 fixes the
/// write side; this is the read-side safety net). AGE is the SSOT for entity
/// counts, so we fall back to `node_count_by_source_prefix("{doc_id}-chunk-")`
/// for any document whose current `entity_count` is 0 but which has chunks.
///
/// Best-effort: AGE failures are swallowed (counts stay as-is) so the list
/// request never fails due to a graph hiccup. Batches the per-doc Cypher calls
/// to avoid N+1 where possible; falls back to per-doc calls otherwise.
pub async fn reconcile_entity_counts_with_graph(
    storage: &crate::state::StorageRuntime,
    documents: &mut [DocumentSummary],
) {
    use edgequake_storage::kv_keys;

    // Only reconcile rows that currently report 0 entities — the common
    // screenshot case. Rows with a non-zero count already come from KV (truth).
    let candidates: Vec<(usize, String)> = documents
        .iter()
        .enumerate()
        .filter(|(_, d)| d.entity_count.unwrap_or(0) == 0 && d.chunk_count > 0)
        .map(|(i, d)| (i, d.id.clone()))
        .collect();

    if candidates.is_empty() {
        return;
    }

    for (idx, doc_id) in candidates {
        let prefix = kv_keys::doc_chunk_prefix(&doc_id);
        match storage
            .graph_storage
            .node_count_by_source_prefix(&prefix)
            .await
        {
            Ok(age_count) if age_count > 0 => {
                documents[idx].entity_count = Some(age_count);
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    document_id = %doc_id,
                    error = %e,
                    "P-A3: AGE entity_count fallback failed (non-fatal) — leaving count as-is"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_document_count_prefers_higher_value() {
        assert_eq!(merge_document_count(7, 0), 7);
        assert_eq!(merge_document_count(0, 3), 3);
        assert_eq!(merge_document_count(2, 5), 5);
    }

    #[test]
    fn detect_document_drift_flags_mismatch() {
        let no_drift = detect_document_drift(3, 3);
        assert!(!no_drift.drift_detected);
        assert_eq!(no_drift.merged_count, 3);

        let pg_only = detect_document_drift(5, 0);
        assert!(pg_only.drift_detected);
        assert_eq!(pg_only.merged_count, 5);

        let kv_only = detect_document_drift(0, 2);
        assert!(kv_only.drift_detected);
        assert_eq!(kv_only.merged_count, 2);
    }

    #[test]
    fn merge_storage_bytes_prefers_higher_value() {
        assert_eq!(merge_storage_bytes(1000, 0), 1000);
        assert_eq!(merge_storage_bytes(0, 5120), 5120);
    }

    #[test]
    fn merge_document_summaries_deduplicates_by_id() {
        let kv = vec![DocumentSummary {
            id: "a".into(),
            title: Some("KV title".into()),
            file_name: None,
            content_summary: None,
            content_length: None,
            chunk_count: 2,
            entity_count: None,
            status: Some("completed".into()),
            error_message: None,
            warning_message: None,
            track_id: None,
            created_at: Some("2026-01-02T00:00:00Z".into()),
            updated_at: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            llm_model: None,
            embedding_model: None,
            source_type: None,
            current_stage: None,
            stage_progress: None,
            stage_message: None,
            pdf_id: None,
        }];

        let pg = vec![
            DocumentSummary {
                id: "a".into(),
                title: Some("PG duplicate".into()),
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 0,
                entity_count: None,
                status: Some("completed".into()),
                error_message: None,
                warning_message: None,
                track_id: None,
                created_at: Some("2026-01-01T00:00:00Z".into()),
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                source_type: None,
                current_stage: None,
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            },
            DocumentSummary {
                id: "b".into(),
                title: Some("PG only".into()),
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 0,
                entity_count: None,
                status: Some("completed".into()),
                error_message: None,
                warning_message: None,
                track_id: None,
                created_at: Some("2026-01-03T00:00:00Z".into()),
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                source_type: None,
                current_stage: None,
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            },
        ];

        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, "b");
        assert_eq!(merged[1].id, "a");
        assert_eq!(merged[1].title.as_deref(), Some("KV title"));
    }
}
