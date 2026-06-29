//! Create relationship handler (FEAT0531, BR0530, BR0531).

use axum::{extract::State, Json};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::handlers::isolation::{load_node_for_tenant_context, stamp_tenant_context_properties};
use crate::handlers::relationships_types::{CreateRelationshipRequest, CreateRelationshipResponse};
use crate::middleware::TenantContext;
use crate::state::StorageRuntime;
use edgequake_storage::GraphEdge;

use super::helpers::{edge_to_relationship_response, extract_relation_type, normalize_entity_name};

/// Create a new relationship.
///
/// # Implements
///
/// - **BR0201**: Tenant isolation (relationship created with tenant/workspace context)
#[utoipa::path(
    post,
    path = "/api/v1/graph/relationships",
    tag = "Relationships",
    request_body = CreateRelationshipRequest,
    responses(
        (status = 201, description = "Relationship created", body = CreateRelationshipResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn create_relationship(
    State(storage): State<StorageRuntime>,
    tenant_ctx: TenantContext,
    Json(req): Json<CreateRelationshipRequest>,
) -> ApiResult<Json<CreateRelationshipResponse>> {
    let src_id = normalize_entity_name(&req.src_id);
    let tgt_id = normalize_entity_name(&req.tgt_id);

    // Verify both entities exist in this tenant/workspace
    load_node_for_tenant_context(storage.graph_storage.as_ref(), &src_id, &tenant_ctx).await?;

    load_node_for_tenant_context(storage.graph_storage.as_ref(), &tgt_id, &tenant_ctx).await?;

    // Generate relationship ID
    let rel_id = format!("rel-{}", Uuid::new_v4());

    // Extract relation type from keywords
    let relation_type = extract_relation_type(&req.keywords);

    // Create relationship properties
    let now = Utc::now().to_rfc3339();
    let mut properties = HashMap::new();
    properties.insert("id".to_string(), rel_id.clone().into());
    properties.insert("relation_type".to_string(), relation_type.into());
    properties.insert("keywords".to_string(), req.keywords.clone().into());
    properties.insert("weight".to_string(), req.weight.into());
    properties.insert("description".to_string(), req.description.clone().into());
    properties.insert("source_id".to_string(), req.source_id.clone().into());
    properties.insert("created_at".to_string(), now.clone().into());
    properties.insert("updated_at".to_string(), now.clone().into());
    properties.insert("is_manual".to_string(), true.into());
    properties.insert("metadata".to_string(), req.metadata.clone());

    stamp_tenant_context_properties(&mut properties, &tenant_ctx)?;

    // Create edge using upsert_edge
    storage
        .graph_storage
        .upsert_edge(&src_id, &tgt_id, properties.clone())
        .await?;

    // Reconstruct edge for response
    let edge = GraphEdge {
        source: src_id.clone(),
        target: tgt_id.clone(),
        properties,
    };

    let relationship = edge_to_relationship_response(edge, &rel_id);

    Ok(Json(CreateRelationshipResponse {
        status: "success".to_string(),
        message: "Relationship created successfully".to_string(),
        relationship,
    }))
}
