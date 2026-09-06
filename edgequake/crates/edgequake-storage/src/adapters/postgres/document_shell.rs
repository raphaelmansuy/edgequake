//! SPEC-091 Wave C — typed document shell backing for the metadata / content /
//! staging KV families (`{doc}-metadata`, `{doc}-content`, `staging:{doc}-*`).
//!
//! The `documents` row is the single shell: admission creates it (staging),
//! promote overwrites it (final). A staging shell is discriminated by the
//! `_shell: "staging"` marker the dual-write merges into `metadata`; the final
//! metadata write overwrites the whole object, clearing the marker — so a
//! staging read after promote correctly misses, mirroring the KV key delete.
//!
//! Cutover pattern: writes stay KV-authoritative with a warn-only typed
//! dual-write; reads are flag-gated (`EDGEQUAKE_KV_FAMILY_METADATA`) typed-
//! first with KV fallback on any gap (no row, empty metadata/content).

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::StorageError;

/// Marker key merged into `documents.metadata` while a shell is staging.
pub const STAGING_SHELL_MARKER: &str = "_shell";
pub const STAGING_SHELL_VALUE: &str = "staging";

/// Shell KV key families routed to the `documents` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Metadata,
    Content,
    StagingMetadata,
    StagingContent,
}

/// Parse a shell key into (kind, document uuid). Non-UUID or non-shell keys
/// return `None` (caller keeps the plain KV path).
pub fn parse_shell_key(key: &str) -> Option<(ShellKind, Uuid)> {
    if let Some(rest) = key.strip_prefix("staging:") {
        if let Some(doc) = rest.strip_suffix("-metadata") {
            return Uuid::parse_str(doc)
                .ok()
                .map(|u| (ShellKind::StagingMetadata, u));
        }
        if let Some(doc) = rest.strip_suffix("-content") {
            return Uuid::parse_str(doc)
                .ok()
                .map(|u| (ShellKind::StagingContent, u));
        }
        return None;
    }
    if let Some(doc) = key.strip_suffix("-metadata") {
        return Uuid::parse_str(doc).ok().map(|u| (ShellKind::Metadata, u));
    }
    if let Some(doc) = key.strip_suffix("-content") {
        return Uuid::parse_str(doc).ok().map(|u| (ShellKind::Content, u));
    }
    None
}

fn legacy_content_value(text: String) -> Value {
    json!({ "content": text })
}

// SPEC-129: SSOT lives in crate-root `documents_column_status` so memory +
// postgres dual-writes share one mapper (no feature-gated drift).
pub use crate::documents_column_status::{
    normalize_documents_column_status, relational_documents_status_for_write,
};

fn status_from_metadata(value: &Value) -> String {
    value
        .get("status")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(normalize_documents_column_status)
        .unwrap_or_else(|| "processing".to_string())
}

/// True when JSONB metadata is missing or `{}` (column SSOT must synthesize).
fn metadata_json_is_empty(metadata: &Option<Value>) -> bool {
    match metadata {
        None => true,
        Some(Value::Object(map)) => map.is_empty(),
        Some(Value::Null) => true,
        _ => false,
    }
}

/// Synthesize a KV-shaped metadata object from relational document columns.
///
/// SPEC-091 / SPEC-021: list already merges relational rows when shell
/// metadata JSONB is empty; detail must do the same or list→detail 404s.
fn synthesize_metadata_from_columns(
    title: Option<String>,
    status: Option<String>,
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    track_id: Option<String>,
    chunk_count: Option<i32>,
    entity_count: Option<i32>,
    file_size_bytes: Option<i64>,
    error_message: Option<String>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Value {
    let status = status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(normalize_documents_column_status)
        .unwrap_or_else(|| "processing".to_string());
    let mut map = serde_json::Map::new();
    map.insert("status".into(), Value::String(status));
    if let Some(t) = title {
        map.insert("title".into(), Value::String(t.clone()));
        map.insert("file_name".into(), Value::String(t));
    }
    if let Some(t) = tenant_id {
        map.insert("tenant_id".into(), Value::String(t));
    }
    if let Some(w) = workspace_id {
        map.insert("workspace_id".into(), Value::String(w));
    }
    if let Some(tr) = track_id {
        map.insert("track_id".into(), Value::String(tr));
    }
    if let Some(n) = chunk_count {
        map.insert("chunk_count".into(), json!(n.max(0)));
    }
    if let Some(n) = entity_count {
        map.insert("entity_count".into(), json!(n.max(0)));
    }
    if let Some(n) = file_size_bytes {
        map.insert("file_size".into(), json!(n.max(0)));
    }
    if let Some(e) = error_message {
        map.insert("error_message".into(), Value::String(e));
    }
    if let Some(ts) = created_at {
        map.insert("created_at".into(), Value::String(ts.to_rfc3339()));
    }
    if let Some(ts) = updated_at {
        map.insert("updated_at".into(), Value::String(ts.to_rfc3339()));
    }
    Value::Object(map)
}

/// Column SSOT for synthesizing empty metadata shells.
type ShellSynthCols = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<i64>,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
);

