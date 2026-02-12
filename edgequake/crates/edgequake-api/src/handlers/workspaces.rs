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
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

// ============ Stats Cache ============

/// Cached workspace stats with timestamp
#[derive(Clone)]
struct CachedStats {
    stats: WorkspaceStatsResponse,
    cached_at: Instant,
}

/// Thread-safe cache for workspace stats with TTL
///
/// WHY: Workspace stats queries can be expensive (15ms for KV storage, 1-5ms for PostgreSQL)
/// Dashboard frequently polls stats (every 30s), so caching provides:
/// - 100x faster response for cache hits (<1ms)
/// - Reduced load on storage backends
/// - Better UX with instant dashboard loads
///
/// TTL: 60 seconds (acceptable staleness for dashboard statistics)
type StatsCache = Arc<RwLock<HashMap<Uuid, CachedStats>>>;

lazy_static::lazy_static! {
    static ref WORKSPACE_STATS_CACHE: StatsCache = Arc::new(RwLock::new(HashMap::new()));
}

const STATS_CACHE_TTL: Duration = Duration::from_secs(60);

// Re-export DTOs for backward compatibility
pub use crate::handlers::workspaces_types::{
    workspaces_default_limit, CreateTenantRequest, CreateWorkspaceApiRequest,
    MetricsHistoryResponse, MetricsSnapshotDTO, PaginationParams, RebuildEmbeddingsRequest,
    RebuildEmbeddingsResponse, RebuildKnowledgeGraphRequest, RebuildKnowledgeGraphResponse,
    ReprocessAllRequest, ReprocessAllResponse, TenantListResponse, TenantResponse,
    UpdateTenantRequest, UpdateWorkspaceApiRequest, WorkspaceListResponse, WorkspaceResponse,
    WorkspaceStatsResponse,
};

use edgequake_core::{MetricsTriggerType, Workspace};

// ============ Helper Functions ============

/// Invalidate workspace stats cache entry.
///
/// WHY: After document upload/processing completes, the cached stats become stale.
/// Without invalidation, the dashboard will show old entity/relationship counts
/// until the 60-second TTL expires. This fixes the Dashboard showing 0 entities
/// while the Workspace page shows correct counts after processing.
pub async fn invalidate_workspace_stats_cache(workspace_id: Uuid) {
    let mut cache = WORKSPACE_STATS_CACHE.write().await;
    cache.remove(&workspace_id);
    tracing::debug!(
        workspace_id = %workspace_id,
        "Invalidated workspace stats cache after document processing"
    );
}

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
) -> Result<StatusCode, ApiError> {
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
    // HYBRID APPROACH WITH CACHING: 4-tier performance optimization
    // See: logs/2026-01-26-18-00-storage-architecture-analysis.md
    //
    // Performance tiers:
    // 0. Cache (<1ms) - FASTEST, 60s TTL
    // 1. PostgreSQL documents table (1-5ms) - Fast but currently empty
    // 2. KV storage aggregation (15ms) - Moderate, current data source
    // 3. AGE graph queries (50-200ms) - Slowest, last resort

    use std::time::Instant;
    let start = Instant::now();

    // Tier 0: Check cache first (fastest path - <1ms)
    {
        let cache = WORKSPACE_STATS_CACHE.read().await;
        if let Some(cached) = cache.get(&workspace_id) {
            if cached.cached_at.elapsed() < STATS_CACHE_TTL {
                let elapsed = start.elapsed();
                tracing::debug!(
                    workspace_id = %workspace_id,
                    duration_us = elapsed.as_micros(),
                    method = "cache",
                    age_secs = cached.cached_at.elapsed().as_secs(),
                    "Workspace stats retrieved from cache (fastest path)"
                );
                return Ok(Json(cached.stats.clone()));
            }
        }
    }

    // Cache miss - fetch from storage
    let stats = fetch_workspace_stats_uncached(&state, workspace_id, start).await?;

    // Update cache for next request
    {
        let mut cache = WORKSPACE_STATS_CACHE.write().await;
        cache.insert(
            workspace_id,
            CachedStats {
                stats: stats.clone(),
                cached_at: Instant::now(),
            },
        );
    }

    Ok(Json(stats))
}

/// Fetch workspace stats from storage backends (uncached).
///
/// This implements the hybrid fallback strategy across storage tiers.
async fn fetch_workspace_stats_uncached(
    state: &AppState,
    workspace_id: Uuid,
    start: Instant,
) -> Result<WorkspaceStatsResponse, ApiError> {
    // Try Method 1: PostgreSQL documents table (fastest if populated)
    if let Ok(mut stats) = try_postgres_stats(state, workspace_id).await {
        if stats.document_count > 0 {
            // Enrich with entity_type_count from graph storage
            // (PostgreSQL path doesn't have this in the documents table)
            stats.entity_type_count = state
                .graph_storage
                .distinct_node_type_count_by_workspace(&workspace_id)
                .await
                .unwrap_or(0);

            let elapsed = start.elapsed();
            tracing::info!(
                workspace_id = %workspace_id,
                duration_ms = elapsed.as_millis(),
                method = "postgresql",
                "Workspace stats retrieved from PostgreSQL"
            );
            return Ok(stats);
        }
    }

    // Method 2: KV storage aggregation (moderate speed, reliable)
    // This is the current source of truth since PostgreSQL tables are empty
    let stats = try_kv_storage_stats(state, workspace_id).await?;
    let elapsed = start.elapsed();
    tracing::info!(
        workspace_id = %workspace_id,
        duration_ms = elapsed.as_millis(),
        method = "kv_storage",
        "Workspace stats retrieved from KV storage"
    );
    Ok(stats)
}

