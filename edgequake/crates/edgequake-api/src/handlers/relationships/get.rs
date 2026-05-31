//! Get single relationship handler (FEAT0530).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::{ApiError, ApiResult};
use crate::handlers::relationships_types::{
    EntitySummary, GetRelationshipResponse, RelationshipEntities,
};
use crate::state::AppState;

use super::helpers::edge_to_relationship_response;

/// Get a relationship by ID.
///
/// Note: This implementation searches through all relationships.
/// In production, you'd want an indexed lookup by relationship ID.
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
    Path(relationship_id): Path<String>,
) -> ApiResult<Json<GetRelationshipResponse>> {
    // Search through all edges to find matching relationship ID
    // This is inefficient but works for the prototype
    // In production, maintain a separate index for relationship IDs

    // Get all nodes and search their edges
    let nodes = state.storage.graph_storage.get_all_nodes().await?;

    for node in nodes {
        let edges = state.storage.graph_storage.get_node_edges(&node.id).await?;

        for edge in edges {
            let edge_id = edge
                .properties
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if edge_id == relationship_id {
                // Found the relationship
                let relationship = edge_to_relationship_response(edge.clone(), &relationship_id);

                // Get entity summaries
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

                return Ok(Json(GetRelationshipResponse {
                    relationship,
                    entities,
                }));
            }
        }
    }

    Err(ApiError::NotFound(format!(
        "Relationship '{}' not found",
        relationship_id
    )))
}
