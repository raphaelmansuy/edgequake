//! List relationships handler (FEAT0530).

use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::isolation::filter_edges_by_tenant_context;
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
    // Clamp page_size to range [1, 100]
    let page_size = query.page_size.clamp(1, 100);
    let page = query.page.max(1);
    let offset = ((page - 1) * page_size) as usize;

    // Get all edges from graph storage
    // WHY: We need to fetch all edges and filter in memory because the storage
    // interface doesn't support pagination/filtering yet.
    let all_edges = state.storage.graph_storage.get_all_edges().await?;

    // WHY: Apply tenant isolation first to ensure multi-tenancy is respected
    let tenant_filtered_edges = filter_edges_by_tenant_context(all_edges, &tenant_ctx);

    // Apply filters
    let mut filtered_edges: Vec<_> = tenant_filtered_edges
        .into_iter()
        .filter(|edge| {
            // Filter by relationship_type if specified
            if let Some(ref rel_type) = query.relationship_type {
                let edge_type = edge
                    .properties
                    .get("relation_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !edge_type.eq_ignore_ascii_case(rel_type) {
                    return false;
                }
            }
            true
        })
        .collect();

    // Sort by edge ID for consistent ordering
    filtered_edges.sort_by(|a, b| {
        let a_id = format!("{}_{}", a.source, a.target);
        let b_id = format!("{}_{}", b.source, b.target);
        a_id.cmp(&b_id)
    });

    let total = filtered_edges.len();
    let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;

    // Apply pagination
    let page_edges: Vec<_> = filtered_edges
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .collect();

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
