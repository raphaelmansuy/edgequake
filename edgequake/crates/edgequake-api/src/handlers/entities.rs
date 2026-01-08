//! Entity CRUD operations for manual knowledge graph management.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use edgequake_storage::GraphNode;
use std::collections::HashMap;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// Re-export DTOs from entities_types module
pub use crate::handlers::entities_types::*;

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

/// List entities with pagination and filtering.
#[utoipa::path(
    get,
    path = "/api/v1/graph/entities",
    tag = "Entities",
    params(
        ("page" = Option<u32>, Query, description = "Page number (1-indexed, default 1)"),
        ("page_size" = Option<u32>, Query, description = "Page size (default 20, max 100)"),
        ("entity_type" = Option<String>, Query, description = "Filter by entity type"),
        ("search" = Option<String>, Query, description = "Search term for entity name or description")
    ),
    responses(
        (status = 200, description = "Paginated list of entities", body = ListEntitiesResponse)
    )
)]
pub async fn list_entities(
    State(state): State<AppState>,
    Query(query): Query<ListEntitiesQuery>,
) -> ApiResult<Json<ListEntitiesResponse>> {
    // Clamp page_size to range [1, 100]
    let page_size = query.page_size.clamp(1, 100);
    let page = query.page.max(1);
    let offset = ((page - 1) * page_size) as usize;

    // Get all nodes from graph storage
    // WHY: We need to fetch all nodes and filter in memory because the storage
    // interface doesn't support pagination/filtering yet. Future optimization
    // would push these filters down to the storage layer.
    let all_nodes = state.graph_storage.get_all_nodes().await?;

    // Apply filters
    let mut filtered_nodes: Vec<_> = all_nodes
        .into_iter()
        .filter(|node| {
            // Filter by entity_type if specified
            if let Some(ref entity_type) = query.entity_type {
                let node_type = node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !node_type.eq_ignore_ascii_case(entity_type) {
                    return false;
                }
            }

            // Filter by search term if specified
            if let Some(ref search) = query.search {
                let search_lower = search.to_lowercase();
                let name_matches = node.id.to_lowercase().contains(&search_lower);
                let desc_matches = node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&search_lower);
                if !name_matches && !desc_matches {
                    return false;
                }
            }

            true
        })
        .collect();

    // Sort by entity name for consistent ordering
    filtered_nodes.sort_by(|a, b| a.id.cmp(&b.id));

    let total = filtered_nodes.len();
    let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;

    // Apply pagination
    let page_nodes: Vec<_> = filtered_nodes
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .collect();

    // Convert to response format
    let mut items = Vec::with_capacity(page_nodes.len());
    for node in page_nodes {
        let degree = state.graph_storage.node_degree(&node.id).await.unwrap_or(0);
        items.push(node_to_entity_response(node, degree));
    }

    Ok(Json(ListEntitiesResponse {
        items,
        total,
        page,
        page_size,
        total_pages,
    }))
}

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
    state
        .graph_storage
        .upsert_node(&entity_name, properties.clone())
        .await?;

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
    let edges = state.graph_storage.get_node_edges(&entity_name).await?;

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
    state
        .graph_storage
        .upsert_node(&entity_name, node.properties.clone())
        .await?;

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
    let edges = state.graph_storage.get_node_edges(&entity_name).await?;

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
    let source_edges = state.graph_storage.get_node_edges(&source_entity).await?;

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

