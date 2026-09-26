//! Lightweight document search handler for the scope picker (SPEC-031).
//!
//! `GET /api/v1/documents/search?q=&page_size=&status=`
//!
//! Returns minimal projections (id, title, status, created_at) — no chunk
//! counts, no entity counts, no cost data.  Optimised for type-ahead UI.

use std::collections::HashSet;

use axum::{
    extract::{Query, State},
    Json,
};
use serde_json::Value;
use tracing::debug;

use crate::error::ApiResult;
use crate::handlers::documents_types::{
    DocumentSearchItem, DocumentSearchRequest, DocumentSearchResponse, DocumentSummary,
};
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_scoped_document_metadata;
use crate::services::tenant_guard::{has_full_tenant_context, warn_missing_tenant_context};
use crate::state::{PostgresRuntime, StorageRuntime};

/// Candidate row before status, title, and page filters.
struct SearchCandidate {
    id: String,
    title: String,
    status: String,
    created_at: Option<String>,
}

/// Scope picker `status=completed` matches the documents-list completed bucket.
///
/// Relational writes store terminal success as `indexed`
/// (`relational_documents_status_for_write`). The picker always asks for
/// `completed`, so both values must pass.
fn is_scope_ready_status(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "completed" | "indexed"
    )
}

/// `status=all` lists every row. Any other value (including the default
/// `completed`) keeps terminal-success documents only.
fn passes_status_filter(status_param: Option<&str>, status: &str) -> bool {
    if status_param.is_some_and(|s| s.eq_ignore_ascii_case("all")) {
        return true;
    }
    is_scope_ready_status(status)
}

fn passes_title_filter(query_lower: Option<&str>, title: &str) -> bool {
    match query_lower {
        Some(q) if !q.is_empty() => title.to_lowercase().contains(q),
        _ => true,
    }
}

fn search_candidate_from_metadata(value: &Value) -> Option<SearchCandidate> {
    let obj = value.as_object()?;
    let id = obj.get("id").and_then(Value::as_str)?.to_string();
    if id.is_empty() {
        return None;
    }
    let title = obj
        .get("title")
        .and_then(Value::as_str)
        .or_else(|| obj.get("file_name").and_then(Value::as_str))
        .unwrap_or(id.as_str())
        .to_string();
    let status = obj
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let created_at = obj
        .get("created_at")
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(SearchCandidate {
        id,
        title,
        status,
        created_at,
    })
}

fn search_candidate_from_summary(doc: &DocumentSummary) -> SearchCandidate {
    let title = doc
        .title
        .as_deref()
        .filter(|t| !t.is_empty())
        .or(doc.file_name.as_deref().filter(|t| !t.is_empty()))
        .unwrap_or(doc.id.as_str())
        .to_string();
    SearchCandidate {
        id: doc.id.clone(),
        title,
        status: doc.status.clone().unwrap_or_else(|| "unknown".to_string()),
        created_at: doc.created_at.clone(),
    }
}

/// Keep metadata rows, then append relational rows whose ids are missing.
fn merge_search_candidates(
    metadata: Vec<SearchCandidate>,
    relational: Vec<SearchCandidate>,
) -> Vec<SearchCandidate> {
    let mut seen: HashSet<String> = metadata.iter().map(|c| c.id.clone()).collect();
    let mut out = metadata;
    for rel in relational {
        if seen.insert(rel.id.clone()) {
            out.push(rel);
        }
    }
    out
}

fn filter_search_candidates(
    candidates: impl IntoIterator<Item = SearchCandidate>,
    query_lower: Option<&str>,
    status_param: Option<&str>,
) -> Vec<DocumentSearchItem> {
    let mut items: Vec<DocumentSearchItem> = candidates
        .into_iter()
        .filter(|c| passes_status_filter(status_param, &c.status))
        .filter(|c| passes_title_filter(query_lower, &c.title))
        .map(|c| DocumentSearchItem {
            id: c.id,
            title: c.title,
            status: c.status,
            created_at: c.created_at,
        })
        .collect();
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    items
}

