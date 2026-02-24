//! Entity CRUD handlers: list, create, get, update, delete.
//!
//! @implements UC0102 (Search Entities by Name)
//! @implements UC0103 (Delete Entity from Graph)
//! @implements FEAT0203 (Graph Mutation Operations)
//! @implements BR0201 (Tenant isolation)

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use edgequake_storage::GraphNode;
use std::collections::HashMap;

use crate::error::{ApiError, ApiResult};
use crate::handlers::isolation::filter_nodes_by_tenant_context;
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::{node_to_entity_response, normalize_entity_name};
pub use crate::handlers::entities_types::{
    ChangesSummary, CreateEntityRequest, CreateEntityResponse, DeleteEntityQuery,
    DeleteEntityResponse, EntityStatistics, GetEntityResponse, ListEntitiesQuery,
    ListEntitiesResponse, RelationshipSummary, RelationshipsInfo, UpdateEntityRequest,
    UpdateEntityResponse,
};

/// List entities with pagination and filtering.
///
/// # Implements
///
/// - **BR0201**: Tenant isolation (entities filtered by tenant/workspace context)
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
    tenant_ctx: TenantContext,
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

    // WHY: Apply tenant isolation first to ensure multi-tenancy is respected
    // This filters out nodes belonging to other tenants/workspaces
    let tenant_filtered_nodes = filter_nodes_by_tenant_context(all_nodes, &tenant_ctx);

    // Apply additional filters (entity_type, search)
    let mut filtered_nodes: Vec<_> = tenant_filtered_nodes
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
///
/// # Implements
///
/// - **BR0201**: Tenant isolation (entity created with tenant/workspace context)
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
    tenant_ctx: TenantContext,
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

    // WHY: Add tenant context to isolate entity to the current tenant/workspace
    if let Some(ref tenant_id) = tenant_ctx.tenant_id {
        properties.insert("tenant_id".to_string(), tenant_id.clone().into());
    }
    if let Some(ref workspace_id) = tenant_ctx.workspace_id {
        properties.insert("workspace_id".to_string(), workspace_id.clone().into());
    }

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
