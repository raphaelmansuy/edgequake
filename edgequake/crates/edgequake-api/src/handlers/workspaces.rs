//! Workspace and tenant management handlers.
//!
//! # Implements
//!
//! - **UC0301**: Create Workspace
//! - **UC0302**: List Workspaces
//! - **UC0303**: Switch Workspace
//! - **UC0304**: Delete Workspace
//! - **FEAT0701**: Multi-Tenancy Support
//! - **FEAT0702**: Workspace Isolation
//! - **FEAT0401**: REST API Service
//!
//! # Enforces
//!
//! - **BR0201**: Tenant isolation (all operations scoped to tenant)
//! - **BR0202**: Workspace quotas enforced by plan
//! - **BR0203**: Resource limits per workspace
//! - **BR0401**: Authentication required
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | POST | `/api/v1/tenants` | [`create_tenant`] | Create new tenant |
//! | GET | `/api/v1/tenants` | [`list_tenants`] | List all tenants |
//! | POST | `/api/v1/workspaces` | [`create_workspace`] | Create workspace |
//! | GET | `/api/v1/workspaces` | [`list_workspaces`] | List workspaces |
//! | DELETE | `/api/v1/workspaces/:id` | [`delete_workspace`] | Delete workspace |
//!
//! # WHY: Hierarchical Multi-Tenancy
//!
//! EdgeQuake uses a two-level hierarchy:
//! - **Tenant**: Organization/company level (billing, limits, users)
//! - **Workspace**: Project/team level (isolated knowledge graphs)
//!
//! This enables:
//! - SaaS deployment with multiple customers
//! - Per-project knowledge isolation
//! - Usage tracking and billing per tenant

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

// Re-export DTOs for backward compatibility
pub use crate::handlers::workspaces_types::{
    workspaces_default_limit, CreateTenantRequest, CreateWorkspaceApiRequest, PaginationParams,
    RebuildEmbeddingsRequest, RebuildEmbeddingsResponse, TenantListResponse, TenantResponse,
    UpdateTenantRequest, UpdateWorkspaceApiRequest, WorkspaceListResponse, WorkspaceResponse,
    WorkspaceStatsResponse,
};

use edgequake_core::Workspace;

// ============ Helper Functions ============

/// Convert a Workspace domain object to WorkspaceResponse DTO.
///
/// WHY: Centralized conversion ensures all model config fields are always included.
/// This supports SPEC-032 (Ollama/LM Studio provider integration).
fn workspace_to_response(workspace: &Workspace) -> WorkspaceResponse {
    WorkspaceResponse {
        id: workspace.workspace_id,
        tenant_id: workspace.tenant_id,
        name: workspace.name.clone(),
        slug: workspace.slug.clone(),
        description: workspace.description.clone(),
        is_active: workspace.is_active,
        max_documents: workspace.max_documents(),
        // SPEC-032: LLM configuration
        llm_model: workspace.llm_model.clone(),
        llm_provider: workspace.llm_provider.clone(),
        llm_full_id: workspace.llm_full_id(),
        // SPEC-032: Embedding configuration
        embedding_model: workspace.embedding_model.clone(),
        embedding_provider: workspace.embedding_provider.clone(),
        embedding_dimension: workspace.embedding_dimension,
        embedding_full_id: workspace.embedding_full_id(),
        created_at: workspace.created_at.to_rfc3339(),
        updated_at: workspace.updated_at.to_rfc3339(),
    }
}

// ============ Tenant Handlers ============

