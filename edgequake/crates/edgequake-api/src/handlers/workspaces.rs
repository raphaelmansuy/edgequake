//! Workspace management handlers.
//!
//! Provides REST API endpoints for managing tenants and workspaces.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

// ============ Request/Response DTOs ============

/// Request to create a new tenant.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTenantRequest {
    /// Tenant name.
    pub name: String,
    /// URL-friendly slug (auto-generated if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Plan type (free, basic, pro, enterprise).
    pub plan: Option<String>,
}

/// Request to update a tenant.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTenantRequest {
    /// New tenant name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New plan.
    pub plan: Option<String>,
    /// Whether the tenant is active.
    pub is_active: Option<bool>,
}

/// Request to create a new workspace.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkspaceApiRequest {
    /// Workspace name.
    pub name: String,
    /// URL-friendly slug (auto-generated if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Maximum number of documents.
    pub max_documents: Option<usize>,
}

/// Request to update a workspace.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWorkspaceApiRequest {
    /// New workspace name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: Option<bool>,
    /// Maximum number of documents.
    pub max_documents: Option<usize>,
}

/// Tenant response DTO.
#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResponse {
    /// Tenant ID.
    pub id: Uuid,
    /// Tenant name.
    pub name: String,
    /// URL-friendly slug.
    pub slug: String,
    /// Plan type.
    pub plan: String,
    /// Whether the tenant is active.
    pub is_active: bool,
    /// Maximum workspaces allowed.
    pub max_workspaces: usize,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// Workspace response DTO.
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceResponse {
    /// Workspace ID.
    pub id: Uuid,
    /// Parent tenant ID.
    pub tenant_id: Uuid,
    /// Workspace name.
    pub name: String,
    /// URL-friendly slug.
    pub slug: String,
    /// Description.
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: bool,
    /// Maximum documents allowed.
    pub max_documents: Option<usize>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: String,
}

/// List response with pagination info.
#[derive(Debug, Serialize, ToSchema)]
pub struct TenantListResponse {
    /// Items in this page.
    pub items: Vec<TenantResponse>,
    /// Total count.
    pub total: usize,
    /// Current offset.
    pub offset: usize,
    /// Page size limit.
    pub limit: usize,
}

/// List response with pagination info.
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceListResponse {
    /// Items in this page.
    pub items: Vec<WorkspaceResponse>,
    /// Total count.
    pub total: usize,
    /// Current offset.
    pub offset: usize,
    /// Page size limit.
    pub limit: usize,
}

/// Pagination query params.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PaginationParams {
    /// Offset (default 0).
    #[serde(default)]
    pub offset: usize,
    /// Limit (default 20, max 100).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// Workspace statistics response.
#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceStatsResponse {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Number of documents.
    pub document_count: usize,
    /// Number of entities.
    pub entity_count: usize,
    /// Number of relationships.
    pub relationship_count: usize,
    /// Number of chunks.
    pub chunk_count: usize,
    /// Storage used in bytes.
    pub storage_bytes: u64,
}

// ============ Tenant Handlers ============

/// Create a new tenant.
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
    State(_state): State<AppState>,
    Json(request): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<TenantResponse>), ApiError> {
    use edgequake_core::{Tenant, TenantPlan};

    let slug = request
        .slug
        .unwrap_or_else(|| generate_slug(&request.name));

    let plan = match request.plan.as_deref() {
        Some("basic") => TenantPlan::Basic,
        Some("pro") => TenantPlan::Pro,
        Some("enterprise") => TenantPlan::Enterprise,
        _ => TenantPlan::Free,
    };

    let tenant = Tenant::new(&request.name, &slug).with_plan(plan);

    // TODO: Store tenant in database via workspace service
    // For now, return the created tenant

    let response = TenantResponse {
        id: tenant.tenant_id,
        name: tenant.name.clone(),
        slug: tenant.slug.clone(),
        plan: format!("{:?}", tenant.plan).to_lowercase(),
        is_active: tenant.is_active,
        max_workspaces: tenant.max_workspaces,
        created_at: tenant.created_at.to_rfc3339(),
        updated_at: tenant.updated_at.to_rfc3339(),
    };

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
    State(_state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<TenantListResponse>, ApiError> {
    let limit = params.limit.min(100);

    // TODO: Fetch from workspace service
    let response = TenantListResponse {
        items: vec![],
        total: 0,
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
    State(_state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<TenantResponse>, ApiError> {
    // TODO: Fetch from workspace service
    Err(ApiError::NotFound(format!(
        "Tenant {} not found",
        tenant_id
    )))
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
    State(_state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Json(_request): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, ApiError> {
    // TODO: Update via workspace service
    Err(ApiError::NotFound(format!(
        "Tenant {} not found",
        tenant_id
    )))
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
    State(_state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // TODO: Delete via workspace service
    tracing::info!(tenant_id = %tenant_id, "Deleting tenant");
    Err(ApiError::NotFound(format!(
        "Tenant {} not found",
        tenant_id
    )))
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
    State(_state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<CreateWorkspaceApiRequest>,
) -> Result<(StatusCode, Json<WorkspaceResponse>), ApiError> {
    use edgequake_core::Workspace;

    let slug = request
        .slug
        .clone()
        .unwrap_or_else(|| generate_slug(&request.name));

    let mut workspace = Workspace::new(tenant_id, &request.name, &slug);

    if let Some(desc) = request.description {
        workspace = workspace.with_description(desc);
    }

    if let Some(max_docs) = request.max_documents {
        workspace = workspace.with_max_documents(max_docs);
    }

    // TODO: Store workspace in database via workspace service

    let response = WorkspaceResponse {
        id: workspace.workspace_id,
        tenant_id: workspace.tenant_id,
        name: workspace.name.clone(),
        slug: workspace.slug.clone(),
        description: workspace.description.clone(),
        is_active: workspace.is_active,
        max_documents: workspace.max_documents(),
        created_at: workspace.created_at.to_rfc3339(),
        updated_at: workspace.updated_at.to_rfc3339(),
    };

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
    State(_state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<WorkspaceListResponse>, ApiError> {
    let limit = params.limit.min(100);

    tracing::debug!(tenant_id = %tenant_id, "Listing workspaces");

    // TODO: Fetch from workspace service
    let response = WorkspaceListResponse {
        items: vec![],
        total: 0,
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
    State(_state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    // TODO: Fetch from workspace service
    Err(ApiError::NotFound(format!(
        "Workspace {} not found",
        workspace_id
    )))
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
    State(_state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
    Json(_request): Json<UpdateWorkspaceApiRequest>,
) -> Result<Json<WorkspaceResponse>, ApiError> {
    // TODO: Update via workspace service
    Err(ApiError::NotFound(format!(
        "Workspace {} not found",
        workspace_id
    )))
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
    State(_state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // TODO: Delete via workspace service
    tracing::info!(workspace_id = %workspace_id, "Deleting workspace");
    Err(ApiError::NotFound(format!(
        "Workspace {} not found",
        workspace_id
    )))
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
    State(_state): State<AppState>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<WorkspaceStatsResponse>, ApiError> {
    // TODO: Fetch from workspace service
    let stats = WorkspaceStatsResponse {
        workspace_id,
        document_count: 0,
        entity_count: 0,
        relationship_count: 0,
        chunk_count: 0,
        storage_bytes: 0,
    };

    Ok(Json(stats))
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
}
