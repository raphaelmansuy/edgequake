//! Relationship CRUD operations for manual knowledge graph management.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use edgequake_storage::GraphEdge;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// Re-export DTOs for backward compatibility
pub use crate::handlers::relationships_types::{
    default_weight, CreateRelationshipRequest, CreateRelationshipResponse,
    DeleteRelationshipResponse, EntitySummary, GetRelationshipResponse,
    RelationshipChangesSummary, RelationshipEntities, RelationshipResponse,
    UpdateRelationshipRequest, UpdateRelationshipResponse,
};

// ============================================================================
// Request/Response Types (REMOVED - now in relationships_types.rs)
// ============================================================================

// ============================================================================
// Helper Functions
// ============================================================================

/// Normalize entity name to UPPERCASE with underscores.
fn normalize_entity_name(name: &str) -> String {
    name.to_uppercase().replace(' ', "_")
}

/// Extract relation type from keywords.
fn extract_relation_type(keywords: &str) -> String {
    // Simple heuristic: use first keyword as relation type
    keywords
        .split(',')
        .next()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_uppercase().replace(' ', "_"))
        .unwrap_or_else(|| "RELATED_TO".to_string())
}

/// Convert GraphEdge to RelationshipResponse.
fn edge_to_relationship_response(edge: GraphEdge, rel_id: &str) -> RelationshipResponse {
    let props = &edge.properties;

    RelationshipResponse {
        id: rel_id.to_string(),
        src_id: edge.source.clone(),
        tgt_id: edge.target.clone(),
        relation_type: props
            .get("relation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("RELATED_TO")
            .to_string(),
        keywords: props
            .get("keywords")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        weight: props.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.8),
        description: props
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        source_id: props
            .get("source_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        created_at: props
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        updated_at: props
            .get("updated_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        metadata: props
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::json!({})),
    }
}

// ============================================================================
// API Handlers
// ============================================================================

/// Create a new relationship.
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
    State(state): State<AppState>,
    Json(req): Json<CreateRelationshipRequest>,
) -> ApiResult<Json<CreateRelationshipResponse>> {
    let src_id = normalize_entity_name(&req.src_id);
    let tgt_id = normalize_entity_name(&req.tgt_id);

    // Verify both entities exist
    if state.graph_storage.get_node(&src_id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Source entity '{}' not found",
            src_id
        )));
    }

    if state.graph_storage.get_node(&tgt_id).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Target entity '{}' not found",
            tgt_id
        )));
    }

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

    // Create edge using upsert_edge
    state
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
    let nodes = state.graph_storage.get_all_nodes().await?;

    for node in nodes {
        let edges = state.graph_storage.get_node_edges(&node.id).await?;

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
                    .graph_storage
                    .get_node(&edge.source)
                    .await?
                    .ok_or_else(|| ApiError::NotFound("Source entity not found".to_string()))?;

                let target_node = state
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

/// Update a relationship.
///
/// Note: This implementation searches through all relationships.
/// In production, you'd want an indexed lookup by relationship ID.
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
    Path(relationship_id): Path<String>,
    Json(req): Json<UpdateRelationshipRequest>,
) -> ApiResult<Json<UpdateRelationshipResponse>> {
    // Search through all edges to find matching relationship ID
    let nodes = state.graph_storage.get_all_nodes().await?;

    for node in nodes {
        let edges = state.graph_storage.get_node_edges(&node.id).await?;

        for mut edge in edges {
            let edge_id = edge
                .properties
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if edge_id == relationship_id {
                // Found the relationship - update it
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

                // Update timestamp
                let now = Utc::now().to_rfc3339();
                edge.properties.insert("updated_at".to_string(), now.into());

                // Update edge in storage using upsert_edge
                let src = edge.source.clone();
                let tgt = edge.target.clone();
                state
                    .graph_storage
                    .upsert_edge(&src, &tgt, edge.properties.clone())
                    .await?;

                let relationship = edge_to_relationship_response(edge, &relationship_id);

                let changes = RelationshipChangesSummary {
                    fields_updated,
                    previous_weight,
                };

                return Ok(Json(UpdateRelationshipResponse {
                    status: "success".to_string(),
                    message: "Relationship updated successfully".to_string(),
                    relationship,
                    changes,
                }));
            }
        }
    }

    Err(ApiError::NotFound(format!(
        "Relationship '{}' not found",
        relationship_id
    )))
}

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
    let nodes = state.graph_storage.get_all_nodes().await?;

    for node in nodes {
        let edges = state.graph_storage.get_node_edges(&node.id).await?;

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

                state.graph_storage.delete_edge(&src_id, &tgt_id).await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_relation_type() {
        assert_eq!(extract_relation_type("works for, employed by"), "WORKS_FOR");
        assert_eq!(extract_relation_type("located in"), "LOCATED_IN");
        assert_eq!(extract_relation_type(""), "RELATED_TO");
    }

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(
            normalize_entity_name("quantum computing"),
            "QUANTUM_COMPUTING"
        );
    }

    #[test]
    fn test_create_relationship_request_defaults() {
        let json = r#"{
            "src_id": "ENTITY_A",
            "tgt_id": "ENTITY_B",
            "keywords": "works for",
            "description": "Employment relationship",
            "source_id": "manual_entry"
        }"#;
        let request: Result<CreateRelationshipRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.src_id, "ENTITY_A");
        assert_eq!(req.weight, 0.8); // default
    }

    #[test]
    fn test_create_relationship_request_custom_weight() {
        let json = r#"{
            "src_id": "A",
            "tgt_id": "B",
            "keywords": "connects",
            "weight": 0.5,
            "description": "test",
            "source_id": "doc-1"
        }"#;
        let request: Result<CreateRelationshipRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert!((req.weight - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_relationship_request_partial() {
        let json = r#"{"weight": 0.9}"#;
        let request: Result<UpdateRelationshipRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.weight, Some(0.9));
        assert!(req.keywords.is_none());
    }

    #[test]
    fn test_default_weight() {
        assert!((default_weight() - 0.8).abs() < f64::EPSILON);
    }
}