fn resolve_metadata_value(
    metadata: Option<Value>,
    shell: Option<String>,
    cols: ShellSynthCols,
) -> Option<Value> {
    let is_staging = shell.as_deref() == Some(STAGING_SHELL_VALUE);
    let _ = is_staging; // staging filter applied by caller for Staging* kinds
    if !metadata_json_is_empty(&metadata) {
        return metadata;
    }
    Some(synthesize_metadata_from_columns(
        cols.0, cols.1, cols.2, cols.3, cols.4, cols.5, cols.6, cols.7, cols.8, cols.9, cols.10,
    ))
}

/// Typed single-key read. Returns `Ok(None)` on miss (→ KV fallback).
///
/// Empty `metadata` JSONB (`{}` / NULL) is **not** a miss for Metadata shells:
/// we synthesize from relational columns so list→detail stays consistent
/// (SPEC-021 / SPEC-091 relational primary).
pub async fn shell_value_by_key(pool: &PgPool, key: &str) -> Result<Option<Value>, StorageError> {
    let Some((kind, doc)) = parse_shell_key(key) else {
        return Ok(None);
    };
    let row: Option<(
        Option<Value>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i32>,
        Option<i32>,
        Option<i64>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT metadata, NULLIF(content, ''), metadata->>'_shell', \
                title, status, tenant_id::text, workspace_id::text, track_id, \
                chunk_count, entity_count, file_size_bytes, error_message, \
                created_at, updated_at \
         FROM public.documents WHERE id = $1",
    )
    .bind(doc)
    .fetch_optional(pool)
    .await
    .map_err(|e| StorageError::Database(format!("document shell read failed: {e}")))?;

    let Some((
        metadata,
        content,
        shell,
        title,
        status,
        tenant_id,
        workspace_id,
        track_id,
        chunk_count,
        entity_count,
        file_size_bytes,
        error_message,
        created_at,
        updated_at,
    )) = row
    else {
        return Ok(None);
    };

    let cols = (
        title,
        status,
        tenant_id,
        workspace_id,
        track_id,
        chunk_count,
        entity_count,
        file_size_bytes,
        error_message,
        created_at,
        updated_at,
    );
    let is_staging = shell.as_deref() == Some(STAGING_SHELL_VALUE);
    let resolved_meta = resolve_metadata_value(metadata, shell.clone(), cols);

    Ok(match kind {
        ShellKind::Metadata => resolved_meta,
        ShellKind::Content => content.map(legacy_content_value),
        ShellKind::StagingMetadata if is_staging => resolved_meta,
        ShellKind::StagingContent if is_staging => content.map(legacy_content_value),
        _ => None,
    })
}