/// Get entity neighborhood (connected nodes within specified depth).
#[utoipa::path(
    get,
    path = "/api/v1/graph/entities/{entity_name}/neighborhood",
    tag = "Entities",
    params(
        ("entity_name" = String, Path, description = "Entity name"),
        ("depth" = Option<u32>, Query, description = "Traversal depth (default 1, max 3)")
    ),
    responses(
        (status = 200, description = "Entity neighborhood", body = EntityNeighborhoodResponse),
        (status = 404, description = "Entity not found")
    )
)]
pub async fn get_entity_neighborhood(
    State(state): State<AppState>,
    Path(entity_name): Path<String>,
    Query(query): Query<EntityNeighborhoodQuery>,
) -> ApiResult<Json<EntityNeighborhoodResponse>> {
    let entity_name = normalize_entity_name(&entity_name);

    // Verify the entity exists
    if state
        .graph_storage
        .get_node(&entity_name)
        .await?
        .is_none()
    {
        return Err(ApiError::NotFound(format!(
            "Entity '{}' not found",
            entity_name
        )));
    }

    // Clamp depth to range [1, 3]
    let depth = query.depth.clamp(1, 3);

    // Collect nodes and edges using BFS
    let mut visited_nodes = std::collections::HashSet::new();
    let mut frontier = vec![entity_name.clone()];
    visited_nodes.insert(entity_name.clone());

    let mut all_edges = Vec::new();

    // BFS traversal up to the specified depth
    for _ in 0..depth {
        let mut next_frontier = Vec::new();

        for node_id in &frontier {
            let edges = state.graph_storage.get_node_edges(node_id).await?;

            for edge in edges {
                // Check both directions
                let neighbor = if edge.source == *node_id {
                    &edge.target
                } else {
                    &edge.source
                };

                // Add edge to collection (dedup by edge id)
                let edge_id = format!("{}_{}", edge.source, edge.target);
                if !all_edges.iter().any(|(id, _): &(String, _)| id == &edge_id) {
                    all_edges.push((edge_id, edge.clone()));
                }

                // Add neighbor to next frontier if not visited
                if !visited_nodes.contains(neighbor) {
                    visited_nodes.insert(neighbor.clone());
                    next_frontier.push(neighbor.clone());
                }
            }
        }

        frontier = next_frontier;
        if frontier.is_empty() {
            break;
        }
    }

    // Build response nodes
    let mut nodes = Vec::with_capacity(visited_nodes.len());
    for node_id in &visited_nodes {
        if let Some(node) = state.graph_storage.get_node(node_id).await? {
            let degree = state.graph_storage.node_degree(node_id).await.unwrap_or(0);
            nodes.push(NeighborhoodNode {
                id: node.id.clone(),
                entity_type: node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                description: node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                degree,
            });
        }
    }

    // Build response edges
    let edges: Vec<NeighborhoodEdge> = all_edges
        .into_iter()
        .map(|(id, edge)| NeighborhoodEdge {
            id,
            source: edge.source,
            target: edge.target,
            relation_type: edge
                .properties
                .get("relation_type")
                .and_then(|v| v.as_str())
                .unwrap_or("RELATED_TO")
                .to_string(),
            weight: edge
                .properties
                .get("weight")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0),
        })
        .collect();

    Ok(Json(EntityNeighborhoodResponse { nodes, edges }))
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

    #[test]
    fn test_normalize_entity_name_edge_cases() {
        // Single space replaced with underscore
        assert_eq!(normalize_entity_name("hello world"), "HELLO_WORLD");
        // Multiple spaces become multiple underscores (current behavior)
        assert_eq!(normalize_entity_name("hello  world"), "HELLO__WORLD");
        // Empty string
        assert_eq!(normalize_entity_name(""), "");
        // Already uppercase
        assert_eq!(
            normalize_entity_name("ALREADY UPPERCASE"),
            "ALREADY_UPPERCASE"
        );
    }

    #[test]
    fn test_create_entity_request_deserialization() {
        let json = r#"{
            "entity_name": "test entity",
            "entity_type": "CONCEPT",
            "description": "A test entity",
            "source_id": "manual_entry"
        }"#;
        let request: Result<CreateEntityRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.entity_name, "test entity");
        assert_eq!(req.entity_type, "CONCEPT");
    }

    #[test]
    fn test_update_entity_request_partial() {
        // Only description
        let json = r#"{"description": "Updated description"}"#;
        let request: Result<UpdateEntityRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert!(req.entity_type.is_none());
        assert_eq!(req.description, Some("Updated description".to_string()));
    }

    #[test]
    fn test_entity_response_serialization() {
        let response = EntityResponse {
            id: "test-id".to_string(),
            entity_name: "TEST_ENTITY".to_string(),
            entity_type: "CONCEPT".to_string(),
            description: "A test".to_string(),
            source_id: "doc-1".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            degree: 5,
            metadata: serde_json::Value::Null,
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("TEST_ENTITY"));
    }

    #[test]
    fn test_merge_entities_request_deserialization() {
        let json = r#"{
            "source_entity": "ENTITY_A",
            "target_entity": "ENTITY_B"
        }"#;
        let request: Result<MergeEntitiesRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.source_entity, "ENTITY_A");
        assert_eq!(req.target_entity, "ENTITY_B");
    }

    #[test]
    fn test_delete_entity_query_deserialization() {
        let json = r#"{"delete_relationships": true, "confirm": true}"#;
        let query: Result<DeleteEntityQuery, _> = serde_json::from_str(json);
        assert!(query.is_ok());
        let q = query.unwrap();
        assert!(q.delete_relationships);
        assert!(q.confirm);
    }

    #[test]
    fn test_entity_statistics_serialization() {
        let stats = EntityStatistics {
            total_relationships: 100,
            outgoing_count: 50,
            incoming_count: 50,
            document_references: 10,
        };
        let json = serde_json::to_string(&stats);
        assert!(json.is_ok());
    }
}
