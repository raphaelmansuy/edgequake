//! Delete relationship handler (FEAT0533).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::{ApiError, ApiResult};
use crate::handlers::relationships_types::DeleteRelationshipResponse;
use crate::state::AppState;

/// Delete a relationship.
///
/// Note: This implementation searches through all relationships.
/// In production, you'd want an indexed lookup by relationship ID.
#[utoipa::path(
    delete,
    path = "/api/v1/graph/relationships/{relationship_id}",
    tag = "Relationships",
    params(
        ("relationship_id" = String, Path, description = "Relationship ID")
    ),
    responses(
        (status = 200, description = "Relationship deleted", body = DeleteRelationshipResponse),
        (status = 404, description = "Relationship not found")
    )
)]
pub async fn delete_relationship(
    State(state): State<AppState>,
    Path(relationship_id): Path<String>,
) -> ApiResult<Json<DeleteRelationshipResponse>> {
    // Search through all edges to find matching relationship ID
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
                // Found the relationship - delete it
                let src_id = edge.source.clone();
                let tgt_id = edge.target.clone();

                state
                    .storage
                    .graph_storage
                    .delete_edge(&src_id, &tgt_id)
                    .await?;

                return Ok(Json(DeleteRelationshipResponse {
                    status: "success".to_string(),
                    message: "Relationship deleted successfully".to_string(),
                    deleted_relationship_id: relationship_id,
                    src_id,
                    tgt_id,
                }));
            }
        }
    }

    Err(ApiError::NotFound(format!(
        "Relationship '{}' not found",
        relationship_id
    )))
}
