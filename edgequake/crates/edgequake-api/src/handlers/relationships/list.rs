//! List relationships handler (FEAT0530).

use axum::{
    extract::{Query, State},
    Json,
};

use edgequake_storage::traits::EdgeListFilter;

use crate::error::ApiResult;
use crate::handlers::relationships_types::{ListRelationshipsQuery, ListRelationshipsResponse};
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::helpers::edge_to_relationship_response;

/// List relationships with pagination and filtering.
#[utoipa::path(
    get,
    path = "/api/v1/graph/relationships",
    tag = "Relationships",
    params(
        ("page" = Option<u32>, Query, description = "Page number (1-indexed, default 1)"),
        ("page_size" = Option<u32>, Query, description = "Page size (default 20, max 100)"),
        ("relationship_type" = Option<String>, Query, description = "Filter by relationship type")
    ),
    responses(
        (status = 200, description = "Paginated list of relationships", body = ListRelationshipsResponse)
    )
)]
pub async fn list_relationships(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Query(query): Query<ListRelationshipsQuery>,
) -> ApiResult<Json<ListRelationshipsResponse>> {
    // SPEC-006: BR-006-010 — AppState resource SSOT
    let page_size = state.resource_budget().clamp_page_size(query.page_size);
    let page = query.page.max(1);
    let offset = ((page - 1) * page_size) as usize;

    // SPEC-006: TR-006-001 — push-down edge pagination
    let filter = EdgeListFilter {
        tenant_id: tenant_ctx.tenant_id.clone(),
        workspace_id: tenant_ctx.workspace_id.clone(),
        relationship_type: query.relationship_type.clone(),
    };

    let page_result = state
        .storage
        .graph_storage
        .list_edges_filtered(&filter, offset, page_size as usize)
        .await?;

    let total = page_result.total;
    let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;
    let page_edges = page_result.items;

    // Convert to response format
    let items: Vec<_> = page_edges
        .into_iter()
        .map(|edge| {
            let rel_id = format!("{}_{}", edge.source, edge.target);
            edge_to_relationship_response(edge, &rel_id)
        })
        .collect();

    Ok(Json(ListRelationshipsResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    }))
}
