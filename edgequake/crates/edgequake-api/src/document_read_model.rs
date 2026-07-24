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

/// Relational workspace metrics overlay (SPEC-021 / SPEC-087).
///
/// `(document_count, storage_bytes, chunk_count, embedding_count)` from
/// `WorkspaceService::get_workspace_stats` (Postgres `documents` / `chunks`).
///
/// Returns `None` when running in memory mode (no `pg_pool`).
#[cfg(feature = "postgres")]
pub async fn postgres_workspace_metrics(
    state: &AppState,
    workspace_id: Uuid,
) -> Option<(usize, u64, usize, usize)> {
    state.pg_pool.as_ref()?;

    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .ok()?;

    Some((
        stats.document_count,
        stats.storage_bytes as u64,
        stats.chunk_count,
        stats.embedding_count,
    ))
}

/// Fetch `(document_count, storage_bytes)` from the relational `documents` table.
///
/// Returns `None` when running in memory mode (no `pg_pool`).
#[cfg(feature = "postgres")]
pub async fn postgres_document_metrics(
    state: &AppState,
    workspace_id: Uuid,
) -> Option<(usize, u64)> {
    postgres_workspace_metrics(state, workspace_id)
        .await
        .map(|(docs, bytes, _, _)| (docs, bytes))
}