/// Ordered batch read for shell keys; positions of non-shell keys are left
/// `None` for the caller to merge from KV (mirrors the chunk-text flow).
pub async fn shell_values_ordered(
    pool: &PgPool,
    ids: &[String],
) -> Result<Vec<Option<Value>>, StorageError> {
    let parsed: Vec<Option<(ShellKind, Uuid)>> = ids.iter().map(|k| parse_shell_key(k)).collect();
    let mut out: Vec<Option<Value>> = vec![None; ids.len()];

    let shell_idx: Vec<usize> = parsed
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_some())
        .map(|(i, _)| i)
        .collect();
    if shell_idx.is_empty() {
        return Ok(out);
    }

    let uuids: Vec<Uuid> = shell_idx.iter().map(|&i| parsed[i].unwrap().1).collect();
    type BatchRow = (
        Uuid,
        Option<Value>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i32>,
        Option<i32>,
        Option<i64>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    );
    let rows: Vec<BatchRow> = sqlx::query_as(
        "SELECT id, metadata, NULLIF(content, ''), metadata->>'_shell', \
                title, status, tenant_id::text, workspace_id::text, track_id, \
                chunk_count, entity_count, file_size_bytes, error_message, \
                created_at, updated_at \
         FROM public.documents WHERE id = ANY($1)",
    )
    .bind(&uuids)
    .fetch_all(pool)
    .await
    .map_err(|e| StorageError::Database(format!("document shell batch read failed: {e}")))?;

    let map: std::collections::HashMap<Uuid, BatchRow> =
        rows.into_iter().map(|r| (r.0, r)).collect();

    for &i in &shell_idx {
        let (kind, doc) = parsed[i].unwrap();
        let Some(row) = map.get(&doc) else {
            continue;
        };
        let (
            _,
            metadata,
            content,
            shell,
            title,
            status,
            tenant_id,
            workspace_id,
            track_id,
            chunk_count,
            entity_count,
            file_size_bytes,
            error_message,
            created_at,
            updated_at,
        ) = row;
        let cols = (
            title.clone(),
            status.clone(),
            tenant_id.clone(),
            workspace_id.clone(),
            track_id.clone(),
            *chunk_count,
            *entity_count,
            *file_size_bytes,
            error_message.clone(),
            *created_at,
            *updated_at,
        );
        let is_staging = shell.as_deref() == Some(STAGING_SHELL_VALUE);
        let resolved_meta = resolve_metadata_value(metadata.clone(), shell.clone(), cols);
        out[i] = match kind {
            ShellKind::Metadata => resolved_meta,
            ShellKind::Content => content.clone().map(legacy_content_value),
            ShellKind::StagingMetadata if is_staging => resolved_meta,
            ShellKind::StagingContent if is_staging => content.clone().map(legacy_content_value),
            _ => None,
        };
    }
    Ok(out)
}

/// Warn-only typed dual-write for a KV upsert batch. Never fails the caller:
/// the KV write already happened authoritatively.
/// Typed shell upsert for metadata/content/staging keys.
///
/// `authoritative = false` (dual-write phase): failures are warn-only — KV
/// remains the rollback authority. `authoritative = true` (write-stop phase,
/// `EDGEQUAKE_KV_FAMILY_METADATA=relational`): the first failure aborts with
/// `Err` so callers surface the loss instead of silently dropping shell state.
///
/// GAP-091-16 (SPEC-091 IW1, LAW-D7): one `unnest` round trip per shell kind
/// (≤4 total) instead of one INSERT per key — an N-key batch costs O(1) round
/// trips, not O(N).
pub async fn dual_write_shell_upserts(
    pool: &PgPool,
    pairs: &[(String, Value)],
    authoritative: bool,
) -> Result<(), StorageError> {
    let buckets = partition_shell_pairs(pairs);
    for (kind, bucket) in &buckets {
        if bucket.is_empty() {
            continue;
        }
        let result = match kind {
            ShellKind::Metadata => batch_shell_metadata(pool, bucket).await,
            ShellKind::Content => batch_shell_content(pool, bucket).await,
            ShellKind::StagingMetadata => batch_shell_staging_metadata(pool, bucket).await,
            ShellKind::StagingContent => batch_shell_content(pool, bucket).await,
        };
        if let Err(e) = result {
            if authoritative {
                return Err(StorageError::Database(format!(
                    "typed document shell batch write failed for {kind:?} ({} row(s), authoritative): {e}",
                    bucket.len()
                )));
            }
            tracing::warn!(
                kind = ?kind,
                rows = bucket.len(),
                error = %e,
                "typed document shell dual-write batch failed (KV remains)"
            );
        }
    }
    Ok(())
}

/// A parsed shell row ready for one `unnest` batch (DRY across kinds).
struct ShellBatchRow<'a> {
    doc: Uuid,
    value: &'a Value,
}

