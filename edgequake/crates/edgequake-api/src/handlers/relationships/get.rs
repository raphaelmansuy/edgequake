//! Get single relationship handler (FEAT0530).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::{ApiError, ApiResult};
use crate::handlers::relationships_types::{
    EntitySummary, GetRelationshipResponse, RelationshipEntities,
};
use crate::middleware::TenantContext;
use crate::state::AppState;

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
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(relationship_id): Path<String>,
) -> ApiResult<Json<GetRelationshipResponse>> {
    let edge =
        find_relationship_edge(&state.storage.graph_storage, &tenant_ctx, &relationship_id).await?;

    let relationship = edge_to_relationship_response(edge.clone(), &relationship_id);

    let source_node = state
        .storage
        .graph_storage
        .get_node(&edge.source)
        .await?
        .ok_or_else(|| ApiError::NotFound("Source entity not found".to_string()))?;

    let target_node = state
        .storage
        .graph_storage
        .get_node(&edge.target)
        .await?
        .ok_or_else(|| ApiError::NotFound("Target entity not found".to_string()))?;

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
