//! Delete relationship handler (FEAT0533).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::error::ApiResult;
use crate::handlers::relationships_types::DeleteRelationshipResponse;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::helpers::find_relationship_edge;

/// Delete a relationship.
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
    tenant_ctx: TenantContext,
    Path(relationship_id): Path<String>,
) -> ApiResult<Json<DeleteRelationshipResponse>> {
    let edge =
        find_relationship_edge(&state.storage.graph_storage, &tenant_ctx, &relationship_id).await?;

    let src_id = edge.source.clone();
    let tgt_id = edge.target.clone();

    state
        .storage
        .graph_storage
        .delete_edge(&src_id, &tgt_id)
        .await?;

    Ok(Json(DeleteRelationshipResponse {
        status: "success".to_string(),
        message: "Relationship deleted successfully".to_string(),
        deleted_relationship_id: relationship_id,
        src_id,
        tgt_id,
    }))
}
