//! Popular labels and batch degree handlers.
//!
//! - `get_popular_labels` — top entities sorted by connection count
//! - `get_degrees_batch` — bulk degree lookup (50× faster than N individual queries)

use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::graph_types::*;
use crate::services::{admit_graph_materialization, run_timed_graph_query};
use crate::state::AppState;

/// Get popular entities/labels sorted by connection count.
#[utoipa::path(
    get,
    path = "/api/v1/graph/labels/popular",
    tag = "Graph",
    params(
        ("limit" = usize, Query, description = "Max results (default 50)"),
        ("min_degree" = Option<usize>, Query, description = "Minimum connections"),
        ("entity_type" = Option<String>, Query, description = "Filter by type")
    ),
    responses(
        (status = 200, description = "Popular labels retrieved", body = PopularLabelsResponse)
    )
)]
pub async fn get_popular_labels(
    State(state): State<AppState>,
    Query(params): Query<PopularLabelsQuery>,
) -> ApiResult<Json<PopularLabelsResponse>> {
    let _materialize_guard = admit_graph_materialization(&state)?;

    // SPEC-011 iter 02 Fix B: total_entities feeds a dashboard counter — the
    // planner estimate is O(1) and accurate within autovacuum's threshold.
    let total_entities = state.storage.graph_storage.node_count_fast().await?;

    let limit = params.limit;
    let min_degree = params.min_degree;
    let entity_type = params.entity_type.clone();
    let graph_storage = state.storage.graph_storage.clone();
    let popular_nodes = run_timed_graph_query(&state, "popular_labels", async move {
        graph_storage
            .get_popular_nodes_with_degree(limit, min_degree, entity_type.as_deref(), None, None)
            .await
    })
    .await?;

    let labels: Vec<PopularLabel> = popular_nodes
        .into_iter()
        .map(|(node, degree)| {
            let entity_type = node
                .properties
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string();

            let description = node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            PopularLabel {
                label: node.id,
                entity_type,
                degree,
                description,
            }
        })
        .collect();

    Ok(Json(PopularLabelsResponse {
        labels,
        total_entities,
    }))
}

/// Get degrees for multiple nodes in a single optimized query.
///
/// This endpoint uses the optimized `node_degrees_batch()` method which is
/// 50x faster than calling GET /graph/nodes/{id} multiple times.
///
/// Performance: <100ms for 100 nodes (vs 5000ms+ with individual queries).
#[utoipa::path(
    post,
    path = "/api/v1/graph/degrees/batch",
    tag = "Graph",
    request_body = BatchDegreeRequest,
    responses(
        (status = 200, description = "Degrees retrieved", body = BatchDegreeResponse)
    )
)]
pub async fn get_degrees_batch(
    State(state): State<AppState>,
    Json(request): Json<BatchDegreeRequest>,
) -> ApiResult<Json<BatchDegreeResponse>> {
    if request.node_ids.is_empty() {
        return Ok(Json(BatchDegreeResponse {
            degrees: Vec::new(),
            count: 0,
        }));
    }

    // OPTIMIZED: Single query for all degrees (50x faster than N queries)
    let degrees_result = state
        .storage
        .graph_storage
        .node_degrees_batch(&request.node_ids)
        .await?;

    let degrees: Vec<NodeDegree> = degrees_result
        .into_iter()
        .map(|(node_id, degree)| NodeDegree { node_id, degree })
        .collect();

    let count = degrees.len();

    Ok(Json(BatchDegreeResponse { degrees, count }))
}