/// Group a KV upsert batch by shell kind, dropping non-shell keys
/// (`parse_shell_key` rejects non-UUID/non-shell keys — they keep KV only).
fn partition_shell_pairs<'a>(
    pairs: &'a [(String, Value)],
) -> [(ShellKind, Vec<ShellBatchRow<'a>>); 4] {
    let mut buckets: [(ShellKind, Vec<ShellBatchRow<'a>>); 4] = [
        (ShellKind::Metadata, Vec::new()),
        (ShellKind::Content, Vec::new()),
        (ShellKind::StagingMetadata, Vec::new()),
        (ShellKind::StagingContent, Vec::new()),
    ];
    for (key, value) in pairs {
        let Some((kind, doc)) = parse_shell_key(key) else {
            continue;
        };
        let bucket = buckets
            .iter_mut()
            .find(|(k, _)| *k == kind)
            .map(|(_, v)| v)
            .expect("every ShellKind has a bucket");
        bucket.push(ShellBatchRow { doc, value });
    }
    buckets
}

/// One round trip for `Metadata` shells: FK-guarded workspace/tenant column
/// population via LEFT JOIN (mirrors the pre-batch per-row statement).
async fn batch_shell_metadata(
    pool: &PgPool,
    rows: &[ShellBatchRow<'_>],
) -> Result<(), sqlx::Error> {
    let ids: Vec<Uuid> = rows.iter().map(|r| r.doc).collect();
    let metas: Vec<Value> = rows.iter().map(|r| r.value.clone()).collect();
    let titles: Vec<String> = rows
        .iter()
        .map(|r| {
            r.value
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();
    let workspaces: Vec<Option<Uuid>> = rows
        .iter()
        .map(|r| {
            r.value
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
        })
        .collect();
    let tenants: Vec<Option<Uuid>> = rows
        .iter()
        .map(|r| {
            r.value
                .get("tenant_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
        })
        .collect();
    // Status column follows metadata->>'status' but must stay CHECK-safe
    // (migration 032). KV may carry queued/deleting/stage slugs — normalize.
    let statuses: Vec<String> = rows.iter().map(|r| status_from_metadata(r.value)).collect();
    sqlx::query(
        "INSERT INTO public.documents (id, title, content, status, metadata, workspace_id, tenant_id) \
         SELECT u.id, u.title, '', u.status, u.metadata, w.workspace_id, t.tenant_id \
         FROM unnest($1::uuid[], $2::jsonb[], $3::text[], $4::uuid[], $5::uuid[], $6::text[]) \
             AS u(id, metadata, title, ws, tn, status) \
         LEFT JOIN workspaces w ON w.workspace_id = u.ws \
         LEFT JOIN tenants t ON t.tenant_id = u.tn \
         ON CONFLICT (id) DO UPDATE SET metadata = EXCLUDED.metadata, \
             title = CASE WHEN EXCLUDED.title = '' THEN documents.title \
                          ELSE EXCLUDED.title END, \
             status = EXCLUDED.status, \
             workspace_id = COALESCE(documents.workspace_id, EXCLUDED.workspace_id), \
             tenant_id = COALESCE(documents.tenant_id, EXCLUDED.tenant_id), \
             updated_at = now()",
    )
    .bind(&ids)
    .bind(&metas)
    .bind(&titles)
    .bind(&workspaces)
    .bind(&tenants)
    .bind(&statuses)
    .execute(pool)
    .await?;
    Ok(())
}

/// One round trip for `Content`/`StagingContent` shells (identical shape).
async fn batch_shell_content(pool: &PgPool, rows: &[ShellBatchRow<'_>]) -> Result<(), sqlx::Error> {
    let ids: Vec<Uuid> = rows.iter().map(|r| r.doc).collect();
    let texts: Vec<String> = rows
        .iter()
        .map(|r| {
            r.value
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();
    sqlx::query(
        "INSERT INTO public.documents (id, content, status) \
         SELECT u.id, u.content, 'processing' \
         FROM unnest($1::uuid[], $2::text[]) AS u(id, content) \
         ON CONFLICT (id) DO UPDATE SET content = EXCLUDED.content, \
             updated_at = now()",
    )
    .bind(&ids)
    .bind(&texts)
    .execute(pool)
    .await?;
    Ok(())
}

/// One round trip for `StagingMetadata` shells (marker injected per row).
/// Promotes `metadata.title` to the `title` column so list reads do not stick
/// on the schema DEFAULT `'Untitled'` while the doc is still staging.
async fn batch_shell_staging_metadata(
    pool: &PgPool,
    rows: &[ShellBatchRow<'_>],
) -> Result<(), sqlx::Error> {
    let ids: Vec<Uuid> = rows.iter().map(|r| r.doc).collect();
    let shells: Vec<Value> = rows
        .iter()
        .map(|r| {
            let mut shell = r.value.clone();
            shell[STAGING_SHELL_MARKER] = json!(STAGING_SHELL_VALUE);
            shell
        })
        .collect();
    let titles: Vec<String> = rows
        .iter()
        .map(|r| {
            r.value
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();
    let statuses: Vec<String> = rows.iter().map(|r| status_from_metadata(r.value)).collect();
    sqlx::query(
        "INSERT INTO public.documents (id, title, content, status, metadata) \
         SELECT u.id, u.title, '', u.status, u.metadata \
         FROM unnest($1::uuid[], $2::jsonb[], $3::text[], $4::text[]) \
             AS u(id, metadata, title, status) \
         ON CONFLICT (id) DO UPDATE SET metadata = EXCLUDED.metadata, \
             title = CASE WHEN EXCLUDED.title = '' THEN documents.title \
                          ELSE EXCLUDED.title END, \
             status = EXCLUDED.status, updated_at = now()",
    )
    .bind(&ids)
    .bind(&shells)
    .bind(&titles)
    .bind(&statuses)
    .execute(pool)
    .await?;
    Ok(())
}

/// Synthesize `-metadata` keys for all shell-backed documents (suffix scan
/// replacement in relational mode). Empty-metadata rows are excluded, matching
/// the typed-read miss rule.
pub async fn shell_metadata_keys(
    pool: &PgPool,
    limit: Option<usize>,
) -> Result<Vec<String>, StorageError> {
    let sql = match limit {
        Some(_) => {
            "SELECT id::text || '-metadata' FROM public.documents \
             WHERE metadata IS NOT NULL AND metadata <> '{}'::jsonb ORDER BY id LIMIT $1"
        }
        None => {
            "SELECT id::text || '-metadata' FROM public.documents \
             WHERE metadata IS NOT NULL AND metadata <> '{}'::jsonb ORDER BY id"
        }
    };
    let mut q = sqlx::query_scalar::<_, String>(sql);
    if let Some(l) = limit {
        q = q.bind(l as i64);
    }
    q.fetch_all(pool)
        .await
        .map_err(|e| StorageError::Database(format!("document shell key scan failed: {e}")))
}

/// Typed atomic status CAS on a shell metadata key — relational replacement
/// for `KVStorage::transition_if_status` (`UPDATE ... WHERE status = expected`
/// in one statement, same atomicity). Returns `Ok(None)` for non-shell keys
/// (caller keeps the KV path) and `Ok(Some(bool))` = rows transitioned.
pub async fn shell_transition_status(
    pool: &PgPool,
    key: &str,
    expected_status: &str,
    new_status: &str,
) -> Result<Option<bool>, StorageError> {
    let Some((kind, doc)) = parse_shell_key(key) else {
        return Ok(None);
    };
    // Status transitions only make sense on metadata shells; content keys
    // defer to the caller's KV path.
    let staging = match kind {
        ShellKind::Metadata => false,
        ShellKind::StagingMetadata => true,
        _ => return Ok(None),
    };
    let sql = if staging {
        "UPDATE public.documents \
         SET metadata = jsonb_set(metadata, '{status}', to_jsonb($3::text)), updated_at = now() \
         WHERE id = $1 AND metadata->>'status' = $2 \
           AND metadata->>'_shell' = 'staging'"
    } else {
        "UPDATE public.documents \
         SET metadata = jsonb_set(metadata, '{status}', to_jsonb($3::text)), updated_at = now() \
         WHERE id = $1 AND metadata->>'status' = $2"
    };
    let result = sqlx::query(sql)
        .bind(doc)
        .bind(expected_status)
        .bind(new_status)
        .execute(pool)
        .await
        .map_err(|e| StorageError::Database(format!("shell status transition failed: {e}")))?;
    Ok(Some(result.rows_affected() == 1))
}

/// Default page size for staging-shell keyset scans (GAP-091-24 / IW1).
pub const SHELL_STAGING_KEYS_PAGE: i64 = 1000;

/// Synthesize `staging:{id}-metadata` / `-content` keys for live staging
/// shells (prefix scan replacement in relational mode).
///
/// SPEC-091 IW1 (GAP-091-24): bounded keyset pagination — never a LIMIT-less
/// scan. Callers that need the full set page through via `after_id`.
pub async fn shell_staging_keys(pool: &PgPool) -> Result<Vec<String>, StorageError> {
    shell_staging_keys_page(pool, None, SHELL_STAGING_KEYS_PAGE).await
}

/// Keyset page of staging-shell keys (`id > after_id`, ordered by `id`).
pub async fn shell_staging_keys_page(
    pool: &PgPool,
    after_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<String>, StorageError> {
    let limit = limit.clamp(1, 5000);
    let rows: Vec<(String, bool)> = sqlx::query_as(
        "SELECT id::text, (NULLIF(content, '') IS NOT NULL) FROM public.documents \
         WHERE metadata->>'_shell' = 'staging' \
           AND ($1::uuid IS NULL OR id > $1) \
         ORDER BY id LIMIT $2",
    )
    .bind(after_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| StorageError::Database(format!("staging shell key scan failed: {e}")))?;
    let mut keys = Vec::with_capacity(rows.len() * 2);
    for (id, has_content) in rows {
        keys.push(format!("staging:{id}-metadata"));
        if has_content {
            keys.push(format!("staging:{id}-content"));
        }
    }
    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_shell_key_routes_all_four_families() {
        let doc = Uuid::new_v4().to_string();
        assert!(matches!(
            parse_shell_key(&format!("{doc}-metadata")),
            Some((ShellKind::Metadata, _))
        ));
        assert!(matches!(
            parse_shell_key(&format!("{doc}-content")),
            Some((ShellKind::Content, _))
        ));
        assert!(matches!(
            parse_shell_key(&format!("staging:{doc}-metadata")),
            Some((ShellKind::StagingMetadata, _))
        ));
        assert!(matches!(
            parse_shell_key(&format!("staging:{doc}-content")),
            Some((ShellKind::StagingContent, _))
        ));
        assert!(parse_shell_key("not-a-uuid-metadata").is_none());
        assert!(parse_shell_key("doc:hash:ws:abc").is_none());
        assert!(parse_shell_key("staging:hash:ws:abc").is_none());
    }

    #[test]
    fn content_value_wraps_legacy_shape() {
        assert_eq!(
            legacy_content_value("hello".into()),
            json!({"content": "hello"})
        );
    }

    #[test]
    fn empty_metadata_json_detected() {
        assert!(metadata_json_is_empty(&None));
        assert!(metadata_json_is_empty(&Some(Value::Null)));
        assert!(metadata_json_is_empty(&Some(json!({}))));
        assert!(!metadata_json_is_empty(&Some(json!({"status": "processing"}))));
    }

    #[test]
    fn synthesize_metadata_uses_column_ssot() {
        let v = synthesize_metadata_from_columns(
            Some("spec129-dual".into()),
            Some("processing".into()),
            Some("tenant-1".into()),
            Some("ws-1".into()),
            None,
            Some(0),
            None,
            None,
            None,
            None,
            None,
        );
        assert_eq!(v.get("title").and_then(|x| x.as_str()), Some("spec129-dual"));
        assert_eq!(v.get("status").and_then(|x| x.as_str()), Some("processing"));
        assert_eq!(v.get("tenant_id").and_then(|x| x.as_str()), Some("tenant-1"));
        assert_eq!(v.get("workspace_id").and_then(|x| x.as_str()), Some("ws-1"));
    }

    #[test]
    fn resolve_metadata_synthesizes_when_empty() {
        let cols = (
            Some("t".into()),
            Some("failed".into()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let v = resolve_metadata_value(Some(json!({})), None, cols).expect("synth");
        assert_eq!(v.get("title").and_then(|x| x.as_str()), Some("t"));
        assert_eq!(v.get("status").and_then(|x| x.as_str()), Some("failed"));
    }

    // SPEC-129: vocabulary matrix lives in `documents_column_status` unit tests.
}
