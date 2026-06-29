//! Lightweight document search handler for the scope picker (SPEC-031).
//!
//! `GET /api/v1/documents/search?q=&page_size=&status=`
//!
//! Returns minimal projections (id, title, status, created_at) — no chunk
//! counts, no entity counts, no cost data.  Optimised for type-ahead UI.

use axum::{
    extract::{Query, State},
    Json,
};
use tracing::debug;

use crate::error::ApiResult;
use crate::handlers::documents_types::{
    DocumentSearchItem, DocumentSearchRequest, DocumentSearchResponse,
};
use crate::middleware::TenantContext;
use crate::services::document_metadata_scan::load_scoped_document_metadata;
use crate::services::tenant_guard::{has_full_tenant_context, warn_missing_tenant_context};
use crate::state::StorageRuntime;

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
        ("status" = Option<String>, Query, description = "Status filter: 'completed' (default) or 'all'"),
    ),
    responses(
        (status = 200, description = "Search results", body = DocumentSearchResponse),
    )
)]
pub async fn search_documents(
    State(storage): State<StorageRuntime>,
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

    // Normalise and cap the query string
    let query_lower: Option<String> = params
        .q
        .as_deref()
        .map(|q| q[..q.len().min(200)].to_lowercase())
        .filter(|q| !q.is_empty());

    let require_completed = params.status.as_deref().map_or(true, |s| s != "all");

    // Load metadata via the same SSOT as list_documents (SPEC-027)
    let metadata_values =
        load_scoped_document_metadata(storage.kv_storage.as_ref(), &tenant_ctx).await?;

    debug!(
        workspace_id = ?tenant_ctx.workspace_id,
        query = ?query_lower,
        metadata_count = metadata_values.len(),
        "search_documents: scanning metadata"
    );

    let mut items: Vec<DocumentSearchItem> = Vec::new();

    for value in &metadata_values {
        let obj = match value.as_object() {
            Some(o) => o,
            None => continue,
        };

        let id = match obj.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };

        let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("file_name").and_then(|v| v.as_str()))
            .unwrap_or(id)
            .to_string();

        let status = obj
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Status filter
        if require_completed && status != "completed" {
            continue;
        }

        // Title substring filter (case-insensitive)
        if let Some(ref q) = query_lower {
            if !title.to_lowercase().contains(q.as_str()) {
                continue;
            }
        }

        let created_at = obj
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        items.push(DocumentSearchItem {
            id: id.to_string(),
            title,
            status,
            created_at,
        });
    }

    // Sort: most recently created first
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = items.len();
    let has_more = total > page_size;
    items.truncate(page_size);

    Ok(Json(DocumentSearchResponse {
        items,
        total,
        has_more,
    }))
}
