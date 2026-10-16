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
pub const MERGE_STRATEGY: &str = "relational_primary_kv_fallback";

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
    state.optional_pg_pool()?;

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

/// Result of a bounded relational documents list (GH-400).
#[derive(Debug, Clone)]
pub struct RelationalDocumentList {
    pub documents: Vec<DocumentSummary>,
    /// True when the scan hit `max_entries` — corpus may have more rows.
    pub truncated: bool,
}

/// Load document summaries from the relational `documents` table for a workspace.
///
/// Used to backfill the documents list when KV metadata is missing or scoped to
/// a legacy workspace id.
///
/// GH-400 / LAW-H2:
/// - Never detoast `documents.content` (`LEFT`/`LENGTH` hold pool slots for seconds).
/// - Cap keys with `LIMIT` (interactive envelope, mirrors KV `MAX_LIST_METADATA_ENTRIES`).
/// - Run under `SET LOCAL statement_timeout` so a Rust deadline cannot leave a
///   zombie backend.
#[cfg(feature = "postgres")]
pub async fn list_relational_document_summaries(
    pool: crate::services::OptionalPgPool<'_>,
    tenant_ctx: &TenantContext,
) -> Result<Vec<DocumentSummary>, crate::error::ApiError> {
    Ok(list_relational_document_summaries_limited(
        pool,
        tenant_ctx,
        crate::read_path::MAX_LIST_METADATA_ENTRIES,
        None,
        RelationalCreatedBounds::default(),
        &[],
    )
    .await?
    .documents)
}

/// Columns the documents page renders. `content` stays out (GH-400).
#[cfg(feature = "postgres")]
const DOCUMENT_LIST_PROJECTION: &str = r#"
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
            -- GH-400: never detoast `content`. Preview + length come from
            -- metadata / file_size_bytes (idx_documents_workspace_created).
            LEFT(COALESCE(metadata->>'content_summary', metadata->>'content_preview', ''), 200)
                AS content_preview,
            -- file_size_bytes is BIGINT; COALESCE(int, bigint) → INT8. Decode as i64.
            COALESCE(
                NULLIF(metadata->>'content_length', '')::bigint,
                file_size_bytes
            ) AS content_length,
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
            (metadata->>'total_tokens')::bigint AS total_tokens,
            metadata->>'current_stage' AS current_stage
"#;

#[cfg(feature = "postgres")]
const DOCUMENT_PAGE_COLUMNS: &str = "id, title, status, chunk_count, entity_count, file_size_bytes, track_id, error_message, created_at, updated_at, metadata";

