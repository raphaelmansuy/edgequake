//! Single-node lookup handler (`GET /api/v1/graph/nodes/{node_id}`).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::OptionalAuth;
use crate::handlers::graph_types::GraphNodeResponse;
use crate::handlers::isolation::load_node_for_tenant_context;
use crate::middleware::TenantContext;
use crate::services::spec146_authz::{
    graph_properties_in_allow, optional_auth_user_id, resolve_optional_allow_ids,
    sanitize_graph_description, sanitize_graph_properties,
};
use crate::state::{AppState, StorageRuntime};

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
    State(state): State<AppState>,
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    OptionalAuth(auth_user): OptionalAuth,
    Path(node_id): Path<String>,
) -> ApiResult<Json<GraphNodeResponse>> {
    let node =
        load_node_for_tenant_context(storage.graph_storage.as_ref(), &node_id, &tenant_ctx).await?;

    let user_id = optional_auth_user_id(auth_user.as_ref(), &tenant_ctx);
    let allow_ids = resolve_optional_allow_ids(&state, &tenant_ctx, user_id.as_deref()).await?;
    if !graph_properties_in_allow(&node.properties, allow_ids.as_deref()) {
        return Err(ApiError::NotFound("Node not found".into()));
    }

    let degree = storage.graph_storage.node_degree(&node_id).await?;

    Ok(Json(GraphNodeResponse {
        id: node.id.clone(),
        label: crate::handlers::graph::graph_node_label(&node),
        node_type: node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
        description: sanitize_graph_description(&node.properties, allow_ids.as_deref()),
        degree,
        properties: sanitize_graph_properties(&node.properties, allow_ids.as_deref()),
    }))
}
