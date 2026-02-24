use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use super::helpers::{verify_workspace_tenant_access, workspace_to_response};
use crate::error::ApiError;
use crate::handlers::workspaces_types::*;
use crate::middleware::TenantContext;
use crate::state::AppState;

/// Create a new workspace.
///
/// POST /api/v1/tenants/{tenant_id}/workspaces
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/workspaces",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    request_body = CreateWorkspaceApiRequest,
    responses(
        (status = 201, description = "Workspace created", body = WorkspaceResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Tenant not found"),
        (status = 409, description = "Workspace with this slug already exists"),
    ),
    tags = ["workspaces"]
)]
pub async fn create_workspace(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<CreateWorkspaceApiRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), ApiError> {
    use edgequake_core::CreateWorkspaceRequest;

    // SPEC-032: Fetch parent tenant to inherit default model configuration if not provided
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", tenant_id)))?;

    // SPEC-032: Use tenant defaults if workspace-level config not provided
    let llm_model = request
        .llm_model
        .clone()
        .or_else(|| Some(tenant.default_llm_model.clone()));
    let llm_provider = request
        .llm_provider
        .clone()
        .or_else(|| Some(tenant.default_llm_provider.clone()));
    let embedding_model = request
        .embedding_model
        .clone()
        .or_else(|| Some(tenant.default_embedding_model.clone()));
    let embedding_provider = request
        .embedding_provider
        .clone()
        .or_else(|| Some(tenant.default_embedding_provider.clone()));
    let embedding_dimension = request
        .embedding_dimension
        .or(Some(tenant.default_embedding_dimension));

    // SPEC-041: Inherit default vision LLM from tenant if workspace doesn't specify one
    let vision_llm_model = request
        .vision_llm_model
        .clone()
        .or_else(|| tenant.default_vision_llm_model.clone());
    let vision_llm_provider = request
        .vision_llm_provider
        .clone()
        .or_else(|| tenant.default_vision_llm_provider.clone());

    // SPEC-032: Include LLM and embedding configuration in create request
    let create_request = CreateWorkspaceRequest {
        name: request.name.clone(),
        slug: request.slug.clone(),
        description: request.description.clone(),
        max_documents: request.max_documents,
        llm_model,
        llm_provider,
        embedding_model,
        embedding_provider,
        embedding_dimension,
        vision_llm_model,
        vision_llm_provider,
    };

    // Store workspace via workspace service
    let workspace = state
        .workspace_service
        .create_workspace(tenant_id, create_request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let response = workspace_to_response(&workspace);

    tracing::info!(
        workspace_id = %workspace.workspace_id,
        tenant_id = %tenant_id,
        llm_model = %workspace.llm_full_id(),
        embedding_model = %workspace.embedding_full_id(),
        inherited_from_tenant = request.llm_model.is_none(),
        "Created workspace"
    );

    Ok((StatusCode::CREATED, Json(response)))
}

/// List workspaces for a tenant.
///
/// GET /api/v1/tenants/{tenant_id}/workspaces
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/workspaces",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of workspaces", body = WorkspaceListResponse),
        (status = 404, description = "Tenant not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn list_workspaces(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<WorkspaceListResponse>, ApiError> {
    let limit = params.limit.min(100);

    tracing::debug!(tenant_id = %tenant_id, "Listing workspaces");

    let workspaces = state
        .workspace_service
        .list_workspaces(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<WorkspaceResponse> = workspaces
        .into_iter()
        .skip(params.offset)
        .take(limit)
        .map(|ws| workspace_to_response(&ws))
        .collect();

    let total = items.len();

    let response = WorkspaceListResponse {
        items,
        total,
        offset: params.offset,
        limit,
    };

    Ok(Json(response))
}

/// Get a workspace by ID.
///
/// GET /api/v1/workspaces/{workspace_id}
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Workspace found", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    // BR0201: verify workspace belongs to requesting tenant
    let workspace = verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    let response = workspace_to_response(&workspace);

    Ok(Json(response))
}