/// One required arm of a status chip. Literals come from this list, not from
/// the request, so the statement can name the index column.
#[cfg(feature = "postgres")]
#[derive(Debug)]
enum ChipArm {
    Status(&'static str),
    /// `current_stage` equality. Rows whose `status` is already covered by a
    /// status arm of the same chip are excluded, so a row is returned once.
    Stage(&'static str),
}

#[cfg(feature = "postgres")]
#[derive(Debug)]
enum RelationalListFilter {
    Open,
    /// Bound `status = $n` values. `completed` is two values.
    Statuses(Vec<String>),
    /// Fixed chip. Each arm is an equality the index can stop after LIMIT.
    Chip(&'static [ChipArm]),
}

#[cfg(feature = "postgres")]
static PENDING_ARMS: &[ChipArm] = &[
    ChipArm::Status("pending"),
    ChipArm::Status("queued"),
    ChipArm::Stage("queued"),
];

#[cfg(feature = "postgres")]
static PROCESSING_ARMS: &[ChipArm] = &[
    ChipArm::Status("processing"),
    ChipArm::Stage("converting"),
    ChipArm::Stage("preprocessing"),
    ChipArm::Stage("chunking"),
    ChipArm::Stage("extracting"),
    ChipArm::Stage("gleaning"),
    ChipArm::Stage("merging"),
    ChipArm::Stage("summarizing"),
    ChipArm::Stage("embedding"),
    ChipArm::Stage("storing"),
    ChipArm::Stage("projecting"),
    ChipArm::Stage("indexing"),
];

/// Statuses the list keeps for `status=unknown`. The column is `NOT NULL`,
/// so a SQL `NULL` arm never matches. `queued` cannot be stored.
#[cfg(feature = "postgres")]
static UNKNOWN_ARMS: &[ChipArm] = &[
    ChipArm::Status("chunking"),
    ChipArm::Status("extracting"),
    ChipArm::Status("embedding"),
    ChipArm::Status("indexing"),
    ChipArm::Status("deleting"),
    ChipArm::Status("delete_failed"),
];

/// How the list requires a passed status.
///
/// `unknown` is the complement of the known chips. `NOT IN` seq-scans
/// (9.568 ms, 79,950 removed). One equality per stored complement value
/// stops on `idx_documents_workspace_status_created`. `all` stays open.
/// `pending` and `processing` also match `current_stage`. Each of those
/// values is its own arm: `current_stage = ANY($n)` hides the stage from a
/// generic plan and walks every newer row (12.428 ms, 79,950 removed).
/// `completed` is the stored pair `completed` and `indexed`. Every other
/// passed value is one `status` equality.
#[cfg(feature = "postgres")]
fn relational_list_filter(raw: Option<&str>) -> RelationalListFilter {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return RelationalListFilter::Open;
    };
    match raw.to_ascii_lowercase().as_str() {
        "all" => RelationalListFilter::Open,
        "unknown" => RelationalListFilter::Chip(UNKNOWN_ARMS),
        "pending" => RelationalListFilter::Chip(PENDING_ARMS),
        "processing" => RelationalListFilter::Chip(PROCESSING_ARMS),
        "completed" => {
            RelationalListFilter::Statuses(vec!["completed".to_string(), "indexed".to_string()])
        }
        other => RelationalListFilter::Statuses(vec![other.to_string()]),
    }
}

/// Page of documents for one workspace.
///
/// A generic plan cannot treat `$n IS NULL OR tenant_id = $n` as an index
/// condition, so `ORDER BY created_at` walks the global time index and drops
/// every newer row that belongs to someone else. When the caller names a
/// tenant, each arm is an equality the composite index can stop after LIMIT.
/// `id DESC` is the tie-break that index stores; without it the planner still
/// prefers the global `created_at` index. Rows with no tenant stay visible.
///
/// A passed status is required the same way. `status = 'failed'` on the global
/// time index walks every newer indexed row (8.523 ms, 79,950 removed).
/// `idx_documents_workspace_status_created` stops at the page (0.021 ms).
/// `status_count` is 0, 1, or 2.
///
/// A passed `created_at` bound is required too. `$n IS NULL OR created_at <= $n`
/// is a filter on the time index and removes the newer rows (10.637 ms,
/// 79,950 removed). `created_at <= $n` is an index condition on
/// `idx_documents_created_at` or `idx_documents_workspace_created` and stops
/// at the page (0.026 ms). No new index: both already lead with the column
/// the list sorts by.
#[cfg(feature = "postgres")]
fn document_list_statement(
    tenant_scoped: bool,
    status_count: usize,
    bounds: CreatedAtPredicate,
    titles: TitlePredicate,
) -> String {
    let projection = DOCUMENT_LIST_PROJECTION;
    let page = DOCUMENT_PAGE_COLUMNS;
    let order = "ORDER BY created_at DESC, id DESC";
    let start = 2 + usize::from(tenant_scoped) + status_count;
    let created = bounds.fragment(start);
    let (title_sql, limit) = titles.after(bounds, start);
    match (tenant_scoped, status_count) {
        (false, 0) => format!(
            "SELECT {projection}
             FROM public.documents
             WHERE workspace_id = $1{created}{title_sql}
             ORDER BY created_at DESC
             LIMIT {limit}"
        ),
        (true, 0) => format!(
            "SELECT {projection}
             FROM (
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id = $2{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
               UNION ALL
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id IS NULL{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
             ) page
             {order}
             LIMIT {limit}"
        ),
        (false, 1) => format!(
            "SELECT {projection}
             FROM public.documents
             WHERE workspace_id = $1 AND status = $2{created}{title_sql}
             {order}
             LIMIT {limit}"
        ),
        (false, 2) => format!(
            "SELECT {projection}
             FROM (
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND status = $2{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
               UNION ALL
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND status = $3{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
             ) page
             {order}
             LIMIT {limit}"
        ),
        (true, 1) => format!(
            "SELECT {projection}
             FROM (
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id = $2 AND status = $3{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
               UNION ALL
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id IS NULL AND status = $3{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
             ) page
             {order}
             LIMIT {limit}"
        ),
        (true, 2) => format!(
            "SELECT {projection}
             FROM (
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id = $2 AND status = $3{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
               UNION ALL
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id = $2 AND status = $4{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
               UNION ALL
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id IS NULL AND status = $3{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
               UNION ALL
               (
                 SELECT {page}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id IS NULL AND status = $4{created}{title_sql}
                 {order}
                 LIMIT {limit}
               )
             ) page
             {order}
             LIMIT {limit}"
        ),
        _ => document_list_statement(tenant_scoped, 0, bounds, titles),
    }
}

/// Inclusive `created_at` bounds for one document page.
///
/// Both ends empty keeps the unscoped statement. A caller that passes a bound
/// gets that comparison as a required predicate.
#[derive(Debug, Clone, Copy, Default)]
pub struct RelationalCreatedBounds {
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(feature = "postgres")]
#[derive(Clone, Copy, Default)]
struct CreatedAtPredicate {
    from: bool,
    to: bool,
}

#[cfg(feature = "postgres")]
impl CreatedAtPredicate {
    fn from_bounds(bounds: RelationalCreatedBounds) -> Self {
        Self {
            from: bounds.from.is_some(),
            to: bounds.to.is_some(),
        }
    }

    /// ` AND created_at >= $start ...`, or empty when the caller passed no bound.
    fn fragment(self, start: usize) -> String {
        match (self.from, self.to) {
            (false, false) => String::new(),
            (true, false) => format!(" AND created_at >= ${start}"),
            (false, true) => format!(" AND created_at <= ${start}"),
            (true, true) => {
                format!(
                    " AND created_at >= ${start} AND created_at <= ${}",
                    start + 1
                )
            }
        }
    }

    fn limit_placeholder(self, start: usize) -> String {
        let slot = start + usize::from(self.from) + usize::from(self.to);
        format!("${slot}")
    }
}

/// Required title substrings. Zero keeps the statement free of `LIKE`.
///
/// Each needle is its own `lower(title) LIKE`, so a custom plan can use
/// `idx_documents_title_trgm` or stop on the time index. `LIKE ANY($array)`
/// is not an index condition on a generic plan.
#[cfg(feature = "postgres")]
#[derive(Clone, Copy, Default)]
struct TitlePredicate {
    count: usize,
}

#[cfg(feature = "postgres")]
impl TitlePredicate {
    /// SQL fragment and the LIMIT placeholder. Patterns bind after the dates.
    fn after(self, bounds: CreatedAtPredicate, start: usize) -> (String, String) {
        let pattern_start = start + usize::from(bounds.from) + usize::from(bounds.to);
        if self.count == 0 {
            return (String::new(), bounds.limit_placeholder(start));
        }
        let likes: Vec<String> = (0..self.count)
            .map(|offset| {
                format!(
                    "lower(title) LIKE '%' || lower(${}) || '%' ESCAPE '\\'",
                    pattern_start + offset
                )
            })
            .collect();
        let sql = format!(" AND ({})", likes.join(" OR "));
        let limit = format!("${}", pattern_start + self.count);
        (sql, limit)
    }
}

/// Ids for a query document filter.
///
/// The filter used to load every workspace id (37.104 ms, 80,000 rows, 3,094
/// buffers) and drop dates in memory. A required `created_at` bound is an
/// index condition. The same statement returns the 50 oldest rows in 0.056 ms
/// and 18 buffers. `$1` is the workspace uuid and `$2` is its text form, so
/// dates start at `$3`.
#[cfg(feature = "postgres")]
fn document_filter_statement(
    bounds: CreatedAtPredicate,
    titles: TitlePredicate,
    explicit_ids: bool,
) -> String {
    let start = 3;
    let created = bounds.fragment(start);
    let (title_sql, _) = titles.after(bounds, start);
    let date_slots = usize::from(bounds.from) + usize::from(bounds.to);
    let mut parts = Vec::new();
    if titles.count > 0 {
        parts.push(filter_scope_union(&format!("{created}{title_sql}")));
    }
    if explicit_ids {
        let slot = start + date_slots + titles.count;
        parts.push(filter_scope_union(&format!(
            "{created} AND id = ANY(${slot}::uuid[])"
        )));
    }
    if parts.is_empty() {
        parts.push(filter_scope_union(&created));
    }
    parts.join("\nUNION\n")
}

#[cfg(feature = "postgres")]
fn filter_scope_union(extra: &str) -> String {
    let projection = "id::text AS id, COALESCE(title, '') AS title, created_at, \
         COALESCE(tenant_id::text, metadata->>'tenant_id') AS tenant_id, \
         CASE WHEN workspace_id = $1 THEN workspace_id::text ELSE metadata->>'workspace_id' END AS workspace_id";
    format!(
        "SELECT {projection}
         FROM public.documents
         WHERE workspace_id = $1{extra}
         UNION
         SELECT {projection}
         FROM public.documents
         WHERE metadata->>'workspace_id' = $2
           AND workspace_id IS DISTINCT FROM $1{extra}"
    )
}

/// One document the query filter still has to tenant-check.
#[cfg(feature = "postgres")]
#[derive(Debug, Clone)]
pub struct RelationalFilterCandidate {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
}

/// Documents matching a query filter's date, title pattern, or explicit ids.
///
/// The caller passes at least one of those. The open membership statement stays
/// on the path that asks for every id.
#[cfg(feature = "postgres")]
pub async fn filter_relational_document_candidates(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    bounds: RelationalCreatedBounds,
    title_patterns: &[String],
    explicit_ids: &[Uuid],
) -> Result<Vec<RelationalFilterCandidate>, crate::error::ApiError> {
    use crate::error::ApiError;
    use sqlx::Row;

    if title_patterns.len() > 16 {
        return Err(ApiError::BadRequest(
            "document_pattern accepts at most 16 titles".into(),
        ));
    }
    let timeout_ms = edgequake_storage::interactive_statement_timeout_ms();
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to acquire PG connection: {e}")))?;
    let mut timed = edgequake_storage::LocalTimeoutTx::begin(&mut conn, timeout_ms)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to begin document-filter TX: {e}")))?;
    sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
        .execute(timed.as_mut())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to set document-filter plan mode: {e}")))?;

    let created = CreatedAtPredicate::from_bounds(bounds);
    let titles = TitlePredicate {
        count: title_patterns.len(),
    };
    let sql = document_filter_statement(created, titles, !explicit_ids.is_empty());
    let mut query = sqlx::query(&sql)
        .bind(workspace_id)
        .bind(workspace_id.to_string());
    if let Some(from) = bounds.from {
        query = query.bind(from);
    }
    if let Some(to) = bounds.to {
        query = query.bind(to);
    }
    for pattern in title_patterns {
        query = query.bind(escape_title_like_needle(pattern));
    }
    if !explicit_ids.is_empty() {
        query = query.bind(explicit_ids);
    }
    let rows = query
        .fetch_all(timed.as_mut())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to filter relational documents: {e}")))?;
    timed
        .commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to commit document-filter TX: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            RelationalFilterCandidate {
                id: row.get("id"),
                title: row.get("title"),
                created_at: created_at.to_rfc3339(),
                tenant_id: row.get("tenant_id"),
                workspace_id: row.get("workspace_id"),
            }
        })
        .collect())
}

/// `pending` and `processing` also match `current_stage`.
///
/// One equality per value, then `UNION ALL`, so each arm stops at LIMIT.
/// `current_stage = ANY($n)` is not an index condition on a generic plan
/// (10.739 ms, 79,950 newer rows removed). A stage arm skips rows whose
/// `status` is already a status arm of the same chip, so a row comes back once.
#[cfg(feature = "postgres")]
fn chip_list_statement(
    tenant_scoped: bool,
    arms: &[ChipArm],
    bounds: CreatedAtPredicate,
    titles: TitlePredicate,
) -> String {
    let projection = DOCUMENT_LIST_PROJECTION;
    let page = DOCUMENT_PAGE_COLUMNS;
    let order = "ORDER BY created_at DESC, id DESC";
    let start = 2 + usize::from(tenant_scoped);
    let created = bounds.fragment(start);
    let (title_sql, limit) = titles.after(bounds, start);
    let exclusions = chip_status_exclusions(arms);
    let scopes: Vec<&str> = if tenant_scoped {
        vec![
            "workspace_id = $1 AND tenant_id = $2",
            "workspace_id = $1 AND tenant_id IS NULL",
        ]
    } else {
        vec!["workspace_id = $1"]
    };
    let mut unions = Vec::new();
    for scope in scopes {
        for arm in arms {
            let filter = chip_arm_filter(arm, &exclusions);
            unions.push(format!(
                "(
                   SELECT {page}
                   FROM public.documents
                   WHERE {scope} AND {filter}{created}{title_sql}
                   {order}
                   LIMIT {limit}
                 )"
            ));
        }
    }
    let body = unions.join("\n               UNION ALL\n               ");
    format!(
        "SELECT {projection}
         FROM (
               {body}
         ) page
         {order}
         LIMIT {limit}"
    )
}

#[cfg(feature = "postgres")]
fn chip_status_exclusions(arms: &[ChipArm]) -> String {
    let mut sql = String::new();
    for arm in arms {
        if let ChipArm::Status(status) = arm {
            sql.push_str(" AND status IS DISTINCT FROM ");
            sql.push_str(&sql_ident_literal(status));
        }
    }
    sql
}

#[cfg(feature = "postgres")]
fn chip_arm_filter(arm: &ChipArm, exclusions: &str) -> String {
    match arm {
        ChipArm::Status(status) => format!("status = {}", sql_ident_literal(status)),
        ChipArm::Stage(stage) => {
            format!("current_stage = {}{exclusions}", sql_ident_literal(stage))
        }
    }
}

/// Chip literals are a fixed allowlist. They are not the request string.
#[cfg(feature = "postgres")]
fn sql_ident_literal(value: &str) -> String {
    debug_assert!(
        value.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
        "chip literal must be a lowercase identifier"
    );
    format!("'{value}'")
}

/// Status chips for the same scope as the document page.
///
/// The count must read every matching row. It must not read other tenants.
/// `tenant_id = $2 OR tenant_id IS NULL` is a bitmap of two index ranges once
/// the plan sees the real tenant. A generic plan still guesses half the table
/// and seq-scans, so this statement is planned with `force_custom_plan`.
#[cfg(feature = "postgres")]
fn document_status_statement(tenant_scoped: bool) -> String {
    let predicate = if tenant_scoped {
        "WHERE workspace_id = $1 AND (tenant_id = $2 OR tenant_id IS NULL)"
    } else {
        "WHERE workspace_id = $1"
    };
    format!(
        "SELECT bucket, COUNT(*)::bigint AS n
         FROM (
            SELECT CASE
                WHEN lower(COALESCE(status, '')) IN ('pending', 'queued')
                  OR lower(COALESCE(metadata->>'current_stage', '')) = 'queued'
                    THEN 'pending'
                WHEN lower(COALESCE(status, '')) IN ('processing', 'deleting')
                  OR lower(COALESCE(metadata->>'current_stage', '')) IN (
                        'converting','preprocessing','chunking','extracting','gleaning',
                        'merging','summarizing','embedding','storing','projecting',
                        'indexing','deleting'
                     )
                    THEN 'processing'
                WHEN lower(COALESCE(status, '')) IN ('completed', 'indexed')
                    THEN 'completed'
                WHEN lower(COALESCE(status, '')) = 'partial_failure'
                    THEN 'partial_failure'
                WHEN lower(COALESCE(status, '')) = 'failed'
                    THEN 'failed'
                WHEN lower(COALESCE(status, '')) = 'cancelled'
                    THEN 'cancelled'
                ELSE 'unknown'
            END AS bucket
            FROM public.documents
            {predicate}
         ) s
         GROUP BY bucket"
    )
}

/// Workspace wipe. Each arm is an equality so a generic plan can use an index
/// instead of reading every other tenant in the table.
#[cfg(feature = "postgres")]
fn document_workspace_delete_statement(tenant_scoped: bool) -> &'static str {
    if tenant_scoped {
        "WITH doomed AS (
            SELECT id FROM public.documents
            WHERE workspace_id = $1 AND tenant_id = $2
            UNION
            SELECT id FROM public.documents
            WHERE workspace_id = $1 AND tenant_id IS NULL
            UNION
            SELECT id FROM public.documents
            WHERE workspace_id IS NULL
              AND metadata->>'workspace_id' = $3
              AND tenant_id = $2
            UNION
            SELECT id FROM public.documents
            WHERE workspace_id IS NULL
              AND metadata->>'workspace_id' = $3
              AND tenant_id IS NULL
        )
        DELETE FROM public.documents d
        USING doomed
        WHERE d.id = doomed.id"
    } else {
        "WITH doomed AS (
            SELECT id FROM public.documents
            WHERE workspace_id = $1
            UNION
            SELECT id FROM public.documents
            WHERE workspace_id IS NULL
              AND metadata->>'workspace_id' = $2
        )
        DELETE FROM public.documents d
        USING doomed
        WHERE d.id = doomed.id"
    }
}

/// Bounded relational list with truncation flag (GH-400).
#[cfg(feature = "postgres")]
pub async fn list_relational_document_summaries_limited(
    pool: crate::services::OptionalPgPool<'_>,
    tenant_ctx: &TenantContext,
    max_entries: usize,
    status: Option<&str>,
    bounds: RelationalCreatedBounds,
    title_patterns: &[String],
) -> Result<RelationalDocumentList, crate::error::ApiError> {
    use crate::error::ApiError;
    use sqlx::Row;

    if title_patterns.len() > 16 {
        return Err(ApiError::BadRequest(
            "document_pattern accepts at most 16 titles".into(),
        ));
    }
    let pool = pool.ok_or_else(|| ApiError::Internal("PostgreSQL pool not available".into()))?;
    let max_entries = max_entries.max(1);

    let workspace_id = tenant_ctx
        .workspace_id
        .as_ref()
        .and_then(|w| Uuid::parse_str(w).ok())
        .ok_or_else(|| ApiError::BadRequest("workspace_id required".into()))?;

    let tenant_uuid = tenant_ctx
        .tenant_id
        .as_ref()
        .and_then(|t| Uuid::parse_str(t).ok());

    let timeout_ms = edgequake_storage::interactive_statement_timeout_ms();
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to acquire PG connection: {e}")))?;
    let mut timed = edgequake_storage::LocalTimeoutTx::begin(&mut conn, timeout_ms)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to begin interactive list TX: {e}")))?;

    // A generic plan of `lower(title) LIKE` always starts at the trigram index
    // and reads every title when the letter matches the table (46.237 ms).
    // The custom plan keeps a common letter on the time index.
    if !title_patterns.is_empty() {
        sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
            .execute(timed.as_mut())
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to set list plan mode: {e}")))?;
    }

    // Fetch one extra row to detect truncation without a second COUNT.
    let fetch_limit = (max_entries as i64).saturating_add(1);
    let filter = relational_list_filter(status);
    let created = CreatedAtPredicate::from_bounds(bounds);
    let titles = TitlePredicate {
        count: title_patterns.len(),
    };
    let (sql, status_binds) = match filter {
        RelationalListFilter::Open => (
            document_list_statement(tenant_uuid.is_some(), 0, created, titles),
            Vec::new(),
        ),
        RelationalListFilter::Statuses(values) => {
            let sql = document_list_statement(tenant_uuid.is_some(), values.len(), created, titles);
            (sql, values)
        }
        RelationalListFilter::Chip(arms) => (
            chip_list_statement(tenant_uuid.is_some(), arms, created, titles),
            Vec::new(),
        ),
    };
    let mut query = sqlx::query(&sql).bind(workspace_id);
    if let Some(tenant_id) = tenant_uuid {
        query = query.bind(tenant_id);
    }
    for status_value in &status_binds {
        query = query.bind(status_value);
    }
    if let Some(from) = bounds.from {
        query = query.bind(from);
    }
    if let Some(to) = bounds.to {
        query = query.bind(to);
    }
    for pattern in title_patterns {
        query = query.bind(escape_title_like_needle(pattern));
    }
    let rows = query
        .bind(fetch_limit)
        .fetch_all(timed.as_mut())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list relational documents: {e}")))?;

    timed
        .commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to commit interactive list TX: {e}")))?;

    let truncated = rows.len() > max_entries;
    let documents = rows
        .into_iter()
        .take(max_entries)
        .map(|row| {
            let title: String = row.get("title");
            let status: String = row.get("status");
            let chunk_count: Option<i32> = row.get("chunk_count");
            let entity_count: Option<i32> = row.get("entity_count");
            let content_length: Option<i64> = row.get("content_length");
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");
            let cost_usd: Option<f64> = row.get("cost_usd");
            let input_tokens: Option<i64> = row.get("input_tokens");
            let output_tokens: Option<i64> = row.get("output_tokens");
            let total_tokens: Option<i64> = row.get("total_tokens");
            let track_id: Option<String> = row.get("track_id");
            let current_stage: Option<String> = row.get("current_stage");
            let content_preview: Option<String> = row.get("content_preview");
            let content_summary = content_preview.filter(|s| !s.is_empty());

            DocumentSummary {
                id: row.get("id"),
                title: Some(title.clone()),
                file_name: Some(title),
                content_summary,
                content_length: content_length
                    .filter(|&n| n >= 0)
                    .and_then(|n| usize::try_from(n).ok()),
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
                current_stage,
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
                display_status: None,
                ui_phase: None,
                progress_counts: None,
                run_progress: None,
                queue_position: None,
                eta_seconds: None,
                eta_basis: None,
                query_ready: None,
                cancelled_from_stage: None,
                extraction: Default::default(),
            }
        })
        .collect();

    Ok(RelationalDocumentList {
        documents,
        truncated,
    })
}

/// One document whose title contains the scope-picker query.
#[cfg(feature = "postgres")]
#[derive(Debug, Clone)]
pub struct RelationalTitleHit {
    pub id: String,
    pub title: String,
    pub status: String,
    pub created_at: Option<String>,
}

/// Escape `\`, `%`, and `_` so a title query is a literal substring.
#[cfg(feature = "postgres")]
fn escape_title_like_needle(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    for ch in query.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Title search for the scope picker.
///
/// The picker used to read the newest 2,001 documents and drop titles in
/// memory. On 80,000 documents the 50 oldest `uniquezephyr` titles are not
/// in that page (0 hits). Requiring `lower(title) LIKE` on
/// `idx_documents_created_at` still removes those 79,950 newer rows
/// (38.139 ms, 2,557 buffers).
///
/// `idx_documents_title_trgm` is a GIN on `lower(title)`. A generic plan
/// always starts there and, for a one-letter query that matches every row,
/// reads all 80,000 titles (46.237 ms). This statement is planned with
/// `force_custom_plan`: a common letter stops on the time index (0.104 ms)
/// and a rare title stops on the trigram index.
#[cfg(feature = "postgres")]
fn document_title_search_statement(tenant_scoped: bool, ready_only: bool) -> String {
    let status = if ready_only {
        " AND status IN ('completed', 'indexed')"
    } else {
        ""
    };
    let order = "ORDER BY created_at DESC, id DESC";
    let cols = "id::text AS id, title, status, created_at";
    if tenant_scoped {
        let like = "lower(title) LIKE '%' || lower($3) || '%' ESCAPE '\\'";
        format!(
            "SELECT id, title, status, created_at
             FROM (
               (
                 SELECT {cols}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id = $2 AND {like}{status}
                 {order}
                 LIMIT $4
               )
               UNION ALL
               (
                 SELECT {cols}
                 FROM public.documents
                 WHERE workspace_id = $1 AND tenant_id IS NULL AND {like}{status}
                 {order}
                 LIMIT $4
               )
             ) page
             {order}
             LIMIT $4"
        )
    } else {
        let like = "lower(title) LIKE '%' || lower($2) || '%' ESCAPE '\\'";
        format!(
            "SELECT {cols}
             FROM public.documents
             WHERE workspace_id = $1 AND {like}{status}
             {order}
             LIMIT $3"
        )
    }
}

/// Documents whose title contains `query`, newest first.
///
/// `ready_only` keeps the scope picker's default `completed` bucket
/// (`completed` and `indexed`). `status=all` passes `ready_only = false`.
#[cfg(feature = "postgres")]
pub async fn search_relational_document_titles(
    pool: crate::services::OptionalPgPool<'_>,
    tenant_ctx: &TenantContext,
    query: &str,
    ready_only: bool,
    limit: usize,
) -> Result<Vec<RelationalTitleHit>, crate::error::ApiError> {
    use crate::error::ApiError;
    use sqlx::Row;

    let needle = escape_title_like_needle(query.trim());
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let pool = pool.ok_or_else(|| ApiError::Internal("PostgreSQL pool not available".into()))?;
    let limit = (limit.max(1) as i64).min(51);

    let workspace_id = tenant_ctx
        .workspace_id
        .as_ref()
        .and_then(|w| Uuid::parse_str(w).ok())
        .ok_or_else(|| ApiError::BadRequest("workspace_id required".into()))?;
    let tenant_uuid = tenant_ctx
        .tenant_id
        .as_ref()
        .and_then(|t| Uuid::parse_str(t).ok());

    let timeout_ms = edgequake_storage::interactive_statement_timeout_ms();
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to acquire PG connection: {e}")))?;
    let mut timed = edgequake_storage::LocalTimeoutTx::begin(&mut conn, timeout_ms)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to begin title-search TX: {e}")))?;
    sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
        .execute(timed.as_mut())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to set title-search plan mode: {e}")))?;

    let sql = document_title_search_statement(tenant_uuid.is_some(), ready_only);
    let mut q = sqlx::query(&sql).bind(workspace_id);
    if let Some(tenant_id) = tenant_uuid {
        q = q.bind(tenant_id);
    }
    let rows = q
        .bind(&needle)
        .bind(limit)
        .fetch_all(timed.as_mut())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to search document titles: {e}")))?;
    timed
        .commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to commit title-search TX: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
            RelationalTitleHit {
                id: row.get("id"),
                title: row.get("title"),
                status: row.get("status"),
                created_at: Some(created_at.to_rfc3339()),
            }
        })
        .collect())
}

/// Workspace-wide status bucket counts from `documents` (GH-400).
///
/// Cheap aggregate (no row body) so interactive list can report global chips
/// even when the detail scan is capped at `MAX_LIST_METADATA_ENTRIES`.
#[cfg(feature = "postgres")]
pub async fn count_relational_document_statuses(
    pool: crate::services::OptionalPgPool<'_>,
    tenant_ctx: &TenantContext,
) -> Result<crate::handlers::documents_types::StatusCounts, crate::error::ApiError> {
    use crate::error::ApiError;
    use crate::handlers::documents_types::StatusCounts;
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

    let timeout_ms = edgequake_storage::interactive_statement_timeout_ms();
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to acquire PG connection: {e}")))?;
    let mut timed = edgequake_storage::LocalTimeoutTx::begin(&mut conn, timeout_ms)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to begin status-count TX: {e}")))?;

    // A generic plan prices `tenant_id = $n` as half the table and seq-scans.
    // Replan with the bound tenant so a small tenant reads only its rows.
    sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
        .execute(timed.as_mut())
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to set status-count plan mode: {e}")))?;

    // Bucket using status + metadata current_stage (parity with list.rs chips).
    let rows = if let Some(tenant_id) = tenant_uuid {
        sqlx::query(&document_status_statement(true))
            .bind(workspace_id)
            .bind(tenant_id)
            .fetch_all(timed.as_mut())
            .await
    } else {
        sqlx::query(&document_status_statement(false))
            .bind(workspace_id)
            .fetch_all(timed.as_mut())
            .await
    }
    .map_err(|e| {
        ApiError::Internal(format!("Failed to count relational document statuses: {e}"))
    })?;

    timed
        .commit()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to commit status-count TX: {e}")))?;

    let mut counts = StatusCounts {
        pending: 0,
        processing: 0,
        completed: 0,
        partial_failure: 0,
        failed: 0,
        cancelled: 0,
        unknown: 0,
    };
    for row in rows {
        let bucket: String = row.get("bucket");
        let n: i64 = row.get("n");
        let n = n.max(0) as usize;
        match bucket.as_str() {
            "pending" => counts.pending = n,
            "processing" => counts.processing = n,
            "completed" => counts.completed = n,
            "partial_failure" => counts.partial_failure = n,
            "failed" => counts.failed = n,
            "cancelled" => counts.cancelled = n,
            _ => counts.unknown = n,
        }
    }
    Ok(counts)
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
    pool: crate::services::OptionalPgPool<'_>,
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

    let row = if let Some(tenant_id) = tenant_uuid {
        sqlx::query(
            r#"
            SELECT workspace_id::text AS workspace_id, status, track_id
            FROM public.documents
            WHERE id = $1
              AND workspace_id = $2
              AND (tenant_id IS NULL OR tenant_id = $3)
            "#,
        )
        .bind(doc_uuid)
        .bind(workspace_id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
    } else {
        sqlx::query(
            r#"
            SELECT workspace_id::text AS workspace_id, status, track_id
            FROM public.documents
            WHERE id = $1 AND workspace_id = $2
            "#,
        )
        .bind(doc_uuid)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await
    }
    .map_err(|e| ApiError::Internal(format!("Failed to lookup relational document: {e}")))?;

    Ok(row.map(|r| RelationalDocumentScope {
        workspace_id: r.get("workspace_id"),
        status: r.get("status"),
        track_id: r.get("track_id"),
    }))
}

/// Point lookup of `tenant_id` / `workspace_id` for the detail authority fence.
///
/// GH-400: runs under [`LocalTimeoutTx`] so a hung PG session cannot hold the
/// interactive detail path past the statement budget. Point lookup only —
/// success path is identical to a bare `fetch_optional`.
#[cfg(feature = "postgres")]
pub async fn lookup_document_tenant_workspace(
    pool: &sqlx::PgPool,
    document_id: Uuid,
) -> Result<Option<(Option<Uuid>, Option<Uuid>)>, crate::error::ApiError> {
    use crate::error::ApiError;

    let timeout_ms = edgequake_storage::interactive_statement_timeout_ms();
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ApiError::Internal(format!("document scope lookup acquire: {e}")))?;
    let mut timed = edgequake_storage::LocalTimeoutTx::begin(&mut conn, timeout_ms)
        .await
        .map_err(|e| ApiError::Internal(format!("document scope lookup begin: {e}")))?;

    let row = sqlx::query_as::<_, (Option<Uuid>, Option<Uuid>)>(
        "SELECT tenant_id, workspace_id FROM public.documents WHERE id = $1",
    )
    .bind(document_id)
    .fetch_optional(timed.as_mut())
    .await
    .map_err(|e| ApiError::Internal(format!("document scope lookup: {e}")))?;

    timed
        .commit()
        .await
        .map_err(|e| ApiError::Internal(format!("document scope lookup commit: {e}")))?;

    Ok(row)
}

#[cfg(not(feature = "postgres"))]
pub async fn relational_document_scope<P>(
    _pool: Option<&P>,
    _document_id: &str,
    _tenant_ctx: &TenantContext,
) -> Result<Option<RelationalDocumentScope>, crate::error::ApiError> {
    Ok(None)
}

/// Delete a single relational `documents` row under tenant/workspace scope.
///
/// First principles: list = merge(KV, relational). A successful document delete
/// must remove the SQL row or the UI re-injects a "ghost" on refresh.
/// Returns rows affected (0 = already absent — success for idempotent purge).
#[cfg(feature = "postgres")]
pub async fn delete_relational_document(
    pool: crate::services::OptionalPgPool<'_>,
    document_id: &str,
    tenant_ctx: &TenantContext,
) -> Result<u64, crate::error::ApiError> {
    use crate::error::ApiError;

    let Some(pool) = pool else {
        return Ok(0);
    };

    let Ok(doc_uuid) = Uuid::parse_str(document_id) else {
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

    let result = if let Some(tenant_id) = tenant_uuid {
        sqlx::query(
            r#"
            DELETE FROM public.documents
            WHERE id = $1
              AND workspace_id = $2
              AND (tenant_id IS NULL OR tenant_id = $3)
            "#,
        )
        .bind(doc_uuid)
        .bind(workspace_id)
        .bind(tenant_id)
        .execute(pool)
        .await
    } else {
        sqlx::query(
            r#"
            DELETE FROM public.documents
            WHERE id = $1 AND workspace_id = $2
            "#,
        )
        .bind(doc_uuid)
        .bind(workspace_id)
        .execute(pool)
        .await
    }
    .map_err(|e| {
        ApiError::Internal(format!(
            "Failed to delete relational document {document_id}: {e}"
        ))
    })?;

    Ok(result.rows_affected())
}

#[cfg(not(feature = "postgres"))]
pub async fn delete_relational_document<P>(
    _pool: Option<&P>,
    _document_id: &str,
    _tenant_ctx: &TenantContext,
) -> Result<u64, crate::error::ApiError> {
    Ok(0)
}

/// Delete all relational `documents` rows for the request workspace (bulk delete SSOT).
#[cfg(feature = "postgres")]
pub async fn delete_relational_documents_for_workspace(
    pool: crate::services::OptionalPgPool<'_>,
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

    // Wave B3/C + SPEC-091 IW1 (GAP-091-24): shell-written documents may carry
    // the workspace in metadata JSONB while the FK-guarded `workspace_id`
    // column stays NULL. Each arm is an equality so the planner can use the
    // workspace btree or the expression index on `metadata->>'workspace_id'`
    // (migration 128) instead of reading every other tenant.
    let workspace_id_str = tenant_ctx.workspace_id.clone();
    let result = if let Some(tenant_id) = tenant_uuid {
        sqlx::query(document_workspace_delete_statement(true))
            .bind(workspace_id)
            .bind(tenant_id)
            .bind(workspace_id_str)
            .execute(pool)
            .await
    } else {
        sqlx::query(document_workspace_delete_statement(false))
            .bind(workspace_id)
            .bind(workspace_id_str)
            .execute(pool)
            .await
    }
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

/// True when KV indicates the document is still moving through the pipeline
/// or a lifecycle delete (SPEC-098 LAW-098-9).
///
/// Includes `current_stage=queued` (reprocess stage reset) which is not always
/// mirrored into `status` helpers as a standalone status string.
fn kv_summary_is_inflight(doc: &DocumentSummary) -> bool {
    use crate::document_metadata::is_lifecycle_inflight_status;

    if doc
        .status
        .as_deref()
        .is_some_and(is_lifecycle_inflight_status)
    {
        return true;
    }
    doc.current_stage.as_deref().is_some_and(|stage| {
        is_lifecycle_inflight_status(stage) || stage.eq_ignore_ascii_case("queued")
    })
}

/// When relational cancel/fail wins over stale in-flight KV, clear embedding-band
/// progress so Active Runs cannot show "Embedding 99%" for a terminal document.
fn project_terminal_failure_presentation(doc: &mut DocumentSummary, rel_status: &str) {
    let stage = if rel_status.eq_ignore_ascii_case("cancelled") {
        "cancelled"
    } else if rel_status.eq_ignore_ascii_case("partial_failure") {
        "partial_failure"
    } else if rel_status.eq_ignore_ascii_case("delete_failed") {
        "delete_failed"
    } else {
        "failed"
    };
    doc.current_stage = Some(stage.to_string());
    doc.stage_progress = Some(0.0);
    doc.stage_message = Some(match stage {
        "cancelled" => "Processing cancelled".into(),
        "partial_failure" => "Processing completed with issues".into(),
        "delete_failed" => "Document delete failed".into(),
        _ => "Processing failed".into(),
    });
}

/// True when relational title should win over KV for the same document id.
///
/// Placeholder relational titles (`None`, empty, `"Untitled"`) must not hide a
/// real KV `title` / `file_name` — admission historically left the schema
/// DEFAULT while KV already held the upload filename.
fn should_overlay_relational_title(rel_title: Option<&str>, kv: &DocumentSummary) -> bool {
    let Some(rel) = rel_title.map(str::trim).filter(|t| !t.is_empty()) else {
        return false;
    };
    let placeholder = rel.eq_ignore_ascii_case("Untitled");
    if !placeholder {
        return true;
    }
    let kv_has_real = kv
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("Untitled"))
        .is_some()
        || kv
            .file_name
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("Untitled"))
            .is_some();
    !kv_has_real
}

/// Merge KV-derived documents with relational rows (relational fills gaps).
///
/// Status rules:
/// - SPEC-054 reprocess honesty: when KV is in-flight, do **not** let a stale
///   relational **success** (`completed`/`indexed`/`partial_success`) overwrite
///   it. Reprocess accept updates KV first; relational lags until the worker
///   mirrors status.
/// - Cancel/fail terminals are intentional: relational `cancelled`/`failed`/
///   `partial_failure` **must** win over stale in-flight KV (zombie Active Runs
///   after cancel that only touched `documents.status`).
pub fn merge_document_summaries(
    mut kv_documents: Vec<DocumentSummary>,
    relational_documents: Vec<DocumentSummary>,
) -> Vec<DocumentSummary> {
    use crate::document_metadata::{
        is_terminal_document_status, is_terminal_failure_status, is_terminal_success_status,
    };
    use std::collections::HashMap;

    let mut by_id: HashMap<String, usize> = HashMap::new();
    for (idx, doc) in kv_documents.iter().enumerate() {
        by_id.insert(doc.id.clone(), idx);
    }

    for rel in relational_documents {
        if let Some(&idx) = by_id.get(&rel.id) {
            // SPEC-045: relational title/counts are durable; KV may hold swapped blobs.
            // Do not overlay schema-default / empty placeholder titles over a real
            // KV filename (markdown admission used to leave DEFAULT 'Untitled').
            let kv = &mut kv_documents[idx];
            if should_overlay_relational_title(rel.title.as_deref(), kv) {
                kv.title = rel.title.clone();
                kv.file_name = rel.file_name.clone();
            }
            if let Some(rel_status) = rel.status.as_deref() {
                let kv_was_inflight = kv_summary_is_inflight(kv);
                // SPEC-054: KV inflight beats stale relational *success* only.
                let keep_kv_inflight = kv_was_inflight && is_terminal_success_status(rel_status);
                // Zombie guard: KV terminal (cancelled/failed/completed) must not
                // be overwritten by a stale relational non-terminal column that
                // shell upserts historically left at `processing`.
                let keep_kv_terminal = kv
                    .status
                    .as_deref()
                    .is_some_and(is_terminal_document_status)
                    && !is_terminal_document_status(rel_status);
                // SPEC-098 LAW-098-11: KV delete_failed beats shell-collapsed SQL failed.
                let keep_kv_delete_failed = kv
                    .status
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case("delete_failed"))
                    && rel_status.eq_ignore_ascii_case("failed");
                if !keep_kv_inflight && !keep_kv_terminal && !keep_kv_delete_failed {
                    kv.status = rel.status.clone();
                    // Project presentation so Active Runs cannot keep embedding 99%.
                    if kv_was_inflight && is_terminal_failure_status(rel_status) {
                        project_terminal_failure_presentation(kv, rel_status);
                    }
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
/// counts, so we fall back to `node_counts_by_source_prefixes_capped`
/// (SPEC-054 L1-a + SPEC-089 / GH-336) for documents whose current
/// `entity_count` is 0.
///
/// LAW-H1 / LAW-H5: callers must pass the **paginated page** (or a similarly
/// bounded slice), never the full corpus. Write-path P-A1 remains primary.
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

/// Probe series upper bound for P-A3 (mirrors storage `source_count_probe_limit`).
#[inline]
pub(crate) fn reconcile_probe_limit_from_chunk_counts(
    chunk_counts: impl Iterator<Item = usize>,
) -> usize {
    const MAX: usize = 256;
    let max_c = chunk_counts.max().unwrap_or(0);
    if max_c == 0 {
        MAX
    } else {
        max_c.clamp(1, MAX)
    }
}

/// Best-effort: AGE failures/timeouts are swallowed (counts stay as-is) so the
/// interactive list never fails with `read_path_busy` due to a graph probe.
///
/// SPEC-089 / LAW-H2: Rust timeout is a client budget; storage also applies
/// `SET LOCAL statement_timeout` so cancelled work cannot hold pool slots.
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

    let probe_limit = reconcile_probe_limit_from_chunk_counts(
        candidates.iter().map(|(i, _)| documents[*i].chunk_count),
    );

    // Hard cap well under EDGEQUAKE_DOCUMENTS_READ_TIMEOUT_MS so list/detail
    // keep serving KV counts when AGE is slow (142k+ node graphs).
    // Storage `SOURCE_COUNT_STATEMENT_TIMEOUT_MS` (300ms) is the server kill.
    const AGE_RECONCILE_TIMEOUT: Duration = Duration::from_millis(400);
    let counts = match tokio::time::timeout(
        AGE_RECONCILE_TIMEOUT,
        storage
            .graph_storage
            .node_counts_by_source_prefixes_capped(&prefixes, probe_limit),
    )
    .await
    {
        Ok(Ok(map)) => map,
        Ok(Err(e)) => {
            // SPEC-107 R2: tag dataop + PG kill budget for prod greps (57014).
            #[cfg(feature = "postgres")]
            let pg_timeout_ms = edgequake_storage::SOURCE_COUNT_STATEMENT_TIMEOUT_MS;
            #[cfg(not(feature = "postgres"))]
            let pg_timeout_ms = 300u32;
            tracing::warn!(
                error = %e,
                candidate_count = candidates.len(),
                probe_limit,
                op = "DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES",
                pg_timeout_ms,
                "P-A3: batched AGE entity_count fallback failed (non-fatal) — leaving counts as-is (often SQLSTATE 57014 under load)"
            );
            return;
        }
        Err(_) => {
            #[cfg(feature = "postgres")]
            let pg_timeout_ms = edgequake_storage::SOURCE_COUNT_STATEMENT_TIMEOUT_MS;
            #[cfg(not(feature = "postgres"))]
            let pg_timeout_ms = 300u32;
            tracing::warn!(
                candidate_count = candidates.len(),
                probe_limit,
                timeout_ms = AGE_RECONCILE_TIMEOUT.as_millis() as u64,
                op = "DATA-AGE-GRAPH-NODE-COUNTS-BY-SOURCE-PREFIXES",
                pg_timeout_ms,
                "P-A3: AGE entity_count reconcile timed out — serving KV counts (app 400ms / PG SET LOCAL)"
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

    #[cfg(feature = "postgres")]
    #[test]
    fn status_page_requires_the_status_column() {
        assert!(matches!(
            relational_list_filter(None),
            RelationalListFilter::Open
        ));
        assert!(matches!(
            relational_list_filter(Some("all")),
            RelationalListFilter::Open
        ));
        match relational_list_filter(Some("unknown")) {
            RelationalListFilter::Chip(arms) => assert!(std::ptr::eq(arms, UNKNOWN_ARMS)),
            other => panic!("unknown must be the unknown chip, got {other:?}"),
        }
        match relational_list_filter(Some("pending")) {
            RelationalListFilter::Chip(arms) => assert!(std::ptr::eq(arms, PENDING_ARMS)),
            other => panic!("pending must be the pending chip, got {other:?}"),
        }
        match relational_list_filter(Some("processing")) {
            RelationalListFilter::Chip(arms) => assert!(std::ptr::eq(arms, PROCESSING_ARMS)),
            other => panic!("processing must be the processing chip, got {other:?}"),
        }
        match relational_list_filter(Some("failed")) {
            RelationalListFilter::Statuses(values) => {
                assert_eq!(values, vec!["failed".to_string()]);
            }
            other => panic!("failed must be one status equality, got {other:?}"),
        }
        match relational_list_filter(Some("completed")) {
            RelationalListFilter::Statuses(values) => {
                assert_eq!(values, vec!["completed".to_string(), "indexed".to_string()]);
            }
            other => panic!("completed must be two status equalities, got {other:?}"),
        }

        let open = document_list_statement(
            false,
            0,
            CreatedAtPredicate::default(),
            TitlePredicate::default(),
        );
        assert!(!open.contains("status ="));
        assert!(open.contains("ORDER BY created_at DESC"));
        assert!(open.contains("LIMIT $2"));
        assert!(!open.contains("created_at <="));
        assert!(!open.contains("created_at >="));

        let failed = document_list_statement(
            false,
            1,
            CreatedAtPredicate::default(),
            TitlePredicate::default(),
        );
        assert!(failed.contains("workspace_id = $1 AND status = $2"));
        assert!(failed.contains("ORDER BY created_at DESC, id DESC"));
        assert!(failed.contains("LIMIT $3"));
        assert!(!failed.contains("IS NULL OR"));

        let completed = document_list_statement(
            false,
            2,
            CreatedAtPredicate::default(),
            TitlePredicate::default(),
        );
        assert!(completed.contains("status = $2"));
        assert!(completed.contains("status = $3"));
        assert!(completed.contains("UNION ALL"));

        let tenant = document_list_statement(
            true,
            1,
            CreatedAtPredicate::default(),
            TitlePredicate::default(),
        );
        assert!(tenant.contains("tenant_id = $2 AND status = $3"));
        assert!(tenant.contains("tenant_id IS NULL AND status = $3"));
        assert!(!tenant.contains("$2::uuid IS NULL"));

        let pending = chip_list_statement(
            false,
            PENDING_ARMS,
            CreatedAtPredicate::default(),
            TitlePredicate::default(),
        );
        assert!(pending.contains("status = 'pending'"));
        assert!(pending.contains("status = 'queued'"));
        assert!(pending.contains("current_stage = 'queued'"));
        assert!(pending.contains("status IS DISTINCT FROM 'pending'"));
        assert!(pending.contains("status IS DISTINCT FROM 'queued'"));
        assert!(pending.contains("ORDER BY created_at DESC, id DESC"));
        assert!(pending.contains("LIMIT $2"));
        assert!(!pending.contains("IS NULL OR"));
        assert!(!pending.contains("ANY("));

        let processing = chip_list_statement(
            false,
            PROCESSING_ARMS,
            CreatedAtPredicate::default(),
            TitlePredicate::default(),
        );
        assert!(processing.contains("status = 'processing'"));
        assert!(processing.contains("current_stage = 'extracting'"));
        assert!(processing.contains("status IS DISTINCT FROM 'processing'"));
        assert!(!processing.contains("= ANY("));

        let pending_tenant = chip_list_statement(
            true,
            PENDING_ARMS,
            CreatedAtPredicate::default(),
            TitlePredicate::default(),
        );
        assert!(pending_tenant.contains("tenant_id = $2 AND status = 'pending'"));
        assert!(pending_tenant.contains("tenant_id IS NULL AND current_stage = 'queued'"));
        assert!(pending_tenant.contains("LIMIT $3"));

        let unknown = chip_list_statement(
            false,
            UNKNOWN_ARMS,
            CreatedAtPredicate::default(),
            TitlePredicate::default(),
        );
        assert!(unknown.contains("status = 'chunking'"));
        assert!(unknown.contains("status = 'extracting'"));
        assert!(unknown.contains("status = 'embedding'"));
        assert!(unknown.contains("status = 'indexing'"));
        assert!(unknown.contains("status = 'deleting'"));
        assert!(unknown.contains("status = 'delete_failed'"));
        assert!(unknown.contains("UNION ALL"));
        assert!(unknown.contains("ORDER BY created_at DESC, id DESC"));
        assert!(unknown.contains("LIMIT $2"));
        assert!(!unknown.contains("NOT IN"));
        assert!(!unknown.contains("IS NULL OR"));
        assert!(!unknown.contains("ANY("));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn date_page_requires_created_at() {
        let open = document_list_statement(
            false,
            0,
            CreatedAtPredicate {
                from: false,
                to: true,
            },
            TitlePredicate::default(),
        );
        assert!(open.contains("workspace_id = $1 AND created_at <= $2"));
        assert!(open.contains("ORDER BY created_at DESC"));
        assert!(!open.contains("id DESC"));
        assert!(open.contains("LIMIT $3"));
        assert!(!open.contains("IS NULL OR"));

        let ranged = document_list_statement(
            true,
            1,
            CreatedAtPredicate {
                from: true,
                to: true,
            },
            TitlePredicate::default(),
        );
        assert!(ranged
            .contains("tenant_id = $2 AND status = $3 AND created_at >= $4 AND created_at <= $5"));
        assert!(ranged.contains(
            "tenant_id IS NULL AND status = $3 AND created_at >= $4 AND created_at <= $5"
        ));
        assert!(ranged.contains("LIMIT $6"));

        let pending = chip_list_statement(
            false,
            PENDING_ARMS,
            CreatedAtPredicate {
                from: true,
                to: false,
            },
            TitlePredicate::default(),
        );
        assert!(pending.contains("status = 'pending' AND created_at >= $2"));
        assert!(pending.contains(
            "current_stage = 'queued' AND status IS DISTINCT FROM 'pending' AND status IS DISTINCT FROM 'queued' AND created_at >= $2"
        ));
        assert!(pending.contains("LIMIT $3"));
        assert!(!pending.contains("IS NULL"));
        assert!(!pending.contains("LIKE"));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn query_filter_requires_the_passed_predicate() {
        let dated = document_filter_statement(
            CreatedAtPredicate {
                from: false,
                to: true,
            },
            TitlePredicate::default(),
            false,
        );
        assert!(dated.contains("workspace_id = $1 AND created_at <= $3"));
        assert!(dated.contains("metadata->>'workspace_id' = $2"));
        assert!(dated.contains("workspace_id IS DISTINCT FROM $1 AND created_at <= $3"));
        assert!(!dated.contains("LIKE"));
        assert!(!dated.contains("ANY("));
        assert!(!dated.contains("IS NULL OR"));
        assert!(!dated.contains("LIMIT"));

        let titled = document_filter_statement(
            CreatedAtPredicate::default(),
            TitlePredicate { count: 1 },
            false,
        );
        assert!(titled.contains("lower(title) LIKE '%' || lower($3) || '%' ESCAPE '\\'"));
        assert!(!titled.contains("created_at <="));
        assert!(!titled.contains("created_at >="));

        let both = document_filter_statement(
            CreatedAtPredicate {
                from: true,
                to: false,
            },
            TitlePredicate { count: 1 },
            true,
        );
        assert!(both.contains(
            "created_at >= $3 AND (lower(title) LIKE '%' || lower($4) || '%' ESCAPE '\\')"
        ));
        assert!(both.contains("created_at >= $3 AND id = ANY($5::uuid[])"));
        assert_eq!(both.matches("UNION").count(), 3);
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn title_pattern_requires_like() {
        let open = document_list_statement(
            false,
            0,
            CreatedAtPredicate::default(),
            TitlePredicate { count: 1 },
        );
        assert!(open.contains(
            "workspace_id = $1 AND (lower(title) LIKE '%' || lower($2) || '%' ESCAPE '\\')"
        ));
        assert!(open.contains("LIMIT $3"));
        assert!(!open.contains("ANY("));
        assert!(!open.contains("IS NULL OR"));

        let two = document_list_statement(
            false,
            0,
            CreatedAtPredicate {
                from: false,
                to: true,
            },
            TitlePredicate { count: 2 },
        );
        assert!(two.contains("created_at <= $2 AND (lower(title) LIKE '%' || lower($3) || '%' ESCAPE '\\' OR lower(title) LIKE '%' || lower($4) || '%' ESCAPE '\\')"));
        assert!(two.contains("LIMIT $5"));

        let pending = chip_list_statement(
            true,
            PENDING_ARMS,
            CreatedAtPredicate::default(),
            TitlePredicate { count: 1 },
        );
        assert!(pending.contains(
            "tenant_id = $2 AND status = 'pending' AND (lower(title) LIKE '%' || lower($3) || '%' ESCAPE '\\')"
        ));
        assert!(pending.contains("LIMIT $4"));
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn title_search_requires_the_title_predicate() {
        assert_eq!(escape_title_like_needle("100%_a\\b"), "100\\%\\_a\\\\b");
        let ready = document_title_search_statement(true, true);
        assert!(ready.contains("lower(title) LIKE '%' || lower($3) || '%' ESCAPE '\\'"));
        assert!(ready.contains("tenant_id = $2"));
        assert!(ready.contains("tenant_id IS NULL"));
        assert!(ready.contains("status IN ('completed', 'indexed')"));
        assert!(ready.contains("LIMIT $4"));
        assert!(!ready.contains("IS NULL OR"));
        let all = document_title_search_statement(false, false);
        assert!(all.contains("lower(title) LIKE '%' || lower($2) || '%' ESCAPE '\\'"));
        assert!(!all.contains("status IN"));
        assert!(all.contains("LIMIT $3"));
    }

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
    fn iss089_probe_limit_from_chunk_counts() {
        assert_eq!(
            reconcile_probe_limit_from_chunk_counts(std::iter::empty()),
            256
        );
        assert_eq!(
            reconcile_probe_limit_from_chunk_counts([0, 0].into_iter()),
            256
        );
        assert_eq!(
            reconcile_probe_limit_from_chunk_counts([3, 12, 5].into_iter()),
            12
        );
        assert_eq!(
            reconcile_probe_limit_from_chunk_counts([400].into_iter()),
            256
        );
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
            progress_counts: None,
            run_progress: None,
            queue_position: None,
            eta_seconds: None,
            eta_basis: None,
            query_ready: None,
            cancelled_from_stage: None,
            extraction: Default::default(),
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
            progress_counts: None,
            run_progress: None,
            queue_position: None,
            eta_seconds: None,
            eta_basis: None,
            query_ready: None,
            cancelled_from_stage: None,
            extraction: Default::default(),
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
            progress_counts: None,
            run_progress: None,
            queue_position: None,
            eta_seconds: None,
            eta_basis: None,
            query_ready: None,
            cancelled_from_stage: None,
            extraction: Default::default(),
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
            progress_counts: None,
            run_progress: None,
            queue_position: None,
            eta_seconds: None,
            eta_basis: None,
            query_ready: None,
            cancelled_from_stage: None,
            extraction: Default::default(),
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
            progress_counts: None,
            run_progress: None,
            queue_position: None,
            eta_seconds: None,
            eta_basis: None,
            query_ready: None,
            cancelled_from_stage: None,
            extraction: Default::default(),
        }
    }

    #[test]
    fn merge_keeps_kv_filename_over_relational_untitled_placeholder() {
        let mut kv = summary("doc-1", "pending", Some("queued"));
        kv.title = Some("notes.md".into());
        kv.file_name = Some("notes.md".into());
        let mut pg = summary("doc-1", "processing", None);
        pg.title = Some("Untitled".into());
        pg.file_name = Some("Untitled".into());
        let merged = merge_document_summaries(vec![kv], vec![pg]);
        assert_eq!(merged[0].title.as_deref(), Some("notes.md"));
        assert_eq!(merged[0].file_name.as_deref(), Some("notes.md"));
    }

    #[test]
    fn merge_overlays_relational_title_when_not_placeholder() {
        let mut kv = summary("doc-1", "completed", None);
        kv.title = Some("old.md".into());
        let mut pg = summary("doc-1", "completed", None);
        pg.title = Some("renamed.md".into());
        let merged = merge_document_summaries(vec![kv], vec![pg]);
        assert_eq!(merged[0].title.as_deref(), Some("renamed.md"));
    }

    #[test]
    fn should_overlay_skips_untitled_when_kv_has_file_name_only() {
        let mut kv = summary("doc-1", "pending", None);
        kv.title = None;
        kv.file_name = Some("readme.md".into());
        assert!(!should_overlay_relational_title(Some("Untitled"), &kv));
        assert!(should_overlay_relational_title(Some("readme.md"), &kv));
        assert!(!should_overlay_relational_title(Some(""), &kv));
        assert!(!should_overlay_relational_title(None, &kv));
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
    fn merge_keeps_kv_deleting_over_stale_relational_completed() {
        // SPEC-098 F-098-12: mid-delete list honesty.
        let kv = vec![summary("doc-1", "deleting", Some("deleting"))];
        let pg = vec![summary("doc-1", "completed", Some("completed"))];
        let merged = merge_document_summaries(kv, pg);
        assert_eq!(
            merged[0].status.as_deref(),
            Some("deleting"),
            "KV deleting must beat stale SQL completed"
        );
    }

    #[test]
    fn merge_keeps_kv_deleting_over_relational_indexed() {
        let kv = vec![summary("doc-1", "deleting", Some("deleting"))];
        let pg = vec![summary("doc-1", "indexed", None)];
        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged[0].status.as_deref(), Some("deleting"));
    }

    #[test]
    fn merge_prefers_relational_delete_failed_over_kv_deleting() {
        // Intentional fail-closed terminal must surface.
        let kv = vec![summary("doc-1", "deleting", Some("deleting"))];
        let pg = vec![summary("doc-1", "delete_failed", Some("delete_failed"))];
        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged[0].status.as_deref(), Some("delete_failed"));
        assert_eq!(merged[0].current_stage.as_deref(), Some("delete_failed"));
    }

    #[test]
    fn merge_keeps_kv_delete_failed_over_shell_collapsed_sql_failed() {
        // SPEC-098 LAW-098-11: legacy shell collapse must not win.
        let kv = vec![summary("doc-1", "delete_failed", Some("delete_failed"))];
        let pg = vec![summary("doc-1", "failed", Some("failed"))];
        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged[0].status.as_deref(), Some("delete_failed"));
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
        // Both terminal → relational may refine (indexed vs completed).
        assert_eq!(merged[0].status.as_deref(), Some("indexed"));
    }

    #[test]
    fn merge_keeps_kv_cancelled_over_stale_relational_processing() {
        let kv = vec![summary("doc-1", "cancelled", Some("cancelled"))];
        let pg = vec![summary("doc-1", "processing", Some("converting"))];
        let merged = merge_document_summaries(kv, pg);
        assert_eq!(
            merged[0].status.as_deref(),
            Some("cancelled"),
            "terminal KV must not be resurrected by stale relational processing"
        );
    }

    #[test]
    fn merge_keeps_kv_failed_over_stale_relational_embedding() {
        let kv = vec![summary("doc-1", "failed", Some("failed"))];
        let pg = vec![summary("doc-1", "embedding", None)];
        let merged = merge_document_summaries(kv, pg);
        assert_eq!(merged[0].status.as_deref(), Some("failed"));
    }

    #[test]
    fn merge_prefers_relational_cancelled_over_kv_embedding() {
        // Incident shape: cancel touched documents.status but left KV at embedding 99%.
        let mut kv = summary("doc-1", "extracting", Some("embedding"));
        kv.stage_progress = Some(0.99);
        kv.stage_message = Some("Embedding chunks: starting (58 total)".into());
        let pg = vec![summary("doc-1", "cancelled", Some("cancelled"))];
        let merged = merge_document_summaries(vec![kv], pg);
        assert_eq!(merged[0].status.as_deref(), Some("cancelled"));
        assert_eq!(merged[0].current_stage.as_deref(), Some("cancelled"));
        assert_eq!(merged[0].stage_progress, Some(0.0));
        assert_eq!(
            merged[0].stage_message.as_deref(),
            Some("Processing cancelled")
        );
    }

    #[test]
    fn merge_prefers_relational_failed_over_kv_extracting() {
        let mut kv = summary("doc-1", "processing", Some("extracting"));
        kv.stage_progress = Some(0.5);
        let pg = vec![summary("doc-1", "failed", Some("failed"))];
        let merged = merge_document_summaries(vec![kv], pg);
        assert_eq!(merged[0].status.as_deref(), Some("failed"));
        assert_eq!(merged[0].current_stage.as_deref(), Some("failed"));
        assert_eq!(merged[0].stage_progress, Some(0.0));
    }
}
