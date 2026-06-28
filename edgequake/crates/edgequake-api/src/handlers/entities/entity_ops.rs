//! Advanced entity operations: exists check, merge, neighborhood traversal.
//!
//! @implements UC0101 (Explore Entity Neighborhood)
//! @implements FEAT0202 (Graph Traversal)

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;

use crate::error::{ApiError, ApiResult};
use crate::handlers::isolation::{
    filter_edges_by_tenant_context, load_node_for_tenant_context, stamp_tenant_context_properties,
};
use crate::middleware::TenantContext;
use crate::services::entity_merge::rewire_merged_entity_edges;
use crate::services::entity_neighborhood::build_entity_neighborhood;
use crate::state::AppState;

use super::{node_to_entity_response, normalize_entity_name_for_graph};
pub use crate::handlers::entities_types::{
    EntityExistsQuery, EntityExistsResponse, EntityNeighborhoodQuery, EntityNeighborhoodResponse,
    MergeDetails, MergeEntitiesRequest, MergeEntitiesResponse,
};

async fn resolve_entity_node(
    state: &AppState,
    entity_name: &str,
    ctx: &TenantContext,
) -> ApiResult<Option<edgequake_storage::GraphNode>> {
    let normalized_name = normalize_entity_name_for_graph(entity_name);

    if let Ok(node) =
        load_node_for_tenant_context(state.storage.graph_storage.as_ref(), &normalized_name, ctx)
            .await
    {
        return Ok(Some(node));
    }

    if normalized_name != entity_name {
        if let Ok(node) =
            load_node_for_tenant_context(state.storage.graph_storage.as_ref(), entity_name, ctx)
                .await
        {
            return Ok(Some(node));
        }
    }

    let search_results = state
        .storage
        .graph_storage
        .search_nodes(
            entity_name,
            10,
            None,
            ctx.tenant_id.as_deref(),
            ctx.workspace_id.as_deref(),
        )
        .await
        .unwrap_or_default();

    if let Some((node, _)) = search_results.iter().find(|(node, _)| {
        node.id.eq_ignore_ascii_case(entity_name) || node.id.eq_ignore_ascii_case(&normalized_name)
    }) {
        return Ok(Some(node.clone()));
    }

    Ok(search_results.into_iter().next().map(|(node, _)| node))
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
    tenant_ctx: TenantContext,
    Query(params): Query<EntityExistsQuery>,
) -> ApiResult<Json<EntityExistsResponse>> {
    if let Some(node) = resolve_entity_node(&state, &params.entity_name, &tenant_ctx).await? {
        let degree = state.storage.graph_storage.node_degree(&node.id).await?;
        let entity_type = node
            .properties
            .get("entity_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Json(EntityExistsResponse {
            exists: true,
            entity_id: Some(node.id),
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
    tenant_ctx: TenantContext,
    Json(req): Json<MergeEntitiesRequest>,
) -> ApiResult<Json<MergeEntitiesResponse>> {
    let source_node = resolve_entity_node(&state, &req.source_entity, &tenant_ctx)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Source entity '{}' not found", req.source_entity))
        })?;
    let source_entity = source_node.id.clone();

    let mut target_node = resolve_entity_node(&state, &req.target_entity, &tenant_ctx)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Target entity '{}' not found", req.target_entity))
        })?;
    let target_entity = target_node.id.clone();

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

    // Rewire source relationships to the target entity before deleting the source node.
    // WHY: A merge must preserve graph semantics, not just delete the duplicate label.
    let source_edges = filter_edges_by_tenant_context(
        state
            .storage
            .graph_storage
            .get_node_edges(&source_entity)
            .await?,
        &tenant_ctx,
    );

    let rewire_stats = rewire_merged_entity_edges(
        state.storage.graph_storage.as_ref(),
        &source_edges,
        &source_entity,
        &target_entity,
        &tenant_ctx,
    )
    .await?;

    // Update target node
    let now = Utc::now().to_rfc3339();
    target_node
        .properties
        .insert("updated_at".to_string(), now.into());
    stamp_tenant_context_properties(&mut target_node.properties, &tenant_ctx)?;
    state
        .storage
        .graph_storage
        .upsert_node(&target_entity, target_node.properties.clone())
        .await?;

    let deleted = state
        .storage
        .graph_storage
        .delete_node_scoped(&source_entity, tenant_id, workspace_id)
        .await?;

    if !deleted {
        return Err(ApiError::NotFound(format!(
            "Source entity '{}' not found",
            source_entity
        )));
    }

    let degree = state
        .storage
        .graph_storage
        .node_degree(&target_entity)
        .await?;
    let merged_entity = node_to_entity_response(target_node, degree);

    let merge_details = MergeDetails {
        source_entity_id: source_entity,
        target_entity_id: target_entity,
        relationships_merged: rewire_stats.relationships_merged,
        duplicate_relationships_removed: rewire_stats.duplicate_relationships_removed,
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
    tenant_ctx: TenantContext,
    Path(entity_name): Path<String>,
    Query(query): Query<EntityNeighborhoodQuery>,
) -> ApiResult<Json<EntityNeighborhoodResponse>> {
    let resolved_entity = resolve_entity_node(&state, &entity_name, &tenant_ctx)
        .await?
        .map(|node| node.id)
        .ok_or_else(|| ApiError::NotFound(format!("Entity '{}' not found", entity_name)))?;

    // Clamp depth to range [1, 3]
    let depth = query.depth.clamp(1, 3);

    let (nodes, edges) = build_entity_neighborhood(
        &state.storage.graph_storage,
        &tenant_ctx,
        &resolved_entity,
        depth,
    )
    .await?;

    Ok(Json(EntityNeighborhoodResponse { nodes, edges }))
}