/// Search documents by title for the scope picker.
///
/// Requires full tenant context (workspace_id + tenant_id).
/// Returns at most 50 results sorted by `created_at` descending.
///
/// @implements SPEC-031: Document search endpoint
#[utoipa::path(
    get,
    path = "/api/v1/documents/search",
    tag = "Documents",
    params(
        ("q" = Option<String>, Query, description = "Title search query (case-insensitive substring)"),
        ("page_size" = Option<usize>, Query, description = "Max results (default 20, max 50)"),
        ("status" = Option<String>, Query, description = "Status filter: 'completed' (default; includes indexed) or 'all'"),
    ),
    responses(
        (status = 200, description = "Search results", body = DocumentSearchResponse),
    )
)]
pub async fn search_documents(
    State(storage): State<StorageRuntime>,
    State(pg_runtime): State<PostgresRuntime>,
    tenant_ctx: TenantContext,
    Query(params): Query<DocumentSearchRequest>,
) -> ApiResult<Json<DocumentSearchResponse>> {
    // Security: require full tenant context — same guard as list_documents
    if !has_full_tenant_context(&tenant_ctx) {
        warn_missing_tenant_context(&tenant_ctx, "search_documents");
        return Ok(Json(DocumentSearchResponse {
            items: vec![],
            total: 0,
            has_more: false,
        }));
    }

    // Hard cap on page_size to prevent abuse
    let page_size = params.page_size.min(50);

    // Normalise and cap the query string (UTF-8–safe byte cap)
    let query_lower: Option<String> = params
        .q
        .as_deref()
        .map(|q| edgequake_observability::utf8_prefix(q, 200).to_lowercase())
        .filter(|q| !q.is_empty());

    // Same membership SSOT as list_documents. A `None` pool skips the relational
    // `documents` index and returns an empty picker after the KV cutover.
    let metadata_values = {
        let pool = pg_runtime.optional_pg_pool();
        load_scoped_document_metadata(storage.kv_storage.as_ref(), pool, &tenant_ctx).await?
    };

    debug!(
        workspace_id = ?tenant_ctx.workspace_id,
        query = ?query_lower,
        metadata_count = metadata_values.len(),
        "search_documents: scanning metadata"
    );

    let mut candidates: Vec<SearchCandidate> = metadata_values
        .iter()
        .filter_map(search_candidate_from_metadata)
        .collect();

    // SPEC-021 P5-01: relational `documents` is the durable list when KV
    // `*-metadata` is empty or the KV relation has been dropped. The graph
    // is written on a separate path, so an empty scan here shows "No completed
    // documents" while the explorer still has nodes.
    #[cfg(feature = "postgres")]
    if pg_runtime.pool.is_some() {
        match crate::document_read_model::list_relational_document_summaries(
            pg_runtime.pool.as_ref(),
            &tenant_ctx,
        )
        .await
        {
            Ok(relational) if !relational.is_empty() => {
                let extra = relational
                    .iter()
                    .map(search_candidate_from_summary)
                    .collect();
                candidates = merge_search_candidates(candidates, extra);
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(
                    error = %e,
                    tenant = ?tenant_ctx.tenant_id,
                    workspace = ?tenant_ctx.workspace_id,
                    "Relational document backfill failed — scope picker may show 0 docs"
                );
            }
        }
    }

    let mut items =
        filter_search_candidates(candidates, query_lower.as_deref(), params.status.as_deref());
    let total = items.len();
    let has_more = total > page_size;
    items.truncate(page_size);

    Ok(Json(DocumentSearchResponse {
        items,
        total,
        has_more,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, title: &str, status: &str, created_at: Option<&str>) -> SearchCandidate {
        SearchCandidate {
            id: id.to_string(),
            title: title.to_string(),
            status: status.to_string(),
            created_at: created_at.map(str::to_string),
        }
    }

    fn ids(items: &[DocumentSearchItem]) -> Vec<&str> {
        items.iter().map(|i| i.id.as_str()).collect()
    }

    #[test]
    fn completed_filter_includes_indexed_and_excludes_processing() {
        let items = filter_search_candidates(
            vec![
                candidate(
                    "c",
                    "Completed paper",
                    "completed",
                    Some("2026-01-02T00:00:00Z"),
                ),
                candidate(
                    "i",
                    "Indexed paper",
                    "indexed",
                    Some("2026-01-03T00:00:00Z"),
                ),
                candidate(
                    "p",
                    "Still running",
                    "processing",
                    Some("2026-01-04T00:00:00Z"),
                ),
            ],
            None,
            Some("completed"),
        );
        assert_eq!(ids(&items), vec!["i", "c"]);
    }

    #[test]
    fn all_does_not_filter_status() {
        let items = filter_search_candidates(
            vec![
                candidate("c", "Completed paper", "completed", None),
                candidate("p", "Still running", "processing", None),
            ],
            None,
            Some("all"),
        );
        assert_eq!(ids(&items), vec!["c", "p"]);
    }

    #[test]
    fn backfill_adds_missing_and_skips_duplicate() {
        let merged = merge_search_candidates(
            vec![candidate("keep", "From metadata", "completed", None)],
            vec![
                candidate("keep", "Relational duplicate", "indexed", None),
                candidate("new", "Only in documents table", "indexed", None),
            ],
        );
        let ids: Vec<&str> = merged.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["keep", "new"]);
        assert_eq!(merged[0].title, "From metadata");
    }

    #[test]
    fn title_query_ai_matches_relational_title_only() {
        let merged = merge_search_candidates(
            vec![],
            vec![
                candidate(
                    "hit",
                    "AI Safety Notes",
                    "indexed",
                    Some("2026-02-01T00:00:00Z"),
                ),
                candidate(
                    "miss",
                    "Budget Review",
                    "completed",
                    Some("2026-02-02T00:00:00Z"),
                ),
                candidate(
                    "inflight",
                    "AI draft",
                    "processing",
                    Some("2026-02-03T00:00:00Z"),
                ),
            ],
        );
        let items = filter_search_candidates(merged, Some("ai"), Some("completed"));
        assert_eq!(ids(&items), vec!["hit"]);
        assert_eq!(items[0].title, "AI Safety Notes");
    }

    #[test]
    fn summary_backfill_uses_title_and_keeps_indexed_status() {
        let summary = DocumentSummary {
            id: "rel-1".into(),
            title: Some("AI Paper".into()),
            file_name: Some("ignored.pdf".into()),
            content_summary: None,
            content_length: None,
            chunk_count: 0,
            entity_count: None,
            status: Some("indexed".into()),
            error_message: None,
            warning_message: None,
            track_id: None,
            created_at: Some("2026-03-01T00:00:00Z".into()),
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
            queue_position: None,
            eta_seconds: None,
            eta_basis: None,
            query_ready: None,
            cancelled_from_stage: None,
        };
        let merged = merge_search_candidates(vec![], vec![search_candidate_from_summary(&summary)]);
        let items = filter_search_candidates(merged, Some("ai"), Some("completed"));
        assert_eq!(ids(&items), vec!["rel-1"]);
        assert_eq!(items[0].status, "indexed");
    }
}