/// Create a new tenant (organization).
///
/// # Implements
///
/// - **FEAT0701**: Multi-Tenancy Support
///
/// # Enforces
///
/// - **BR0401**: Admin authentication required
///
/// POST /api/v1/tenants
#[utoipa::path(
    post,
    path = "/api/v1/tenants",
    request_body = CreateTenantRequest,
    responses(
        (status = 201, description = "Tenant created", body = TenantResponse),
        (status = 400, description = "Invalid request"),
        (status = 409, description = "Tenant with this slug already exists"),
    ),
    tags = ["tenants"]
)]
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(request): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>), ApiError> {
    use edgequake_core::{Tenant, TenantPlan};

    let slug = request.slug.unwrap_or_else(|| generate_slug(&request.name));

    let plan = match request.plan.as_deref() {
        Some("basic") => TenantPlan::Basic,
        Some("pro") => TenantPlan::Pro,
        Some("enterprise") => TenantPlan::Enterprise,
        _ => TenantPlan::Free,
    };

    let mut tenant = Tenant::new(&request.name, &slug).with_plan(plan);

    if let Some(desc) = request.description.as_ref() {
        tenant = tenant.with_description(desc);
    }

    // SPEC-032: Apply LLM configuration if provided
    if let (Some(model), Some(provider)) =
        (&request.default_llm_model, &request.default_llm_provider)
    {
        tenant = tenant.with_llm_config(model, provider);
    } else if let Some(model) = &request.default_llm_model {
        // Auto-detect provider from model name
        let provider = edgequake_core::Workspace::detect_provider_from_model(model);
        tenant = tenant.with_llm_config(model, provider);
    }

    // SPEC-032: Apply embedding configuration if provided
    if let (Some(model), Some(provider), Some(dimension)) = (
        &request.default_embedding_model,
        &request.default_embedding_provider,
        request.default_embedding_dimension,
    ) {
        tenant = tenant.with_embedding_config(model, provider, dimension);
    } else if let Some(model) = &request.default_embedding_model {
        // Auto-detect provider and dimension from model name
        let provider = edgequake_core::Workspace::detect_provider_from_model(model);
        let dimension = edgequake_core::Workspace::detect_dimension_from_model(model);
        let final_provider = request
            .default_embedding_provider
            .clone()
            .unwrap_or(provider);
        let final_dimension = request.default_embedding_dimension.unwrap_or(dimension);
        tenant = tenant.with_embedding_config(model, final_provider, final_dimension);
    }

    // Store tenant via workspace service
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Auto-create a default workspace for the new tenant (R004)
    // This ensures users always have at least one workspace available
    // SPEC-032: Workspace inherits tenant's default model configuration
    let default_workspace_request =
        edgequake_core::CreateWorkspaceRequest::new("Default Workspace")
            .with_llm_config(
                &created_tenant.default_llm_model,
                &created_tenant.default_llm_provider,
            )
            .with_embedding_config(
                &created_tenant.default_embedding_model,
                &created_tenant.default_embedding_provider,
                created_tenant.default_embedding_dimension,
            );

    if let Err(e) = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, default_workspace_request)
        .await
    {
        tracing::warn!(
            tenant_id = %created_tenant.tenant_id,
            error = %e,
            "Failed to auto-create default workspace"
        );
        // Continue anyway - tenant was created successfully
    } else {
        tracing::info!(
            tenant_id = %created_tenant.tenant_id,
            default_llm = %format!("{}/{}", created_tenant.default_llm_provider, created_tenant.default_llm_model),
            default_embedding = %format!("{}/{}", created_tenant.default_embedding_provider, created_tenant.default_embedding_model),
            "Auto-created default workspace for tenant with model config"
        );
    }

    let response = TenantResponse {
        id: created_tenant.tenant_id,
        name: created_tenant.name.clone(),
        slug: created_tenant.slug.clone(),
        plan: format!("{}", created_tenant.plan),
        is_active: created_tenant.is_active,
        max_workspaces: created_tenant.max_workspaces,
        default_llm_model: created_tenant.default_llm_model.clone(),
        default_llm_provider: created_tenant.default_llm_provider.clone(),
        default_llm_full_id: format!(
            "{}/{}",
            created_tenant.default_llm_provider, created_tenant.default_llm_model
        ),
        default_embedding_model: created_tenant.default_embedding_model.clone(),
        default_embedding_provider: created_tenant.default_embedding_provider.clone(),
        default_embedding_dimension: created_tenant.default_embedding_dimension,
        default_embedding_full_id: format!(
            "{}/{}",
            created_tenant.default_embedding_provider, created_tenant.default_embedding_model
        ),
        created_at: created_tenant.created_at.to_rfc3339(),
        updated_at: created_tenant.updated_at.to_rfc3339(),
    };

    tracing::info!(
        tenant_id = %created_tenant.tenant_id,
        default_llm = %response.default_llm_full_id,
        default_embedding = %response.default_embedding_full_id,
        "Created tenant with model configuration"
    );
    Ok((StatusCode::CREATED, Json(response)))
}

