//! Popular labels and batch degree handlers.
//!
//! - `get_popular_labels` — top entities sorted by connection count
//! - `get_degrees_batch` — bulk degree lookup (50× faster than N individual queries)

use axum::{
    extract::{Query, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::auth::OptionalAuth;
use crate::handlers::graph_types::*;
use crate::middleware::TenantContext;
use crate::services::{admit_graph_materialization, run_timed_graph_query};
use crate::state::{AppState, GraphQueryRuntime, StorageRuntime};

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
    State(storage): State<StorageRuntime>,
    State(graph): State<GraphQueryRuntime>,
    tenant_ctx: TenantContext,
    OptionalAuth(auth_user): OptionalAuth,
    Query(params): Query<PopularLabelsQuery>,
) -> ApiResult<Json<PopularLabelsResponse>> {
    let _materialize_guard = admit_graph_materialization(&graph)?;

    // SPEC-011 iter 02 Fix B: total_entities feeds a dashboard counter — the
    // planner estimate is O(1) and accurate within autovacuum's threshold.
    let total_entities = storage.graph_storage.node_count_fast().await?;

    let limit = params.limit;
    let min_degree = params.min_degree;
    let entity_type = params.entity_type.clone();
    let tenant_id = tenant_ctx.tenant_id.clone();
    let workspace_id = tenant_ctx.workspace_id.clone();
    let graph_storage = storage.graph_storage.clone();

    // SPEC-146: when ABAC on, over-fetch then filter by allow-set provenance.
    let user_id = crate::services::spec146_authz::optional_auth_user_id(
        auth_user.as_ref(),
        &tenant_ctx,
    );
    let allow_ids = crate::services::spec146_authz::resolve_optional_allow_ids(
        &state,
        &tenant_ctx,
        user_id.as_deref(),
    )
    .await?;
    let fetch_limit = if allow_ids.is_some() {
        limit.saturating_mul(4).max(limit).min(500)
    } else {
        limit
    };

    let popular_nodes = run_timed_graph_query(&graph.budget, "popular_labels", async move {
        graph_storage
            .get_popular_nodes_with_degree(
                fetch_limit,
                min_degree,
                entity_type.as_deref(),
                tenant_id.as_deref(),
                workspace_id.as_deref(),
            )
            .await
    })
    .await?;

    let popular_nodes = crate::services::spec146_authz::filter_graph_nodes_by_allow(
        popular_nodes,
        allow_ids.as_deref(),
        limit,
    );

    // LAW-146-11: never advertise unfiltered workspace-global degrees under ABAC.
    let mut labels = Vec::with_capacity(popular_nodes.len());
    for (node, raw_degree) in popular_nodes {
        let degree = if allow_ids.is_some() {
            crate::services::spec146_authz::authorized_node_degree(
                storage.graph_storage.as_ref(),
                &node.id,
                &tenant_ctx,
                allow_ids.as_deref(),
            )
            .await
        } else {
            raw_degree
        };
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();
        let description = crate::services::spec146_authz::sanitize_graph_description(
            &node.properties,
            allow_ids.as_deref(),
        );
        labels.push(PopularLabel {
            label: crate::handlers::graph::graph_node_label(&node),
            entity_type,
            degree,
            description,
        });
    }

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
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    OptionalAuth(auth_user): OptionalAuth,
    Json(request): Json<BatchDegreeRequest>,
) -> ApiResult<Json<BatchDegreeResponse>> {
    if request.node_ids.is_empty() {
        return Ok(Json(BatchDegreeResponse {
            degrees: Vec::new(),
            count: 0,
        }));
    }

    // SPEC-146 / LAW-146-11: provenance-gate nodes; authorized degree only.
    // Unauthorized / missing nodes are omitted (existence-hiding like get_node).
    let user_id = crate::services::spec146_authz::optional_auth_user_id(
        auth_user.as_ref(),
        &tenant_ctx,
    );
    let allow_ids = crate::services::spec146_authz::resolve_optional_allow_ids(
        &state,
        &tenant_ctx,
        user_id.as_deref(),
    )
    .await?;

    let degrees_result = crate::services::spec146_authz::authorized_degrees_batch(
        storage.graph_storage.as_ref(),
        &request.node_ids,
        &tenant_ctx,
        allow_ids.as_deref(),
    )
    .await?;

    let degrees: Vec<NodeDegree> = degrees_result
        .into_iter()
        .map(|(node_id, degree)| NodeDegree { node_id, degree })
        .collect();

    let count = degrees.len();

    Ok(Json(BatchDegreeResponse { degrees, count }))
}
