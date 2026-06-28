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
use edgequake_storage::traits::NodeListFilter;
use edgequake_storage::GraphNode;
use std::collections::HashMap;

use crate::error::{ApiError, ApiResult};
use crate::handlers::isolation::{
    filter_edges_by_tenant_context, load_node_for_tenant_context, properties_match_tenant_context,
    stamp_tenant_context_properties,
};
use crate::middleware::TenantContext;
use crate::state::AppState;

use super::{node_to_entity_response, normalize_entity_name_for_graph};
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
    // SPEC-006: BR-006-010 — page size from AppState resource SSOT
    let page_size = state.resource_budget().clamp_page_size(query.page_size);
    let page = query.page.max(1);
    let offset = ((page - 1) * page_size) as usize;

    // SPEC-006: TR-006-001 — push-down filtered pagination (no get_all_nodes)
    let filter = NodeListFilter {
        tenant_id: tenant_ctx.tenant_id.clone(),
        workspace_id: tenant_ctx.workspace_id.clone(),
        entity_type: query.entity_type.clone(),
        search: query.search.clone(),
        community_ids: None,
    };

    let page_result = state
        .storage
        .graph_storage
        .list_nodes_filtered(&filter, offset, page_size as usize)
        .await?;

    let total = page_result.total;
    let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;
    let page_nodes = page_result.items;

    // Convert to response format (SPEC-027 IMP-015: batch degree lookup)
    let node_ids: Vec<String> = page_nodes.iter().map(|node| node.id.clone()).collect();
    let degree_map: std::collections::HashMap<String, usize> = if node_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        state
            .storage
            .graph_storage
            .node_degrees_batch(&node_ids)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect()
    };

    let mut items = Vec::with_capacity(page_nodes.len());
    for node in page_nodes {
        let degree = degree_map.get(&node.id).copied().unwrap_or(0);
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
    let entity_name = normalize_entity_name_for_graph(&req.entity_name);

    // Check if entity already exists in this tenant/workspace
    if let Some(existing) = state.storage.graph_storage.get_node(&entity_name).await? {
        if properties_match_tenant_context(&existing.properties, &tenant_ctx) {
            return Err(ApiError::Conflict(format!(
                "Entity '{}' already exists",
                entity_name
            )));
        }
        // Node id collision across tenants — deny write (MERGE would IDOR-overwrite).
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

    stamp_tenant_context_properties(&mut properties, &tenant_ctx)?;

    // Create node using upsert_node
    state
        .storage
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
    tenant_ctx: TenantContext,
    Path(entity_name): Path<String>,
) -> ApiResult<Json<GetEntityResponse>> {
    let entity_name = normalize_entity_name_for_graph(&entity_name);

    let node = load_node_for_tenant_context(
        state.storage.graph_storage.as_ref(),
        &entity_name,
        &tenant_ctx,
    )
    .await?;

    let degree = state
        .storage
        .graph_storage
        .node_degree(&entity_name)
        .await?;
    let entity = node_to_entity_response(node, degree);

    // Get relationships (outgoing and incoming) — tenant-scoped
    let edges = filter_edges_by_tenant_context(
        state
            .storage
            .graph_storage
            .get_node_edges(&entity_name)
            .await?,
        &tenant_ctx,
    );

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
    tenant_ctx: TenantContext,
    Path(entity_name): Path<String>,
    Json(req): Json<UpdateEntityRequest>,
) -> ApiResult<Json<UpdateEntityResponse>> {
    let entity_name = normalize_entity_name_for_graph(&entity_name);

    let mut node = load_node_for_tenant_context(
        state.storage.graph_storage.as_ref(),
        &entity_name,
        &tenant_ctx,
    )
    .await?;

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

    // Preserve tenant scope (defense in depth — never strip on update)
    stamp_tenant_context_properties(&mut node.properties, &tenant_ctx)?;

    // Update node in storage using upsert_node
    state
        .storage
        .graph_storage
        .upsert_node(&entity_name, node.properties.clone())
        .await?;

    let degree = state
        .storage
        .graph_storage
        .node_degree(&entity_name)
        .await?;
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
    tenant_ctx: TenantContext,
    Path(entity_name): Path<String>,
    Query(params): Query<DeleteEntityQuery>,
) -> ApiResult<Json<DeleteEntityResponse>> {
    let entity_name = normalize_entity_name_for_graph(&entity_name);

    // Check confirmation
    if !params.confirm {
        return Err(ApiError::BadRequest(
            "Confirmation required to delete entity".to_string(),
        ));
    }

    let (tenant_id, workspace_id) = (
        tenant_ctx
            .tenant_id
            .as_deref()
            .ok_or_else(|| ApiError::BadRequest("Tenant context required".to_string()))?,
        tenant_ctx
            .workspace_id
            .as_deref()
            .ok_or_else(|| ApiError::BadRequest("Workspace context required".to_string()))?,
    );

    // Verify entity belongs to tenant before collecting edge metadata
    let _ = load_node_for_tenant_context(
        state.storage.graph_storage.as_ref(),
        &entity_name,
        &tenant_ctx,
    )
    .await?;

    let edges = filter_edges_by_tenant_context(
        state
            .storage
            .graph_storage
            .get_node_edges(&entity_name)
            .await?,
        &tenant_ctx,
    );

    let mut affected_entities = Vec::new();
    for edge in &edges {
        if edge.source == entity_name {
            affected_entities.push(edge.target.clone());
        } else {
            affected_entities.push(edge.source.clone());
        }
    }
    let deleted_relationships = edges.len();

    let deleted = state
        .storage
        .graph_storage
        .delete_node_scoped(&entity_name, tenant_id, workspace_id)
        .await?;

    if !deleted {
        return Err(ApiError::NotFound(format!(
            "Entity '{}' not found",
            entity_name
        )));
    }

    Ok(Json(DeleteEntityResponse {
        status: "success".to_string(),
        message: "Entity deleted successfully".to_string(),
        deleted_entity_id: entity_name,
        deleted_relationships,
        affected_entities,
    }))
}