/// List all tenants.
///
/// GET /api/v1/tenants
#[utoipa::path(
    get,
    path = "/api/v1/tenants",
    params(PaginationParams),
    responses(
        (status = 200, description = "List of tenants", body = TenantListResponse),
    ),
    tags = ["tenants"]
)]
pub async fn list_tenants(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<TenantListResponse>, ApiError> {
    let limit = params.limit.min(100);

    let tenants = state
        .workspace_service
        .list_tenants(limit, params.offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let items: Vec<TenantResponse> = tenants
        .into_iter()
        .map(|t| TenantResponse {
            id: t.tenant_id,
            name: t.name.clone(),
            slug: t.slug.clone(),
            plan: format!("{}", t.plan),
            is_active: t.is_active,
            max_workspaces: t.max_workspaces,
            default_llm_model: t.default_llm_model.clone(),
            default_llm_provider: t.default_llm_provider.clone(),
            default_llm_full_id: format!("{}/{}", t.default_llm_provider, t.default_llm_model),
            default_embedding_model: t.default_embedding_model.clone(),
            default_embedding_provider: t.default_embedding_provider.clone(),
            default_embedding_dimension: t.default_embedding_dimension,
            default_embedding_full_id: format!(
                "{}/{}",
                t.default_embedding_provider, t.default_embedding_model
            ),
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
        })
        .collect();

    let total = items.len();

    let response = TenantListResponse {
        items,
        total,
        offset: params.offset,
        limit,
    };

    Ok(Json(response))
}

/// Get a tenant by ID.
///
/// GET /api/v1/tenants/{tenant_id}
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 200, description = "Tenant found", body = TenantResponse),
        (status = 404, description = "Tenant not found"),
    ),
    tags = ["tenants"]
)]
pub async fn get_tenant(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantResponse>, ApiError> {
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", tenant_id)))?;

    let response = TenantResponse {
        id: tenant.tenant_id,
        name: tenant.name.clone(),
        slug: tenant.slug.clone(),
        plan: format!("{}", tenant.plan),
        is_active: tenant.is_active,
        max_workspaces: tenant.max_workspaces,
        default_llm_model: tenant.default_llm_model.clone(),
        default_llm_provider: tenant.default_llm_provider.clone(),
        default_llm_full_id: format!(
            "{}/{}",
            tenant.default_llm_provider, tenant.default_llm_model
        ),
        default_embedding_model: tenant.default_embedding_model.clone(),
        default_embedding_provider: tenant.default_embedding_provider.clone(),
        default_embedding_dimension: tenant.default_embedding_dimension,
        default_embedding_full_id: format!(
            "{}/{}",
            tenant.default_embedding_provider, tenant.default_embedding_model
        ),
        created_at: tenant.created_at.to_rfc3339(),
        updated_at: tenant.updated_at.to_rfc3339(),
    };

    Ok(Json(response))
}

/// Update a tenant.
///
/// PUT /api/v1/tenants/{tenant_id}
#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    request_body = UpdateTenantRequest,
    responses(
        (status = 200, description = "Tenant updated", body = TenantResponse),
        (status = 404, description = "Tenant not found"),
    ),
    tags = ["tenants"]
)]
pub async fn update_tenant(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, ApiError> {
    // Get existing tenant
    let mut tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Tenant {} not found", tenant_id)))?;

    // Apply updates
    if let Some(name) = request.name {
        tenant.name = name;
    }
    if let Some(description) = request.description {
        tenant.description = Some(description);
    }
    if let Some(is_active) = request.is_active {
        tenant.is_active = is_active;
    }
    if let Some(plan_str) = request.plan {
        tenant.plan = plan_str.parse().unwrap_or(tenant.plan);
    }
    tenant.updated_at = chrono::Utc::now();

    // Save updated tenant
    let updated = state
        .workspace_service
        .update_tenant(tenant)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = TenantResponse {
        id: updated.tenant_id,
        name: updated.name.clone(),
        slug: updated.slug.clone(),
        plan: format!("{}", updated.plan),
        is_active: updated.is_active,
        max_workspaces: updated.max_workspaces,
        default_llm_model: updated.default_llm_model.clone(),
        default_llm_provider: updated.default_llm_provider.clone(),
        default_llm_full_id: format!(
            "{}/{}",
            updated.default_llm_provider, updated.default_llm_model
        ),
        default_embedding_model: updated.default_embedding_model.clone(),
        default_embedding_provider: updated.default_embedding_provider.clone(),
        default_embedding_dimension: updated.default_embedding_dimension,
        default_embedding_full_id: format!(
            "{}/{}",
            updated.default_embedding_provider, updated.default_embedding_model
        ),
        created_at: updated.created_at.to_rfc3339(),
        updated_at: updated.updated_at.to_rfc3339(),
    };

    Ok(Json(response))
}

/// Delete a tenant.
///
/// DELETE /api/v1/tenants/{tenant_id}
#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{tenant_id}",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    responses(
        (status = 204, description = "Tenant deleted"),
        (status = 404, description = "Tenant not found"),
    ),
    tags = ["tenants"]
)]
pub async fn delete_tenant(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(tenant_id = %tenant_id, "Deleting tenant");

    state
        .workspace_service
        .delete_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ============ Workspace Handlers ============

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
) -> Result<Json<WorkspaceResponse>, ApiError> {
    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

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
    Json(request): Json<UpdateWorkspaceApiRequest>,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    use edgequake_core::UpdateWorkspaceRequest;

    let update_request = UpdateWorkspaceRequest {
        name: request.name,
        description: request.description,
        is_active: request.is_active,
        max_documents: request.max_documents,
    };

    let workspace = state
        .workspace_service
        .update_workspace(workspace_id, update_request)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;

    let response = workspace_to_response(&workspace);

    Ok(Json(response))
}