/// Try to get stats from PostgreSQL documents table (fastest path).
///
/// This will fail if the documents table is empty (current state) but provides
/// the fastest query path once the pipeline is updated to populate it.
async fn try_postgres_stats(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<WorkspaceStatsResponse, ApiError> {
    // WHY: Call workspace_service which has access to PgPool
    // This uses the existing service layer with optimized SQL queries
    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("PostgreSQL stats query failed: {}", e)))?;

    Ok(WorkspaceStatsResponse {
        workspace_id: stats.workspace_id,
        document_count: stats.document_count,
        entity_count: stats.entity_count,
        relationship_count: stats.relationship_count,
        entity_type_count: 0, // PostgreSQL path doesn't have this yet; will be overridden by graph query
        chunk_count: stats.chunk_count,
        embedding_count: stats.embedding_count,
        storage_bytes: stats.storage_bytes as u64,
    })
}

/// Get stats from KV storage (moderate speed, current source of truth).
///
/// This aggregates document metadata from KV storage and counts chunks.
/// Reliable but slower than PostgreSQL as it requires fetching all metadata
/// and filtering in memory.
async fn try_kv_storage_stats(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<WorkspaceStatsResponse, ApiError> {
    // Get all keys from KV storage
    let all_keys = state
        .kv_storage
        .keys()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get KV storage keys: {}", e)))?;

    // Filter metadata keys
    let metadata_keys: Vec<String> = all_keys
        .iter()
        .filter(|k| k.ends_with("-metadata"))
        .cloned()
        .collect();

    // Get all metadata values
    let metadata_values = state
        .kv_storage
        .get_by_ids(&metadata_keys)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get document metadata: {}", e)))?;

    // Aggregate stats from documents belonging to this workspace
    let mut document_count = 0;
    let mut storage_bytes: u64 = 0;
    let mut workspace_doc_ids = Vec::new();

    for value in metadata_values {
        if let Some(obj) = value.as_object() {
            // Check if document belongs to this workspace
            let doc_workspace_id = obj
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok());

            if doc_workspace_id == Some(workspace_id) {
                document_count += 1;

                // Collect document ID for chunk counting
                if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
                    workspace_doc_ids.push(id.to_string());
                }

                // Sum storage bytes
                if let Some(bytes) = obj.get("file_size_bytes").and_then(|v| v.as_u64()) {
                    storage_bytes += bytes;
                }
            }
        }
    }

    // OODA-03: Get entity/relationship counts from Apache AGE graph storage
    // WHY: KV metadata doesn't have entity_count/relationship_count fields.
    // The actual entity/relationship data is stored in the graph, not metadata.
    // This fixes dashboard showing 0 entities despite successful extraction.
    let entity_count = state
        .graph_storage
        .node_count_by_workspace(&workspace_id)
        .await
        .unwrap_or(0);

    let relationship_count = state
        .graph_storage
        .edge_count_by_workspace(&workspace_id)
        .await
        .unwrap_or(0);

    // Count chunks and embeddings for this workspace's documents
    let mut chunk_count = 0;
    let mut embedding_count = 0;

    for doc_id in &workspace_doc_ids {
        // Count chunk keys for this document
        let doc_chunk_keys: Vec<String> = all_keys
            .iter()
            .filter(|k| k.starts_with(&format!("{}-chunk-", doc_id)))
            .cloned()
            .collect();

        chunk_count += doc_chunk_keys.len();

        // Get chunk data to check for embeddings
        if !doc_chunk_keys.is_empty() {
            let chunk_values = state
                .kv_storage
                .get_by_ids(&doc_chunk_keys)
                .await
                .map_err(|e| ApiError::Internal(format!("Failed to get chunk data: {}", e)))?;

            // Count chunks that have embeddings
            for chunk_value in chunk_values {
                if let Some(obj) = chunk_value.as_object() {
                    if obj.get("embedding").is_some() {
                        embedding_count += 1;
                    }
                }
            }
        }
    }

    // Get distinct entity type count from graph storage.
    // WHY: Dashboard EntityTypes KPI was extremely slow — it fetched ALL graph
    // nodes over the wire just to compute unique types. This single aggregate
    // query reduces latency from seconds to milliseconds.
    let entity_type_count = state
        .graph_storage
        .distinct_node_type_count_by_workspace(&workspace_id)
        .await
        .unwrap_or(0);

    Ok(WorkspaceStatsResponse {
        workspace_id,
        document_count,
        entity_count,
        relationship_count,
        entity_type_count,
        chunk_count,
        embedding_count,
        storage_bytes,
    })
}

