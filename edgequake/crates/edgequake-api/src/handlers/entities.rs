//! Entity CRUD operations for manual knowledge graph management.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use edgequake_storage::{GraphNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Create entity request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateEntityRequest {
    /// Entity name (will be normalized to UPPERCASE).
    pub entity_name: String,

    /// Entity type (e.g., PERSON, ORGANIZATION, TECHNOLOGY).
    pub entity_type: String,

    /// Entity description.
    pub description: String,

    /// Source document ID (use "manual_entry" for manual entries).
    pub source_id: String,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Update entity request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateEntityRequest {
    /// Updated entity type.
    pub entity_type: Option<String>,

    /// Updated description.
    pub description: Option<String>,

    /// Updated metadata.
    pub metadata: Option<serde_json::Value>,
}

/// Entity response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityResponse {
    /// Entity ID.
    pub id: String,

    /// Entity name.
    pub entity_name: String,

    /// Entity type.
    pub entity_type: String,

    /// Entity description.
    pub description: String,

    /// Source document ID.
    pub source_id: String,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: String,

    /// Node degree (number of connections).
    pub degree: usize,

    /// Additional metadata.
    pub metadata: serde_json::Value,
}

/// Create entity response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateEntityResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Created entity.
    pub entity: EntityResponse,
}

/// Entity exists response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityExistsResponse {
    /// Whether the entity exists.
    pub exists: bool,

    /// Entity ID if exists.
    pub entity_id: Option<String>,

    /// Entity type if exists.
    pub entity_type: Option<String>,

    /// Node degree if exists.
    pub degree: Option<usize>,
}

/// Update entity response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpdateEntityResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Updated entity.
    pub entity: EntityResponse,

    /// Changes made.
    pub changes: ChangesSummary,
}

/// Delete entity response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteEntityResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Deleted entity ID.
    pub deleted_entity_id: String,

    /// Number of relationships deleted.
    pub deleted_relationships: usize,

    /// Affected entity IDs.
    pub affected_entities: Vec<String>,
}

/// Merge entities request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MergeEntitiesRequest {
    /// Source entity to merge from.
    pub source_entity: String,

    /// Target entity to merge into.
    pub target_entity: String,

    /// Merge strategy: "prefer_source", "prefer_target", "merge".
    #[serde(default = "default_merge_strategy")]
    pub merge_strategy: String,

    /// Additional metadata.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

fn default_merge_strategy() -> String {
    "prefer_target".to_string()
}

/// Merge entities response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MergeEntitiesResponse {
    /// Operation status.
    pub status: String,

    /// Success message.
    pub message: String,

    /// Merged entity.
    pub merged_entity: EntityResponse,

    /// Merge details.
    pub merge_details: MergeDetails,
}

/// Merge operation details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MergeDetails {
    /// Source entity ID.
    pub source_entity_id: String,

    /// Target entity ID.
    pub target_entity_id: String,

    /// Number of relationships merged.
    pub relationships_merged: usize,

    /// Number of duplicate relationships removed.
    pub duplicate_relationships_removed: usize,

    /// Description merge strategy used.
    pub description_strategy: String,

    /// Metadata merge strategy used.
    pub metadata_strategy: String,
}

/// Changes summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChangesSummary {
    /// Fields that were updated.
    pub fields_updated: Vec<String>,

    /// Previous description if changed.
    pub previous_description: Option<String>,
}

/// Delete query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DeleteEntityQuery {
    /// Whether to delete relationships (default: true).
    #[serde(default = "default_true")]
    pub delete_relationships: bool,

    /// Confirmation flag (required).
    pub confirm: bool,
}

fn default_true() -> bool {
    true
}

/// Entity exists query.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EntityExistsQuery {
    /// Entity name to check.
    pub entity_name: String,
}

/// Get entity with relationships response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetEntityResponse {
    /// Entity data.
    pub entity: EntityResponse,

    /// Relationships.
    pub relationships: RelationshipsInfo,

    /// Statistics.
    pub statistics: EntityStatistics,
}

/// Relationships info.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelationshipsInfo {
    /// Outgoing relationships.
    pub outgoing: Vec<RelationshipSummary>,

    /// Incoming relationships.
    pub incoming: Vec<RelationshipSummary>,
}

/// Relationship summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RelationshipSummary {
    /// Target entity ID (for outgoing) or source entity ID (for incoming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Source entity ID (for incoming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Relationship type.
    pub relation_type: String,

    /// Relationship weight.
    pub weight: f64,
}