/// Delete a workspace.
///
/// DELETE /api/v1/workspaces/{workspace_id}
#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{workspace_id}",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 204, description = "Workspace deleted"),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn delete_workspace(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(workspace_id = %workspace_id, "Deleting workspace");

    state
        .workspace_service
        .delete_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get workspace statistics.
///
/// GET /api/v1/workspaces/{workspace_id}/stats
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/stats",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Workspace statistics", body = WorkspaceStatsResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_workspace_stats(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceStatsResponse>, ApiError> {
    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = WorkspaceStatsResponse {
        workspace_id: stats.workspace_id,
        document_count: stats.document_count,
        entity_count: stats.entity_count,
        relationship_count: stats.relationship_count,
        chunk_count: stats.chunk_count,
        storage_bytes: stats.storage_bytes as u64,
    };

    Ok(Json(response))
}

// ============================================================================
// SPEC-032: Rebuild Embeddings Endpoint
// ============================================================================

/// Rebuild workspace embeddings with a new model.
///
/// This endpoint clears all vector embeddings for a workspace and optionally
/// updates the embedding model configuration. Documents will need to be
/// re-processed to regenerate embeddings.
///
/// ## Use Cases
///
/// - Changing embedding model (e.g., OpenAI → Ollama)
/// - Upgrading to a better embedding model
/// - Fixing corrupted embeddings
/// - Resetting after provider issues
///
/// ## Implementation Notes
///
/// Current implementation is **synchronous** and clears vectors immediately.
/// Future versions will support async background re-embedding.
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/rebuild-embeddings",
    request_body = RebuildEmbeddingsRequest,
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Rebuild started", body = RebuildEmbeddingsResponse),
        (status = 404, description = "Workspace not found"),
        (status = 400, description = "Invalid request"),
    ),
    tags = ["workspaces"]
)]
pub async fn rebuild_embeddings(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<RebuildEmbeddingsRequest>,
) -> Result<Json<RebuildEmbeddingsResponse>, ApiError> {
    use tracing::info;

    // 1. Get the workspace
    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

    // 2. Get workspace stats to count documents
    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // 3. Determine new embedding config
    let new_model = request
        .embedding_model
        .clone()
        .unwrap_or_else(|| workspace.embedding_model.clone());
    let new_provider = request
        .embedding_provider
        .clone()
        .unwrap_or_else(|| workspace.embedding_provider.clone());
    let new_dimension = request
        .embedding_dimension
        .unwrap_or(workspace.embedding_dimension);

    // 4. Check if config is actually changing
    let config_changed = new_model != workspace.embedding_model
        || new_provider != workspace.embedding_provider
        || new_dimension != workspace.embedding_dimension;

    if !config_changed && !request.force {
        return Err(ApiError::BadRequest(
            "Embedding configuration unchanged. Use 'force: true' to rebuild anyway.".to_string(),
        ));
    }

    info!(
        workspace_id = %workspace_id,
        old_model = %workspace.embedding_model,
        new_model = %new_model,
        old_dimension = workspace.embedding_dimension,
        new_dimension = new_dimension,
        document_count = stats.document_count,
        "Starting embedding rebuild"
    );

    // 5. Clear vector storage for this workspace
    // Note: The vector storage is namespaced by workspace, so clear() only affects this workspace
    let vectors_cleared = state.vector_storage.count().await.unwrap_or(0);
    state
        .vector_storage
        .clear()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to clear vectors: {}", e)))?;

    info!(
        workspace_id = %workspace_id,
        vectors_cleared = vectors_cleared,
        "Vector storage cleared"
    );

    // 6. Update workspace embedding config (if changed)
    // TODO: Implement workspace_service.update_embedding_config() for full support
    // For now, we just clear vectors and the new config takes effect on next document ingestion

    // 7. Build response
    // Estimate: ~1 second per document for embedding (conservative)
    let estimated_time = if stats.document_count > 0 {
        Some(stats.document_count as u64)
    } else {
        None
    };

    let response = RebuildEmbeddingsResponse {
        workspace_id,
        status: "vectors_cleared".to_string(),
        documents_to_process: stats.document_count,
        vectors_cleared,
        embedding_model: new_model,
        embedding_provider: new_provider,
        embedding_dimension: new_dimension,
        estimated_time_seconds: estimated_time,
        job_id: None, // No async job yet
    };

    info!(
        workspace_id = %workspace_id,
        status = %response.status,
        documents = stats.document_count,
        "Embedding rebuild complete (vectors cleared, documents need reprocessing)"
    );

    Ok(Json(response))
}

