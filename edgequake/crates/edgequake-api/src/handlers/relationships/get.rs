//! Get single relationship handler (FEAT0530).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::isolation::load_node_for_tenant_context;
use crate::handlers::relationships_types::{
    EntitySummary, GetRelationshipResponse, RelationshipEntities,
};
use crate::middleware::TenantContext;
use crate::state::StorageRuntime;

use super::helpers::{edge_to_relationship_response, find_relationship_edge};

/// Get a relationship by ID.
#[utoipa::path(
    get,
    path = "/api/v1/graph/relationships/{relationship_id}",
    tag = "Relationships",
    params(
        ("relationship_id" = String, Path, description = "Relationship ID")
    ),
    responses(
        (status = 200, description = "Relationship retrieved", body = GetRelationshipResponse),
        (status = 404, description = "Relationship not found")
    )
)]
pub async fn get_relationship(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Path(relationship_id): Path<String>,
) -> ApiResult<Json<GetRelationshipResponse>> {
    let edge =
        find_relationship_edge(&storage.graph_storage, &tenant_ctx, &relationship_id).await?;

    let relationship = edge_to_relationship_response(edge.clone(), &relationship_id);

    let source_node =
        load_node_for_tenant_context(storage.graph_storage.as_ref(), &edge.source, &tenant_ctx)
            .await?;

    let target_node =
        load_node_for_tenant_context(storage.graph_storage.as_ref(), &edge.target, &tenant_ctx)
            .await?;

    let entities = RelationshipEntities {
        source: EntitySummary {
            id: edge.source.clone(),
            entity_type: source_node
                .properties
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string(),
        },
        target: EntitySummary {
            id: edge.target.clone(),
            entity_type: target_node
                .properties
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN")
                .to_string(),
        },
    };

    Ok(Json(GetRelationshipResponse {
        relationship,
        entities,
    }))
}