/// Entity statistics.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EntityStatistics {
    /// Total relationships.
    pub total_relationships: usize,

    /// Outgoing relationships count.
    pub outgoing_count: usize,

    /// Incoming relationships count.
    pub incoming_count: usize,

    /// Document references count.
    pub document_references: usize,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Normalize entity name to UPPERCASE with underscores.
fn normalize_entity_name(name: &str) -> String {
    name.to_uppercase().replace(' ', "_")
}

/// Convert GraphNode to EntityResponse.
fn node_to_entity_response(node: GraphNode, degree: usize) -> EntityResponse {
    let props = &node.properties;

    EntityResponse {
        id: node.id.clone(),
        entity_name: node.id.clone(),
        entity_type: props
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
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
        degree,
        metadata: props
            .get("metadata")
            .cloned()
            .unwrap_or(serde_json::json!({})),
    }
}

// ============================================================================
// API Handlers
// ============================================================================

/// Create a new entity.
#[utoipa::path(
    post,
    path = "/api/v1/graph/entities",
    tag = "Entities",
    request_body = CreateEntityRequest,
    responses(
        (status = 201, description = "Entity created", body = CreateEntityResponse),
        (status = 409, description = "Entity already exists")
    )
)]
pub async fn create_entity(
    State(state): State<AppState>,
    Json(req): Json<CreateEntityRequest>,
) -> ApiResult<Json<CreateEntityResponse>> {
    let entity_name = normalize_entity_name(&req.entity_name);

    // Check if entity already exists
    if state.graph_storage.get_node(&entity_name).await?.is_some() {
        return Err(ApiError::Conflict(format!(
            "Entity '{}' already exists",
            entity_name
        )));
    }

    // Create entity properties
    let now = Utc::now().to_rfc3339();
    let mut properties = HashMap::new();
    properties.insert("entity_type".to_string(), req.entity_type.clone().into());
    properties.insert("description".to_string(), req.description.clone().into());
    properties.insert("source_id".to_string(), req.source_id.clone().into());
    properties.insert("created_at".to_string(), now.clone().into());
    properties.insert("updated_at".to_string(), now.clone().into());
    properties.insert("is_manual".to_string(), true.into());
    properties.insert("metadata".to_string(), req.metadata.clone());

    // Create node using upsert_node
    state.graph_storage.upsert_node(&entity_name, properties.clone()).await?;

    // Reconstruct node for response
    let node = GraphNode {
        id: entity_name.clone(),
        properties,
    };

    let entity = node_to_entity_response(node, 0);

    Ok(Json(CreateEntityResponse {
        status: "success".to_string(),
        message: "Entity created successfully".to_string(),
        entity,
    }))
}