// ============ Helper Functions ============

/// Generate a URL-friendly slug from a name.
fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_slug() {
        assert_eq!(generate_slug("My Knowledge Base"), "my-knowledge-base");
        assert_eq!(generate_slug("Test 123!"), "test-123");
        assert_eq!(generate_slug("  multiple   spaces  "), "multiple-spaces");
    }

    #[test]
    fn test_generate_slug_edge_cases() {
        assert_eq!(generate_slug(""), "");
        assert_eq!(generate_slug("UPPERCASE"), "uppercase");
        assert_eq!(generate_slug("already-slug"), "already-slug");
        assert_eq!(generate_slug("123"), "123");
    }

    #[test]
    fn test_create_tenant_request_deserialization() {
        let json = r#"{"name": "Test Tenant"}"#;
        let request: Result<CreateTenantRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.name, "Test Tenant");
        assert!(req.slug.is_none());
        assert!(req.plan.is_none());
    }

    #[test]
    fn test_update_tenant_request_partial() {
        let json = r#"{"name": "Updated Name"}"#;
        let request: Result<UpdateTenantRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.name, Some("Updated Name".to_string()));
        assert!(req.is_active.is_none());
    }

    #[test]
    fn test_create_workspace_request_deserialization() {
        let json = r#"{"name": "Test Workspace", "description": "A test workspace"}"#;
        let request: Result<CreateWorkspaceApiRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.name, "Test Workspace");
        assert_eq!(req.description, Some("A test workspace".to_string()));
    }

    #[test]
    fn test_pagination_params_defaults() {
        let json = r#"{}"#;
        let params: Result<PaginationParams, _> = serde_json::from_str(json);
        assert!(params.is_ok());
        let p = params.unwrap();
        // Default values from serde(default)
        assert_eq!(p.offset, 0);
        assert_eq!(p.limit, 20);
    }

    #[test]
    fn test_tenant_response_serialization() {
        let response = TenantResponse {
            id: Uuid::new_v4(),
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            plan: "free".to_string(),
            is_active: true,
            max_workspaces: 5,
            default_llm_model: "gemma3:12b".to_string(),
            default_llm_provider: "ollama".to_string(),
            default_llm_full_id: "ollama/gemma3:12b".to_string(),
            default_embedding_model: "text-embedding-3-small".to_string(),
            default_embedding_provider: "openai".to_string(),
            default_embedding_dimension: 1536,
            default_embedding_full_id: "openai/text-embedding-3-small".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("test-tenant"));
        assert!(json_str.contains("gemma3:12b"));
        assert!(json_str.contains("text-embedding-3-small"));
    }

    #[test]
    fn test_workspace_stats_response_serialization() {
        let response = WorkspaceStatsResponse {
            workspace_id: Uuid::new_v4(),
            document_count: 100,
            entity_count: 500,
            relationship_count: 200,
            chunk_count: 1000,
            storage_bytes: 1024 * 1024,
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("\"document_count\":100"));
    }
}
