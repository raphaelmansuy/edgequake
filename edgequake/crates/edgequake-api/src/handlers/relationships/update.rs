//! Update relationship handler (FEAT0532).

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;

use crate::error::ApiResult;
use crate::handlers::relationships_types::{
    RelationshipChangesSummary, UpdateRelationshipRequest, UpdateRelationshipResponse,
};
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::helpers::{edge_to_relationship_response, find_relationship_edge};

/// Update a relationship.
#[utoipa::path(
    put,
    path = "/api/v1/graph/relationships/{relationship_id}",
    tag = "Relationships",
    params(
        ("relationship_id" = String, Path, description = "Relationship ID")
    ),
    request_body = UpdateRelationshipRequest,
    responses(
        (status = 200, description = "Relationship updated", body = UpdateRelationshipResponse),
        (status = 404, description = "Relationship not found")
    )
)]
pub async fn update_relationship(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path(relationship_id): Path<String>,
    Json(req): Json<UpdateRelationshipRequest>,
) -> ApiResult<Json<UpdateRelationshipResponse>> {
    let mut edge =
        find_relationship_edge(&state.storage.graph_storage, &tenant_ctx, &relationship_id).await?;

    let previous_weight = edge.properties.get("weight").and_then(|v| v.as_f64());
    let mut fields_updated = Vec::new();

    if let Some(keywords) = req.keywords {
        edge.properties
            .insert("keywords".to_string(), keywords.into());
        fields_updated.push("keywords".to_string());
    }

    if let Some(weight) = req.weight {
        edge.properties.insert("weight".to_string(), weight.into());
        fields_updated.push("weight".to_string());
    }

    if let Some(description) = req.description {
        edge.properties
            .insert("description".to_string(), description.into());
        fields_updated.push("description".to_string());
    }

    if let Some(metadata) = req.metadata {
        edge.properties.insert("metadata".to_string(), metadata);
        fields_updated.push("metadata".to_string());
    }

    let now = Utc::now().to_rfc3339();
    edge.properties.insert("updated_at".to_string(), now.into());

    let src = edge.source.clone();
    let tgt = edge.target.clone();
    state
        .storage
        .graph_storage
        .upsert_edge(&src, &tgt, edge.properties.clone())
        .await?;

    let relationship = edge_to_relationship_response(edge, &relationship_id);
    let changes = RelationshipChangesSummary {
        fields_updated,
        previous_weight,
    };

    Ok(Json(UpdateRelationshipResponse {
        status: "success".to_string(),
        message: "Relationship updated successfully".to_string(),
        relationship,
        changes,
    }))
}