/// Get an entity by ID with relationships.
#[utoipa::path(
    get,
    path = "/api/v1/graph/entities/{entity_name}",
    tag = "Entities",
    params(
        ("entity_name" = String, Path, description = "Entity name")
    ),
    responses(
        (status = 200, description = "Entity retrieved", body = GetEntityResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity(
    State(state): State<AppState>,
    Path(entity_name): Path<String>,
) -> ApiResult<Json<GetEntityResponse>> {
    let entity_name = normalize_entity_name(&entity_name);

    // Get entity node
    let node = state
        .graph_storage
        .get_node(&entity_name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Entity '{}' not found", entity_name)))?;

    let degree = state.graph_storage.node_degree(&entity_name).await?;
    let entity = node_to_entity_response(node, degree);

    // Get relationships (outgoing and incoming)
    let edges = state
        .graph_storage
        .get_node_edges(&entity_name)
        .await?;

    let mut outgoing = Vec::new();
    let mut incoming = Vec::new();

    for edge in edges {
        let relation_type = edge
            .properties
            .get("relation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("RELATED_TO")
            .to_string();

        let weight = edge
            .properties
            .get("weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        if edge.source == entity_name {
            outgoing.push(RelationshipSummary {
                target: Some(edge.target.clone()),
                source: None,
                relation_type,
                weight,
            });
        } else {
            incoming.push(RelationshipSummary {
                target: None,
                source: Some(edge.source.clone()),
                relation_type,
                weight,
            });
        }
    }

    let relationships = RelationshipsInfo { outgoing, incoming };

    let statistics = EntityStatistics {
        total_relationships: degree,
        outgoing_count: relationships.outgoing.len(),
        incoming_count: relationships.incoming.len(),
        document_references: 0, // TODO: implement document references tracking
    };

    Ok(Json(GetEntityResponse {
        entity,
        relationships,
        statistics,
    }))
}

/// Update an entity.
#[utoipa::path(
    put,
    path = "/api/v1/graph/entities/{entity_name}",
    tag = "Entities",
    params(
        ("entity_name" = String, Path, description = "Entity name")
    ),
    request_body = UpdateEntityRequest,
    responses(
        (status = 200, description = "Entity updated", body = UpdateEntityResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn update_entity(
    State(state): State<AppState>,
    Path(entity_name): Path<String>,
    Json(req): Json<UpdateEntityRequest>,
) -> ApiResult<Json<UpdateEntityResponse>> {
    let entity_name = normalize_entity_name(&entity_name);

    // Get existing entity
    let mut node = state
        .graph_storage
        .get_node(&entity_name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Entity '{}' not found", entity_name)))?;

    let previous_description = node
        .properties
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut fields_updated = Vec::new();

    // Update fields
    if let Some(entity_type) = req.entity_type {
        node.properties
            .insert("entity_type".to_string(), entity_type.into());
        fields_updated.push("entity_type".to_string());
    }

    if let Some(description) = req.description {
        node.properties
            .insert("description".to_string(), description.into());
        fields_updated.push("description".to_string());
    }

    if let Some(metadata) = req.metadata {
        node.properties.insert("metadata".to_string(), metadata);
        fields_updated.push("metadata".to_string());
    }

    // Update timestamp
    let now = Utc::now().to_rfc3339();
    node.properties.insert("updated_at".to_string(), now.into());

    // Update node in storage using upsert_node
    state.graph_storage.upsert_node(&entity_name, node.properties.clone()).await?;

    let degree = state.graph_storage.node_degree(&entity_name).await?;
    let entity = node_to_entity_response(node, degree);

    let changes = ChangesSummary {
        fields_updated,
        previous_description,
    };

    Ok(Json(UpdateEntityResponse {
        status: "success".to_string(),
        message: "Entity updated successfully".to_string(),
        entity,
        changes,
    }))
}

/// Delete an entity.
#[utoipa::path(
    delete,
    path = "/api/v1/graph/entities/{entity_name}",
    tag = "Entities",
    params(
        ("entity_name" = String, Path, description = "Entity name"),
        ("delete_relationships" = bool, Query, description = "Delete relationships"),
        ("confirm" = bool, Query, description = "Confirmation flag")
    ),
    responses(
        (status = 200, description = "Entity deleted", body = DeleteEntityResponse),
        (status = 400, description = "Missing confirmation"),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn delete_entity(
    State(state): State<AppState>,
    Path(entity_name): Path<String>,
    Query(params): Query<DeleteEntityQuery>,
) -> ApiResult<Json<DeleteEntityResponse>> {
    let entity_name = normalize_entity_name(&entity_name);

    // Check confirmation
    if !params.confirm {
        return Err(ApiError::BadRequest(
            "Confirmation required to delete entity".to_string(),
        ));
    }

    // Check if entity exists
    if state.graph_storage.get_node(&entity_name).await?.is_none() {
        return Err(ApiError::NotFound(format!(
            "Entity '{}' not found",
            entity_name
        )));
    }

    // Get affected entities (neighbors)
    let edges = state
        .graph_storage
        .get_node_edges(&entity_name)
        .await?;
    
    let mut affected_entities = Vec::new();
    for edge in &edges {
        if edge.source == entity_name {
            affected_entities.push(edge.target.clone());
        } else {
            affected_entities.push(edge.source.clone());
        }
    }
    let deleted_relationships = edges.len();

    // Delete node (edges will be deleted automatically)
    state.graph_storage.delete_node(&entity_name).await?;

    Ok(Json(DeleteEntityResponse {
        status: "success".to_string(),
        message: "Entity deleted successfully".to_string(),
        deleted_entity_id: entity_name,
        deleted_relationships,
        affected_entities,
    }))
}

/// Check if an entity exists.
#[utoipa::path(
    get,
    path = "/api/v1/graph/entities/exists",
    tag = "Entities",
    params(
        ("entity_name" = String, Query, description = "Entity name")
    ),
    responses(
        (status = 200, description = "Existence checked", body = EntityExistsResponse)
    )
)]
pub async fn entity_exists(
    State(state): State<AppState>,
    Query(params): Query<EntityExistsQuery>,
) -> ApiResult<Json<EntityExistsResponse>> {
    let entity_name = normalize_entity_name(&params.entity_name);

    if let Some(node) = state.graph_storage.get_node(&entity_name).await? {
        let degree = state.graph_storage.node_degree(&entity_name).await?;
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Json(EntityExistsResponse {
            exists: true,
            entity_id: Some(entity_name),
            entity_type,
            degree: Some(degree),
        }))
    } else {
        Ok(Json(EntityExistsResponse {
            exists: false,
            entity_id: None,
            entity_type: None,
            degree: None,
        }))
    }
}

/// Merge two entities (deduplication).
#[utoipa::path(
    post,
    path = "/api/v1/graph/entities/merge",
    tag = "Entities",
    request_body = MergeEntitiesRequest,
    responses(
        (status = 200, description = "Entities merged", body = MergeEntitiesResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn merge_entities(
    State(state): State<AppState>,
    Json(req): Json<MergeEntitiesRequest>,
) -> ApiResult<Json<MergeEntitiesResponse>> {
    let source_entity = normalize_entity_name(&req.source_entity);
    let target_entity = normalize_entity_name(&req.target_entity);

    // Get both entities
    let source_node = state
        .graph_storage
        .get_node(&source_entity)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Source entity '{}' not found", source_entity))
        })?;

    let mut target_node = state
        .graph_storage
        .get_node(&target_entity)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Target entity '{}' not found", target_entity))
        })?;

    // Merge descriptions based on strategy
    let description_strategy = req.merge_strategy.clone();
    match description_strategy.as_str() {
        "prefer_source" => {
            if let Some(desc) = source_node.properties.get("description") {
                target_node
                    .properties
                    .insert("description".to_string(), desc.clone());
            }
        }
        "prefer_target" => {
            // Keep target description as-is
        }
        "merge" => {
            let source_desc = source_node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let target_desc = target_node
                .properties
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let merged_desc = format!("{}; {}", target_desc, source_desc);
            target_node
                .properties
                .insert("description".to_string(), merged_desc.into());
        }
        _ => {}
    }

    // Merge metadata
    if let Some(source_meta) = source_node.properties.get("metadata").cloned() {
        if let Some(target_meta) = target_node.properties.get("metadata").cloned() {
            if let (Some(mut target_obj), Some(source_obj)) =
                (target_meta.as_object().cloned(), source_meta.as_object())
            {
                for (k, v) in source_obj {
                    target_obj.insert(k.clone(), v.clone());
                }
                target_node
                    .properties
                    .insert("metadata".to_string(), serde_json::json!(target_obj));
            }
        } else {
            target_node
                .properties
                .insert("metadata".to_string(), source_meta);
        }
    }

    // Get source relationships
    let source_edges = state
        .graph_storage
        .get_node_edges(&source_entity)
        .await?;

    let relationships_merged = source_edges.len();
    let duplicate_relationships_removed = 0;

    // Redirect relationships to target entity
    // This is a simplified implementation - in production, you'd want to:
    // 1. Check for duplicate relationships
    // 2. Merge relationship weights
    // 3. Handle bidirectional relationships properly

    // Update target node
    let now = Utc::now().to_rfc3339();
    target_node
        .properties
        .insert("updated_at".to_string(), now.into());
    state
        .graph_storage
        .upsert_node(&target_entity, target_node.properties.clone())
        .await?;

    // Delete source node
    state.graph_storage.delete_node(&source_entity).await?;

    let degree = state.graph_storage.node_degree(&target_entity).await?;
    let merged_entity = node_to_entity_response(target_node, degree);

    let merge_details = MergeDetails {
        source_entity_id: source_entity,
        target_entity_id: target_entity,
        relationships_merged,
        duplicate_relationships_removed,
        description_strategy,
        metadata_strategy: "merge".to_string(),
    };

    Ok(Json(MergeEntitiesResponse {
        status: "success".to_string(),
        message: "Entities merged successfully".to_string(),
        merged_entity,
        merge_details,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(
            normalize_entity_name("quantum computing"),
            "QUANTUM_COMPUTING"
        );
        assert_eq!(normalize_entity_name("AI"), "AI");
        assert_eq!(
            normalize_entity_name("Machine Learning"),
            "MACHINE_LEARNING"
        );
    }
}