// ============================================================================
// OODA-22: Metrics History Endpoint
// ============================================================================

/// Get metrics history for a workspace.
///
/// Returns time-series metrics snapshots in reverse chronological order (newest first).
/// Useful for trend analysis, debugging, and monitoring workspace growth.
///
/// ## Query Parameters
///
/// - `limit`: Maximum number of snapshots to return (default: 100, max: 1000)
/// - `offset`: Number of snapshots to skip (default: 0)
///
/// ## Trigger Types
///
/// - `event`: Recorded after document add/delete operations
/// - `scheduled`: Recorded by background hourly task
/// - `manual`: Recorded by admin request
#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/metrics-history",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID"),
        ("limit" = Option<usize>, Query, description = "Maximum snapshots to return (default: 100)"),
        ("offset" = Option<usize>, Query, description = "Number of snapshots to skip (default: 0)")
    ),
    responses(
        (status = 200, description = "Metrics history", body = MetricsHistoryResponse),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn get_metrics_history(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Query(params): Query<MetricsHistoryParams>,
) -> Result<Json<MetricsHistoryResponse>, ApiError> {
    // Apply defaults and limits
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    let snapshots = state
        .workspace_service
        .get_metrics_history(workspace_id, limit, offset)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let response = MetricsHistoryResponse {
        workspace_id,
        count: snapshots.len(),
        offset,
        limit,
        snapshots: snapshots
            .into_iter()
            .map(|s| MetricsSnapshotDTO {
                id: s.id,
                recorded_at: s.recorded_at.to_rfc3339(),
                trigger_type: s.trigger_type.to_string(),
                document_count: s.document_count,
                chunk_count: s.chunk_count,
                entity_count: s.entity_count,
                relationship_count: s.relationship_count,
                embedding_count: s.embedding_count,
                storage_bytes: s.storage_bytes,
            })
            .collect(),
    };

    Ok(Json(response))
}

/// Query parameters for metrics history endpoint.
#[derive(Debug, Deserialize)]
pub struct MetricsHistoryParams {
    /// Maximum number of snapshots to return.
    pub limit: Option<usize>,
    /// Number of snapshots to skip.
    pub offset: Option<usize>,
}

/// Manually trigger a metrics snapshot for a workspace.
///
/// # Implements
///
/// - **FEAT1701**: Workspace Metrics Tracking
///
/// # WHY: Manual Trigger
///
/// Users may want to capture a metrics snapshot at a specific point in time
/// for debugging, auditing, or comparison purposes. This endpoint allows
/// manual triggering without waiting for automatic event-based recording.
///
/// # Use Cases
///
/// - Debug workspace state at a specific moment
/// - Capture baseline before bulk operations
/// - External scheduler integration (cron jobs)
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/metrics-snapshot",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 201, description = "Snapshot created", body = MetricsSnapshotDTO),
        (status = 404, description = "Workspace not found"),
    ),
    tags = ["workspaces"]
)]
pub async fn trigger_metrics_snapshot(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<(StatusCode, Json<MetricsSnapshotDTO>), ApiError> {
    // Record a manual-triggered snapshot
    let snapshot = state
        .workspace_service
        .record_metrics_snapshot(workspace_id, MetricsTriggerType::Manual)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let dto = MetricsSnapshotDTO {
        id: snapshot.id,
        recorded_at: snapshot.recorded_at.to_rfc3339(),
        trigger_type: snapshot.trigger_type.to_string(),
        document_count: snapshot.document_count,
        chunk_count: snapshot.chunk_count,
        entity_count: snapshot.entity_count,
        relationship_count: snapshot.relationship_count,
        embedding_count: snapshot.embedding_count,
        storage_bytes: snapshot.storage_bytes,
    };

    Ok((StatusCode::CREATED, Json(dto)))
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

    // WHY: Auto-detect dimension from model config when model changes
    // If embedding_dimension is explicitly provided in the request, use it.
    // Otherwise, look up the correct dimension from the model's config.
    // This ensures dimension is always consistent with the selected model.
    let new_dimension = if let Some(dim) = request.embedding_dimension {
        dim
    } else if new_model != workspace.embedding_model || new_provider != workspace.embedding_provider
    {
        // Model is changing - look up the correct dimension for the new model
        state
            .models_config
            .get_model(&new_provider, &new_model)
            .map(|m| m.capabilities.embedding_dimension)
            .unwrap_or_else(|| {
                tracing::warn!(
                    provider = %new_provider,
                    model = %new_model,
                    "No embedding dimension found for model, using workspace default"
                );
                workspace.embedding_dimension
            })
    } else {
        // No model change, keep existing dimension
        workspace.embedding_dimension
    };

    // 4. Check if config is actually changing
    let config_changed = new_model != workspace.embedding_model
        || new_provider != workspace.embedding_provider
        || new_dimension != workspace.embedding_dimension;

    if !config_changed && !request.force {
        return Err(ApiError::BadRequest(
            "Embedding configuration unchanged. Use 'force: true' to rebuild anyway.".to_string(),
        ));
    }

    // REQ-25: Validate chunk size vs embedding model compatibility (CRITICAL INVARIANT)
    // Get the new embedding model's context length to ensure chunks will fit
    let model_context_length = state
        .models_config
        .get_model(&new_provider, &new_model)
        .map(|m| m.capabilities.context_length)
        .unwrap_or(8192); // Default to safe value if model not found

    // Default chunk size is 1200 tokens (from chunker config)
    const DEFAULT_CHUNK_SIZE_TOKENS: usize = 1200;

    if model_context_length > 0 && DEFAULT_CHUNK_SIZE_TOKENS > model_context_length {
        info!(
            workspace_id = %workspace_id,
            chunk_size = DEFAULT_CHUNK_SIZE_TOKENS,
            model_context_length = model_context_length,
            warning = "Default chunk size exceeds model's context length",
            "Chunk-embedding compatibility warning - some chunks may fail to embed"
        );
        // Log warning but allow the operation to proceed
        // Future: Could add a strict mode that blocks incompatible changes
    }

    info!(
        workspace_id = %workspace_id,
        old_model = %workspace.embedding_model,
        new_model = %new_model,
        old_dimension = workspace.embedding_dimension,
        new_dimension = new_dimension,
        document_count = stats.document_count,
        model_context_length = model_context_length,
        "Starting embedding rebuild"
    );

    // 5. Clear vector storage for this specific workspace only
    // Uses workspace-scoped clearing to avoid affecting other workspaces
    let vectors_cleared = state
        .vector_storage
        .clear_workspace(&workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to clear workspace vectors: {}", e)))?;

    info!(
        workspace_id = %workspace_id,
        vectors_cleared = vectors_cleared,
        "Vector storage cleared"
    );

    // OODA-225: Evict cached workspace vector storage when dimension changes
    // WHY: The WorkspaceVectorRegistry caches vector storage instances keyed by workspace_id.
    // When embedding dimension changes (e.g., 768 → 1536), the cached instance still references
    // the old dimension. Without eviction, queries will fail with "different vector dimensions"
    // because the query embedding (new dimension) doesn't match stored vectors (old dimension).
    // Evicting forces recreation with the new dimension on next access.
    if config_changed {
        state.vector_registry.evict(&workspace_id).await;
        info!(
            workspace_id = %workspace_id,
            old_dimension = workspace.embedding_dimension,
            new_dimension = new_dimension,
            "Evicted workspace vector storage cache for dimension change"
        );
    }

    // 6. Update workspace embedding config if changed (SPEC-032)
    if config_changed {
        use edgequake_core::UpdateWorkspaceRequest;

        let update_request = UpdateWorkspaceRequest {
            embedding_model: Some(new_model.clone()),
            embedding_provider: Some(new_provider.clone()),
            embedding_dimension: Some(new_dimension),
            ..Default::default()
        };

        state
            .workspace_service
            .update_workspace(workspace_id, update_request)
            .await
            .map_err(|e| {
                ApiError::Internal(format!(
                    "Failed to update workspace embedding config: {}",
                    e
                ))
            })?;

        info!(
            workspace_id = %workspace_id,
            embedding_model = %new_model,
            embedding_provider = %new_provider,
            embedding_dimension = new_dimension,
            "Workspace embedding configuration updated"
        );
    }

    // 7. Queue documents for re-embedding (SPEC-032 REQ-25)
    // This triggers the actual re-embedding process using the new embedding model
    let (documents_queued, chunks_to_process, track_id) = if stats.document_count > 0 {
        use chrono::Utc;
        use edgequake_tasks::{Task, TaskType, TextInsertData};

        let track_id = format!(
            "rebuild_embed_{}_{}",
            Utc::now().format("%Y%m%d_%H%M%S"),
            &Uuid::new_v4().to_string()[..8]
        );

        // Get all document metadata for this workspace
        let all_keys: Vec<String> = state
            .kv_storage
            .keys()
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to list document keys: {}", e)))?;

        let mut documents_queued = 0;
        let mut total_chunks = 0usize;

        for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
            if let Some(value) = state.kv_storage.get_by_id(key).await.ok().flatten() {
                if let Some(obj) = value.as_object() {
                    // Check if document belongs to this workspace
                    let doc_workspace = obj
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default");

                    if doc_workspace != workspace_id.to_string() && doc_workspace != "default" {
                        continue;
                    }

                    let doc_id = match obj.get("id").and_then(|v| v.as_str()) {
                        Some(id) => id.to_string(),
                        None => continue,
                    };

                    // Extract chunk count for this document
                    let doc_chunk_count =
                        obj.get("chunk_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

                    let title = obj.get("title").and_then(|v| v.as_str());

                    // Get document content
                    let content_key = format!("{}-content", doc_id);
                    let content = match state.kv_storage.get_by_id(&content_key).await {
                        Ok(Some(content_value)) => content_value
                            .get("content")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        _ => None,
                    };

                    let content = match content {
                        Some(c) => c,
                        None => continue,
                    };

                    // Update document status to pending
                    let metadata_key = format!("{}-metadata", doc_id);
                    if let Some(mut metadata) = state
                        .kv_storage
                        .get_by_id(&metadata_key)
                        .await
                        .ok()
                        .flatten()
                    {
                        if let Some(obj) = metadata.as_object_mut() {
                            obj.insert("status".to_string(), serde_json::json!("pending"));
                            obj.insert("track_id".to_string(), serde_json::json!(track_id));
                            obj.insert(
                                "reprocess_at".to_string(),
                                serde_json::json!(Utc::now().to_rfc3339()),
                            );
                            let _ = state.kv_storage.upsert(&[(metadata_key, metadata)]).await;
                        }
                    }

                    // Create processing task
                    let doc_title = title.unwrap_or(&doc_id).to_string();
                    let task_data = TextInsertData {
                        text: content,
                        file_source: doc_title.clone(),
                        workspace_id: workspace_id.to_string(),
                        metadata: Some(serde_json::json!({
                            "document_id": doc_id,
                            "title": doc_title,
                            "track_id": track_id,
                            "is_reprocess": true,
                            "is_embedding_rebuild": true,
                            "workspace_id": workspace_id.to_string(),
                            "tenant_id": workspace.tenant_id.to_string(),
                        })),
                    };

                    let task = Task::new(
                        workspace.tenant_id,
                        workspace_id,
                        TaskType::Insert,
                        serde_json::to_value(&task_data).unwrap(),
                    );

                    // Store and queue task
                    if state.task_storage.create_task(&task).await.is_ok()
                        && state.task_queue.send(task).await.is_ok()
                    {
                        documents_queued += 1;
                        total_chunks += doc_chunk_count;
                    }
                }
            }
        }

        info!(
            workspace_id = %workspace_id,
            track_id = %track_id,
            documents_queued = documents_queued,
            total_chunks = total_chunks,
            "Documents queued for re-embedding"
        );

        (documents_queued, total_chunks, Some(track_id))
    } else {
        (0, 0, None)
    };

    // 8. Build response
    // Estimate: ~1 second per document for embedding (conservative)
    let estimated_time = if stats.document_count > 0 {
        Some(stats.document_count as u64)
    } else {
        None
    };

    // REQ-25: Generate compatibility warning if chunks may exceed model limit
    let compatibility_warning = if model_context_length > 0
        && DEFAULT_CHUNK_SIZE_TOKENS > model_context_length
    {
        Some(format!(
            "Default chunk size ({} tokens) exceeds model's context length ({} tokens). Some chunks may fail to embed.",
            DEFAULT_CHUNK_SIZE_TOKENS, model_context_length
        ))
    } else {
        None
    };
    let has_compatibility_warning = compatibility_warning.is_some();

    // Determine status based on whether documents were queued
    let status = if documents_queued > 0 {
        "processing".to_string()
    } else if vectors_cleared > 0 {
        "vectors_cleared".to_string()
    } else {
        "no_change".to_string()
    };

    let response = RebuildEmbeddingsResponse {
        workspace_id,
        status,
        documents_to_process: documents_queued,
        chunks_to_process,
        vectors_cleared,
        embedding_model: new_model.clone(),
        embedding_provider: new_provider.clone(),
        embedding_dimension: new_dimension,
        model_context_length,
        estimated_time_seconds: estimated_time,
        job_id: track_id.clone(),
        compatibility_warning,
    };

    info!(
        workspace_id = %workspace_id,
        status = %response.status,
        documents_queued = documents_queued,
        chunks_to_process = chunks_to_process,
        vectors_cleared = vectors_cleared,
        embedding_model = %new_model,
        embedding_provider = %new_provider,
        model_context_length = model_context_length,
        has_warning = has_compatibility_warning,
        track_id = ?track_id,
        "Embedding rebuild complete - documents queued for re-embedding"
    );

    Ok(Json(response))
}

// ============================================================================
// Rebuild Knowledge Graph Endpoint (LLM Model Change)
// ============================================================================

/// Rebuild knowledge graph for a workspace after LLM model change.
///
/// This operation:
/// 1. Clears all entities and relationships from the graph storage
/// 2. Optionally clears vector embeddings (default: yes)
/// 3. Queues all documents for reprocessing with the new LLM model
///
/// Use this when:
/// - Changing the extraction/LLM model (e.g., gpt-4o-mini → gemma3:12b)
/// - Upgrading to a new LLM version with better entity extraction
/// - Migrating between LLM providers
///
/// ## WARNING
///
/// This is a destructive operation. All existing knowledge graph data
/// (entities, relationships) will be deleted. The workspace will be empty
/// until document reprocessing is complete.
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph",
    request_body = RebuildKnowledgeGraphRequest,
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Knowledge graph rebuild started", body = RebuildKnowledgeGraphResponse),
        (status = 404, description = "Workspace not found"),
        (status = 400, description = "Invalid request"),
    ),
    tags = ["workspaces"]
)]
pub async fn rebuild_knowledge_graph(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<RebuildKnowledgeGraphRequest>,
) -> Result<Json<RebuildKnowledgeGraphResponse>, ApiError> {
    use chrono::Utc;
    use tracing::info;

    // 1. Get the workspace
    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

    // 2. Get workspace stats
    let stats = state
        .workspace_service
        .get_workspace_stats(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // 3. Determine new LLM config
    let new_llm_model = request
        .llm_model
        .clone()
        .unwrap_or_else(|| workspace.llm_model.clone());
    let new_llm_provider = request
        .llm_provider
        .clone()
        .unwrap_or_else(|| workspace.llm_provider.clone());

    // 4. Check if config is actually changing
    let config_changed =
        new_llm_model != workspace.llm_model || new_llm_provider != workspace.llm_provider;

    if !config_changed && !request.force {
        return Err(ApiError::BadRequest(
            "LLM configuration unchanged. Use 'force: true' to rebuild anyway.".to_string(),
        ));
    }

    info!(
        workspace_id = %workspace_id,
        old_model = %workspace.llm_model,
        new_model = %new_llm_model,
        old_provider = %workspace.llm_provider,
        new_provider = %new_llm_provider,
        document_count = stats.document_count,
        rebuild_embeddings = request.rebuild_embeddings,
        "Starting knowledge graph rebuild"
    );

    // 5. Clear graph storage (workspace-scoped)
    let (nodes_cleared, edges_cleared) =
        state
            .graph_storage
            .clear_workspace(&workspace_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to clear graph: {}", e)))?;

    info!(
        workspace_id = %workspace_id,
        nodes_cleared = nodes_cleared,
        edges_cleared = edges_cleared,
        "Graph storage cleared"
    );

    // 6. Optionally clear vectors (if also changing embeddings)
    let vectors_cleared = if request.rebuild_embeddings {
        let count = state
            .vector_storage
            .clear_workspace(&workspace_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to clear vectors: {}", e)))?;

        // OODA-225: Evict cached workspace vector storage when clearing vectors
        // WHY: If rebuild_embeddings is requested, the embedding model/dimension may change.
        // The cached vector storage instance holds the old dimension configuration.
        // Evicting forces recreation with correct dimension on next access.
        state.vector_registry.evict(&workspace_id).await;

        info!(
            workspace_id = %workspace_id,
            vectors_cleared = count,
            "Vector storage cleared and cache evicted"
        );
        count
    } else {
        0
    };

    // 7. Generate track ID for reprocessing batch
    let track_id = format!(
        "rebuild_kg_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &uuid::Uuid::new_v4().to_string()[..8]
    );

    // 8. Update workspace LLM config if changed (SPEC-032)
    if config_changed {
        use edgequake_core::UpdateWorkspaceRequest;

        let update_request = UpdateWorkspaceRequest {
            llm_model: Some(new_llm_model.clone()),
            llm_provider: Some(new_llm_provider.clone()),
            ..Default::default()
        };

        state
            .workspace_service
            .update_workspace(workspace_id, update_request)
            .await
            .map_err(|e| {
                ApiError::Internal(format!("Failed to update workspace LLM config: {}", e))
            })?;

        info!(
            workspace_id = %workspace_id,
            llm_model = %new_llm_model,
            llm_provider = %new_llm_provider,
            "Workspace LLM configuration updated"
        );
    }

    // 9. Queue all documents for reprocessing (SPEC-032 REQ-24)
    let (documents_queued, chunks_to_process) = if stats.document_count > 0 {
        use edgequake_tasks::{Task, TaskType, TextInsertData};

        // Get all document metadata for this workspace
        let all_keys: Vec<String> = state
            .kv_storage
            .keys()
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to list document keys: {}", e)))?;

        let mut documents_queued = 0;
        let mut total_chunks = 0usize;

        for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
            if let Some(value) = state.kv_storage.get_by_id(key).await.ok().flatten() {
                if let Some(obj) = value.as_object() {
                    // Check if document belongs to this workspace
                    let doc_workspace = obj
                        .get("workspace_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("default");

                    if doc_workspace != workspace_id.to_string() && doc_workspace != "default" {
                        continue;
                    }

                    let doc_id = match obj.get("id").and_then(|v| v.as_str()) {
                        Some(id) => id.to_string(),
                        None => continue,
                    };

                    // Extract chunk count for this document
                    let doc_chunk_count =
                        obj.get("chunk_count").and_then(|v| v.as_u64()).unwrap_or(1) as usize;

                    let title = obj.get("title").and_then(|v| v.as_str());

                    // Get document content
                    let content_key = format!("{}-content", doc_id);
                    let content = match state.kv_storage.get_by_id(&content_key).await {
                        Ok(Some(content_value)) => content_value
                            .get("content")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        _ => None,
                    };

                    let content = match content {
                        Some(c) => c,
                        None => continue,
                    };

                    // Update document status to pending
                    let metadata_key = format!("{}-metadata", doc_id);
                    if let Some(mut metadata) = state
                        .kv_storage
                        .get_by_id(&metadata_key)
                        .await
                        .ok()
                        .flatten()
                    {
                        if let Some(obj) = metadata.as_object_mut() {
                            obj.insert("status".to_string(), serde_json::json!("pending"));
                            obj.insert("track_id".to_string(), serde_json::json!(track_id));
                            obj.insert(
                                "reprocess_at".to_string(),
                                serde_json::json!(Utc::now().to_rfc3339()),
                            );
                            let _ = state.kv_storage.upsert(&[(metadata_key, metadata)]).await;
                        }
                    }

                    // Create processing task
                    let doc_title = title.unwrap_or(&doc_id).to_string();
                    let task_data = TextInsertData {
                        text: content,
                        file_source: doc_title.clone(),
                        workspace_id: workspace_id.to_string(),
                        metadata: Some(serde_json::json!({
                            "document_id": doc_id,
                            "title": doc_title,
                            "track_id": track_id,
                            "is_reprocess": true,
                            "is_kg_rebuild": true,
                            "workspace_id": workspace_id.to_string(),
                            "tenant_id": workspace.tenant_id.to_string(),
                        })),
                    };

                    let task = Task::new(
                        workspace.tenant_id,
                        workspace_id,
                        TaskType::Insert,
                        serde_json::to_value(&task_data).unwrap(),
                    );

                    // Store and queue task
                    if state.task_storage.create_task(&task).await.is_ok()
                        && state.task_queue.send(task).await.is_ok()
                    {
                        documents_queued += 1;
                        total_chunks += doc_chunk_count;
                    }
                }
            }
        }

        info!(
            workspace_id = %workspace_id,
            track_id = %track_id,
            documents_queued = documents_queued,
            total_chunks = total_chunks,
            "Documents queued for knowledge graph rebuild"
        );

        (documents_queued, total_chunks)
    } else {
        (0, 0)
    };

    // 10. Build response
    let estimated_time = if stats.document_count > 0 {
        // Estimate: ~2 seconds per document (extraction + embedding)
        Some(stats.document_count as u64 * 2)
    } else {
        None
    };

    // Determine status based on whether documents were queued
    let status = if documents_queued > 0 {
        "processing".to_string()
    } else if nodes_cleared > 0 || edges_cleared > 0 {
        "graph_cleared".to_string()
    } else {
        "no_change".to_string()
    };

    let response = RebuildKnowledgeGraphResponse {
        workspace_id,
        status,
        nodes_cleared,
        edges_cleared,
        vectors_cleared,
        documents_to_process: documents_queued,
        chunks_to_process,
        llm_model: new_llm_model.clone(),
        llm_provider: new_llm_provider.clone(),
        estimated_time_seconds: estimated_time,
        track_id: Some(track_id.clone()),
    };

    info!(
        workspace_id = %workspace_id,
        status = %response.status,
        nodes = nodes_cleared,
        edges = edges_cleared,
        vectors = vectors_cleared,
        documents_queued = documents_queued,
        chunks_to_process = chunks_to_process,
        llm_model = %new_llm_model,
        llm_provider = %new_llm_provider,
        track_id = %track_id,
        "Knowledge graph rebuild complete - documents queued for reprocessing"
    );

    Ok(Json(response))
}

// SPEC-032: Reprocess All Documents Endpoint
// Focus Area 5 - Trigger document reprocessing after rebuild

/// Reprocess all documents in a workspace.
///
/// This endpoint queues all documents for re-embedding, typically used after
/// a rebuild-embeddings operation to regenerate vector embeddings. Progress
/// can be monitored via the pipeline status endpoint.
///
/// ## Use Cases
///
/// - Regenerate embeddings after model change
/// - Re-extract entities after LLM update
/// - Bulk re-processing for quality improvements
#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/reprocess-documents",
    request_body = ReprocessAllRequest,
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace ID")
    ),
    responses(
        (status = 200, description = "Documents queued for reprocessing", body = ReprocessAllResponse),
        (status = 404, description = "Workspace not found"),
        (status = 400, description = "Invalid request"),
    ),
    tags = ["workspaces"]
)]
pub async fn reprocess_all_documents(
    State(state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<ReprocessAllRequest>,
) -> Result<Json<ReprocessAllResponse>, ApiError> {
    use chrono::Utc;
    use edgequake_tasks::{Task, TaskType, TextInsertData};
    use tracing::info;

    // 1. Verify workspace exists
    let workspace = state
        .workspace_service
        .get_workspace(workspace_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace {} not found", workspace_id)))?;

    // 2. Generate track ID for this batch
    let track_id = format!(
        "reprocess_{}_{}",
        Utc::now().format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..8]
    );

    info!(
        workspace_id = %workspace_id,
        track_id = %track_id,
        include_completed = request.include_completed,
        "Starting reprocess all documents"
    );

    // 3. Get all document metadata for this workspace
    let all_keys: Vec<String> = state
        .kv_storage
        .keys()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list document keys: {}", e)))?;

    // REQ-24: Debug logging for document discovery
    let metadata_keys_count = all_keys.iter().filter(|k| k.ends_with("-metadata")).count();
    info!(
        workspace_id = %workspace_id,
        total_keys = all_keys.len(),
        metadata_keys = metadata_keys_count,
        "Scanning KV storage for documents to reprocess"
    );

    let mut documents_found = 0;
    let mut documents_queued = 0;
    let mut documents_skipped = 0;
    let mut skip_reasons: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();

    // 4. Process each document
    for key in all_keys.iter().filter(|k| k.ends_with("-metadata")) {
        if documents_queued >= request.max_documents {
            *skip_reasons.entry("max_documents_reached").or_insert(0) += 1;
            break;
        }

        if let Some(value) =
            state.kv_storage.get_by_id(key).await.map_err(|e| {
                ApiError::Internal(format!("Failed to get document metadata: {}", e))
            })?
        {
            if let Some(obj) = value.as_object() {
                // Check if document belongs to this workspace
                let doc_workspace = obj
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");

                if doc_workspace != workspace_id.to_string() && doc_workspace != "default" {
                    *skip_reasons.entry("wrong_workspace").or_insert(0) += 1;
                    continue;
                }

                documents_found += 1;

                let status = obj.get("status").and_then(|v| v.as_str());
                let doc_id = obj.get("id").and_then(|v| v.as_str());
                let title = obj.get("title").and_then(|v| v.as_str());

                // Skip if not including completed and already completed
                if !request.include_completed && status == Some("completed") {
                    documents_skipped += 1;
                    *skip_reasons.entry("completed_excluded").or_insert(0) += 1;
                    continue;
                }

                // Skip if currently processing
                if status == Some("processing") {
                    documents_skipped += 1;
                    *skip_reasons.entry("already_processing").or_insert(0) += 1;
                    continue;
                }

                // Get document ID
                let doc_id = match doc_id {
                    Some(id) => id.to_string(),
                    None => {
                        documents_skipped += 1;
                        *skip_reasons.entry("no_doc_id").or_insert(0) += 1;
                        continue;
                    }
                };

                // Get document content
                let content_key = format!("{}-content", doc_id);
                let content = match state.kv_storage.get_by_id(&content_key).await {
                    Ok(Some(content_value)) => content_value
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    _ => None,
                };

                let content = match content {
                    Some(c) => c,
                    None => {
                        documents_skipped += 1;
                        *skip_reasons.entry("no_content").or_insert(0) += 1;
                        continue;
                    }
                };

                // Update document status to pending
                let metadata_key = format!("{}-metadata", doc_id);
                if let Some(mut metadata) = state
                    .kv_storage
                    .get_by_id(&metadata_key)
                    .await
                    .ok()
                    .flatten()
                {
                    if let Some(obj) = metadata.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("pending"));
                        obj.insert("track_id".to_string(), serde_json::json!(track_id));
                        obj.insert(
                            "reprocess_at".to_string(),
                            serde_json::json!(Utc::now().to_rfc3339()),
                        );

                        let _ = state.kv_storage.upsert(&[(metadata_key, metadata)]).await;
                    }
                }

                // Create processing task
                let doc_title = title.unwrap_or(&doc_id).to_string();
                let task_data = TextInsertData {
                    text: content,
                    file_source: doc_title.clone(),
                    workspace_id: workspace_id.to_string(),
                    metadata: Some(serde_json::json!({
                        "document_id": doc_id,
                        "title": doc_title,
                        "track_id": track_id,
                        "is_reprocess": true,
                        "workspace_id": workspace_id.to_string(),
                        "tenant_id": workspace.tenant_id.to_string(),
                    })),
                };

                let task = Task::new(
                    workspace.tenant_id,
                    workspace_id,
                    TaskType::Insert,
                    serde_json::to_value(&task_data).unwrap(),
                );

                // Store and queue task
                if let Err(e) = state.task_storage.create_task(&task).await {
                    info!(error = %e, doc_id = %doc_id, "Failed to create task, skipping");
                    documents_skipped += 1;
                    *skip_reasons.entry("task_create_failed").or_insert(0) += 1;
                    continue;
                }

                if let Err(e) = state.task_queue.send(task).await {
                    info!(error = %e, doc_id = %doc_id, "Failed to queue task, skipping");
                    documents_skipped += 1;
                    *skip_reasons.entry("task_queue_failed").or_insert(0) += 1;
                    continue;
                }

                documents_queued += 1;
            }
        }
    }

    // REQ-24: Log detailed skip reasons for debugging
    if !skip_reasons.is_empty() {
        info!(
            workspace_id = %workspace_id,
            skip_reasons = ?skip_reasons,
            "Document skip reasons breakdown"
        );
    }

    // 5. Estimate processing time (1 second per document conservative)
    let estimated_time = if documents_queued > 0 {
        Some(documents_queued as u64)
    } else {
        None
    };

    let response = ReprocessAllResponse {
        track_id,
        workspace_id,
        status: if documents_queued > 0 {
            "processing".to_string()
        } else {
            "no_documents".to_string()
        },
        documents_found,
        documents_queued,
        documents_skipped,
        estimated_time_seconds: estimated_time,
    };

    info!(
        workspace_id = %workspace_id,
        found = documents_found,
        queued = documents_queued,
        skipped = documents_skipped,
        "Reprocess all documents complete"
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
            entity_type_count: 15,
            chunk_count: 1000,
            embedding_count: 800,
            storage_bytes: 1024 * 1024,
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(json_str.contains("\"document_count\":100"));
        assert!(json_str.contains("\"embedding_count\":800"));
    }
}