/// Get a workspace by slug (for URL-based routing).
///
/// GET /api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID"),
        ("slug" = String, Path, description = "Workspace slug")
    ),
    responses(
        (status = 200, description = "Workspace found", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_workspace_by_slug(
    State(state): State<AppState>,
    Path((tenant_id, slug)): Path<(Uuid, String)>,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    let workspace = state
        .workspace_service
        .get_workspace_by_slug(tenant_id, &slug)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace with slug '{}' not found", slug)))?;

    let response = workspace_to_response(&workspace);

    Ok(Json(response))
}

/// Update a workspace.
///
/// PUT /api/v1/workspaces/{workspace_id}
#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    request_body = UpdateWorkspaceApiRequest,
    responses(
        (status = 200, description = "Workspace updated", body = WorkspaceResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn update_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(request): Json<UpdateWorkspaceApiRequest>,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    use edgequake_core::UpdateWorkspaceRequest;

    // BR0201: verify workspace belongs to requesting tenant before mutating
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    // SPEC-032: Include LLM/embedding model configuration in update
    let update_request = UpdateWorkspaceRequest {
        name: request.name,
        description: request.description,
        is_active: request.is_active,
        max_documents: request.max_documents,
        llm_model: request.llm_model,
        llm_provider: request.llm_provider,
        embedding_model: request.embedding_model,
        embedding_provider: request.embedding_provider,
        embedding_dimension: request.embedding_dimension,
        // SPEC-040: Vision LLM configuration
        vision_llm_provider: request.vision_llm_provider,
        vision_llm_model: request.vision_llm_model,
    };

    let workspace = state
        .workspace_service
        .update_workspace(workspace_id, update_request)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;

    let response = workspace_to_response(&workspace);

    Ok(Json(response))
}

/// Delete a workspace and cascade delete all associated data.
///
/// # Implements
///
/// - **UC0304**: Delete Workspace
/// - **SPEC-028**: Workspace cascade delete
///
/// # Enforces
///
/// - **BR0821**: Workspace deletion cascades to all resources
///
/// # Cascade Order
///
/// ```text
/// 1. Clear vector storage (embeddings)
/// 2. Clear graph storage (entities/relationships)
/// 3. Delete document metadata and content from KV storage
/// 4. Evict workspace from vector registry cache
/// 5. Delete workspace record from database
/// ```
///
/// DELETE /api/v1/workspaces/{workspace_id}
#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{workspace_id}",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 204, description = "Workspace deleted with cascade"),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn delete_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> Result<StatusCode, ApiError> {
    // BR0201: verify workspace belongs to requesting tenant before cascade delete
    verify_workspace_tenant_access(&state, workspace_id, &tenant_ctx).await?;

    tracing::info!(workspace_id = %workspace_id, "Starting workspace cascade delete");

    let workspace_id_str = workspace_id.to_string();

    // 1. Clear vector storage for this workspace
    // WHY: Remove all embeddings (chunks + entities) before deleting workspace
    let vectors_cleared = match state.vector_storage.clear_workspace(&workspace_id).await {
        Ok(count) => {
            tracing::info!(workspace_id = %workspace_id, vectors_cleared = count, "Cleared vector storage");
            count
        }
        Err(e) => {
            tracing::warn!(workspace_id = %workspace_id, error = %e, "Failed to clear vector storage (continuing)");
            0
        }
    };

    // 2. Clear graph storage for this workspace (entities and relationships)
    // WHY: Remove all knowledge graph nodes and edges
    let (nodes_cleared, edges_cleared) = match state
        .graph_storage
        .clear_workspace(&workspace_id)
        .await
    {
        Ok((nodes, edges)) => {
            tracing::info!(
                workspace_id = %workspace_id,
                nodes_cleared = nodes,
                edges_cleared = edges,
                "Cleared graph storage"
            );
            (nodes, edges)
        }
        Err(e) => {
            tracing::warn!(workspace_id = %workspace_id, error = %e, "Failed to clear graph storage (continuing)");
            (0, 0)
        }
    };

    // 3. Delete all documents belonging to this workspace from KV storage
    // WHY: Remove document metadata, content, and chunk data
    let mut documents_deleted = 0;
    let mut chunks_deleted = 0;

    if let Ok(all_keys) = state.kv_storage.keys().await {
        // Find metadata keys and check workspace membership
        let metadata_keys: Vec<String> = all_keys
            .iter()
            .filter(|k| k.ends_with("-metadata"))
            .cloned()
            .collect();

        let mut keys_to_delete: Vec<String> = Vec::new();

        for key in metadata_keys {
            if let Ok(Some(metadata)) = state.kv_storage.get_by_id(&key).await {
                // Check if document belongs to this workspace
                let doc_workspace = metadata
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");

                if doc_workspace == workspace_id_str {
                    let doc_id = key.trim_end_matches("-metadata");

                    // Queue metadata key for deletion
                    keys_to_delete.push(key.clone());

                    // Queue content key for deletion
                    keys_to_delete.push(format!("{}-content", doc_id));

                    // Find and queue all chunk keys for this document
                    let chunk_prefix = format!("{}-chunk-", doc_id);
                    for chunk_key in all_keys.iter().filter(|k| k.starts_with(&chunk_prefix)) {
                        keys_to_delete.push(chunk_key.clone());
                        chunks_deleted += 1;
                    }

                    documents_deleted += 1;
                }
            }
        }

        // Delete all queued keys
        if !keys_to_delete.is_empty() {
            if let Err(e) = state.kv_storage.delete(&keys_to_delete).await {
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    keys_count = keys_to_delete.len(),
                    "Failed to delete some KV storage keys"
                );
            }
        }

        tracing::info!(
            workspace_id = %workspace_id,
            documents_deleted = documents_deleted,
            chunks_deleted = chunks_deleted,
            "Cleared KV storage"
        );
    }

    // 4. Evict workspace from vector registry cache
    // WHY: Ensure cached storage instances are cleaned up
    state.vector_registry.evict(&workspace_id).await;

    // 5. Finally delete the workspace record from database
    state
        .workspace_service
        .delete_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    tracing::info!(
        workspace_id = %workspace_id,
        vectors_cleared = vectors_cleared,
        nodes_cleared = nodes_cleared,
        edges_cleared = edges_cleared,
        documents_deleted = documents_deleted,
        chunks_deleted = chunks_deleted,
        "Workspace cascade delete completed"
    );

    Ok(StatusCode::NO_CONTENT)
}