#[cfg(not(feature = "postgres"))]
pub async fn postgres_workspace_metrics(
    _state: &AppState,
    _workspace_id: Uuid,
) -> Option<(usize, u64, usize, usize)> {
    None
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
                display_status: None,
                ui_phase: None,
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

/// Relational row scope for delete when KV is missing or tenant-scoped out.
///
/// First principle: if the documents list shows a row (PG read model), delete
/// must succeed even when KV metadata/chunks/content are absent.
#[derive(Debug, Clone)]
pub struct RelationalDocumentScope {
    pub workspace_id: String,
    pub status: String,
    pub track_id: Option<String>,
}

/// Look up a document in the relational `documents` table under tenant scope.
#[cfg(feature = "postgres")]
pub async fn relational_document_scope(
    pool: Option<&sqlx::PgPool>,
    document_id: &str,
    tenant_ctx: &TenantContext,
) -> Result<Option<RelationalDocumentScope>, crate::error::ApiError> {
    use crate::error::ApiError;
    use sqlx::Row;

    let Some(pool) = pool else {
        return Ok(None);
    };

    let Ok(doc_uuid) = Uuid::parse_str(document_id) else {
        return Ok(None);
    };

    let workspace_id = tenant_ctx
        .workspace_id
        .as_ref()
        .and_then(|w| Uuid::parse_str(w).ok())
        .ok_or_else(|| ApiError::BadRequest("workspace_id required".into()))?;

    let tenant_uuid = tenant_ctx
        .tenant_id
        .as_ref()
        .and_then(|t| Uuid::parse_str(t).ok());

    let row = sqlx::query(
        r#"
        SELECT workspace_id::text AS workspace_id, status, track_id
        FROM documents
        WHERE id = $1
          AND workspace_id = $2
          AND ($3::uuid IS NULL OR tenant_id IS NULL OR tenant_id = $3)
        "#,
    )
    .bind(doc_uuid)
    .bind(workspace_id)
    .bind(tenant_uuid)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to lookup relational document: {e}")))?;

    Ok(row.map(|r| RelationalDocumentScope {
        workspace_id: r.get("workspace_id"),
        status: r.get("status"),
        track_id: r.get("track_id"),
    }))
}

#[cfg(not(feature = "postgres"))]
pub async fn relational_document_scope<P>(
    _pool: Option<&P>,
    _document_id: &str,
    _tenant_ctx: &TenantContext,
) -> Result<Option<RelationalDocumentScope>, crate::error::ApiError> {
    Ok(None)
}

/// Delete all relational `documents` rows for the request workspace (bulk delete SSOT).
#[cfg(feature = "postgres")]
pub async fn delete_relational_documents_for_workspace(
    pool: Option<&sqlx::PgPool>,
    tenant_ctx: &TenantContext,
) -> Result<u64, crate::error::ApiError> {
    use crate::error::ApiError;

    let Some(pool) = pool else {
        return Ok(0);
    };

    let workspace_id = tenant_ctx
        .workspace_id
        .as_ref()
        .and_then(|w| Uuid::parse_str(w).ok())
        .ok_or_else(|| ApiError::BadRequest("workspace_id required".into()))?;

    let tenant_uuid = tenant_ctx
        .tenant_id
        .as_ref()
        .and_then(|t| Uuid::parse_str(t).ok());

    let result = sqlx::query(
        r#"
        DELETE FROM documents
        WHERE workspace_id = $1
          AND ($2::uuid IS NULL OR tenant_id IS NULL OR tenant_id = $2)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_uuid)
    .execute(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to bulk-delete relational documents: {e}")))?;

    Ok(result.rows_affected())
}

#[cfg(not(feature = "postgres"))]
pub async fn delete_relational_documents_for_workspace<P>(
    _pool: Option<&P>,
    _tenant_ctx: &TenantContext,
) -> Result<u64, crate::error::ApiError> {
    Ok(0)
}

/// True when KV indicates the document is still moving through the pipeline.
///
/// Includes `current_stage=queued` (reprocess stage reset) which is not always
/// mirrored into `status` helpers as a standalone status string.
fn kv_summary_is_inflight(doc: &DocumentSummary) -> bool {
    use crate::document_metadata::is_active_processing_status;

    if doc
        .status
        .as_deref()
        .is_some_and(is_active_processing_status)
    {
        return true;
    }
    doc.current_stage.as_deref().is_some_and(|stage| {
        is_active_processing_status(stage) || stage.eq_ignore_ascii_case("queued")
    })
}

/// Merge KV-derived documents with relational rows (relational fills gaps).
///
/// Status rule (SPEC-054 reprocess honesty): when KV is in-flight, do **not**
/// let a stale relational terminal status (`completed`/`indexed`/`failed`)
/// overwrite it. Reprocess accept updates KV only; relational lags until the
/// worker mirrors status — unconditional overwrite made the list lie as Completed.
pub fn merge_document_summaries(
    mut kv_documents: Vec<DocumentSummary>,
    relational_documents: Vec<DocumentSummary>,
) -> Vec<DocumentSummary> {
    use crate::document_metadata::is_terminal_document_status;
    use std::collections::HashMap;

    let mut by_id: HashMap<String, usize> = HashMap::new();
    for (idx, doc) in kv_documents.iter().enumerate() {
        by_id.insert(doc.id.clone(), idx);
    }

    for rel in relational_documents {
        if let Some(&idx) = by_id.get(&rel.id) {
            // SPEC-045: relational title/counts are durable; KV may hold swapped blobs.
            let kv = &mut kv_documents[idx];
            if rel.title.is_some() {
                kv.title = rel.title.clone();
                kv.file_name = rel.file_name.clone();
            }
            if let Some(rel_status) = rel.status.as_deref() {
                let keep_kv_inflight =
                    kv_summary_is_inflight(kv) && is_terminal_document_status(rel_status);
                if !keep_kv_inflight {
                    kv.status = rel.status.clone();
                }
            }
            kv.chunk_count = kv.chunk_count.max(rel.chunk_count);
            if rel.entity_count.unwrap_or(0) > kv.entity_count.unwrap_or(0) {
                kv.entity_count = rel.entity_count;
            }
            if kv.created_at.is_none() {
                kv.created_at = rel.created_at.clone();
            }
            if rel.updated_at.is_some() {
                kv.updated_at = rel.updated_at.clone();
            }
            if kv.cost_usd.is_none() {
                kv.cost_usd = rel.cost_usd;
            }
        } else {
            by_id.insert(rel.id.clone(), kv_documents.len());
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
/// counts, so we fall back to `node_counts_by_source_prefixes` (SPEC-054 L1-a:
/// **one** AGE round-trip) for documents whose current `entity_count` is 0.
///
/// Whether a list row is eligible for AGE entity-count reconcile.
///
/// Only finished docs with a zero count — never in-progress extraction
/// (those probes can exceed the interactive read deadline on large graphs).
#[inline]
pub(crate) fn should_reconcile_entity_count(
    status: Option<&str>,
    entity_count: Option<usize>,
) -> bool {
    entity_count.unwrap_or(0) == 0
        && matches!(status, Some("completed" | "indexed" | "partial_failure"))
}

/// Best-effort: AGE failures/timeouts are swallowed (counts stay as-is) so the
/// interactive list never fails with `read_path_busy` due to a graph probe.
pub async fn reconcile_entity_counts_with_graph(
    storage: &crate::state::StorageRuntime,
    documents: &mut [DocumentSummary],
) {
    use std::time::Duration;

    use edgequake_storage::kv_keys;

    let candidates: Vec<(usize, String)> = documents
        .iter()
        .enumerate()
        .filter(|(_, d)| should_reconcile_entity_count(d.status.as_deref(), d.entity_count))
        .map(|(i, d)| (i, d.id.clone()))
        .collect();

    if candidates.is_empty() {
        return;
    }

    let prefixes: Vec<String> = candidates
        .iter()
        .map(|(_, doc_id)| kv_keys::doc_chunk_prefix(doc_id))
        .collect();

    // Hard cap well under EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS so list/detail
    // keep serving KV counts when AGE is slow (142k+ node graphs).
    const AGE_RECONCILE_TIMEOUT: Duration = Duration::from_millis(400);
    let counts = match tokio::time::timeout(
        AGE_RECONCILE_TIMEOUT,
        storage
            .graph_storage
            .node_counts_by_source_prefixes(&prefixes),
    )
    .await
    {
        Ok(Ok(map)) => map,
        Ok(Err(e)) => {
            tracing::warn!(
                error = %e,
                candidate_count = candidates.len(),
                "P-A3: batched AGE entity_count fallback failed (non-fatal) — leaving counts as-is"
            );
            return;
        }
        Err(_) => {
            tracing::warn!(
                candidate_count = candidates.len(),
                timeout_ms = AGE_RECONCILE_TIMEOUT.as_millis() as u64,
                "P-A3: AGE entity_count reconcile timed out — serving KV counts"
            );
            return;
        }
    };

    for ((idx, _doc_id), prefix) in candidates.into_iter().zip(prefixes) {
        if let Some(age_count) = counts.get(&prefix).copied() {
            if age_count > 0 {
                documents[idx].entity_count = Some(age_count);
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
    fn reconcile_skips_in_progress_and_nonzero_counts() {
        assert!(should_reconcile_entity_count(Some("completed"), Some(0)));
        assert!(should_reconcile_entity_count(Some("indexed"), None));
        assert!(should_reconcile_entity_count(
            Some("partial_failure"),
            Some(0)
        ));
        assert!(!should_reconcile_entity_count(Some("extracting"), None));
        assert!(!should_reconcile_entity_count(Some("processing"), Some(0)));
        assert!(!should_reconcile_entity_count(Some("completed"), Some(7)));
        assert!(!should_reconcile_entity_count(Some("failed"), Some(0)));
        assert!(!should_reconcile_entity_count(Some("pending"), None));
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
            display_status: None,
            ui_phase: None,
        }];

        let pg = vec![DocumentSummary {
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
            display_status: None,
            ui_phase: None,
        }];

        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, "a");
        // SPEC-045: relational title overlays corrupted KV title for same id.
        assert_eq!(merged[0].title.as_deref(), Some("PG duplicate"));
    }

    #[test]
    fn merge_document_summaries_adds_pg_only_rows() {
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
            display_status: None,
            ui_phase: None,
        }];

        let pg = vec![DocumentSummary {
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
            display_status: None,
            ui_phase: None,
        }];

        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, "b");
        assert_eq!(merged[1].id, "a");
    }

    fn summary(id: &str, status: &str, stage: Option<&str>) -> DocumentSummary {
        DocumentSummary {
            id: id.into(),
            title: Some(id.into()),
            file_name: None,
            content_summary: None,
            content_length: None,
            chunk_count: 0,
            entity_count: None,
            status: Some(status.into()),
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
            current_stage: stage.map(str::to_string),
            stage_progress: None,
            stage_message: None,
            pdf_id: None,
            display_status: None,
            ui_phase: None,
        }
    }

    #[test]
    fn merge_keeps_kv_processing_over_stale_relational_completed() {
        let kv = vec![summary("doc-1", "processing", Some("queued"))];
        let pg = vec![summary("doc-1", "completed", Some("completed"))];
        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].status.as_deref(),
            Some("processing"),
            "in-flight KV must win over stale relational completed"
        );
        assert_eq!(merged[0].current_stage.as_deref(), Some("queued"));
    }

    #[test]
    fn merge_keeps_kv_processing_over_relational_indexed() {
        let kv = vec![summary("doc-1", "processing", Some("extracting"))];
        let pg = vec![summary("doc-1", "indexed", None)];
        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged[0].status.as_deref(), Some("processing"));
    }

    #[test]
    fn merge_allows_relational_completed_when_kv_already_completed() {
        let kv = vec![summary("doc-1", "completed", Some("completed"))];
        let pg = vec![summary("doc-1", "indexed", None)];
        let merged = merge_document_summaries(kv, pg);
        // KV terminal → relational still wins (normalize happens upstream on PG load).
        assert_eq!(merged[0].status.as_deref(), Some("indexed"));
    }
}
