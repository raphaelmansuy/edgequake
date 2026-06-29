//! Single-node lookup handler (`GET /api/v1/graph/nodes/{node_id}`).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::graph_types::GraphNodeResponse;
use crate::handlers::isolation::load_node_for_tenant_context;
use crate::middleware::TenantContext;
use crate::state::StorageRuntime;

/// Get a specific node.
#[utoipa::path(
    get,
    path = "/api/v1/graph/nodes/{node_id}",
    tag = "Graph",
    params(
        ("node_id" = String, Path, description = "Node ID")
    ),
    responses(
        (status = 200, description = "Node retrieved", body = GraphNodeResponse),
        (status = 404, description = "Node not found")
    )
)]
pub async fn get_node(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(node_id): Path<String>,
) -> ApiResult<Json<GraphNodeResponse>> {
    let node =
        load_node_for_tenant_context(storage.graph_storage.as_ref(), &node_id, &tenant_ctx).await?;

    let degree = storage.graph_storage.node_degree(&node_id).await?;

    Ok(Json(GraphNodeResponse {
        id: node.id.clone(),
        label: node.id.clone(),
        node_type: node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
        description: node
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        degree,
        properties: serde_json::to_value(&node.properties).unwrap_or_default(),
    }))
}
