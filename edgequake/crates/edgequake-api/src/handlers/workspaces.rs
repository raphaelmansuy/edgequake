//! Workspace management handlers.
//!
//! Provides REST API endpoints for managing tenants and workspaces.

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
    CreateTenantRequest, CreateWorkspaceApiRequest, PaginationParams, TenantListResponse,
    TenantResponse, UpdateTenantRequest, UpdateWorkspaceApiRequest, WorkspaceListResponse,
    WorkspaceResponse, WorkspaceStatsResponse, default_limit,
};

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

    // Store tenant via workspace service
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Auto-create a default workspace for the new tenant (R004)
    // This ensures users always have at least one workspace available
    let default_workspace_request = edgequake_core::CreateWorkspaceRequest {
        name: "Default Workspace".to_string(),
        slug: Some("default".to_string()),
        description: Some("Automatically created default workspace".to_string()),
        max_documents: None,
    };

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
            "Auto-created default workspace for tenant"
        );
    }

    let response = TenantResponse {
        id: created_tenant.tenant_id,
        name: created_tenant.name.clone(),
        slug: created_tenant.slug.clone(),
        plan: format!("{}", created_tenant.plan),
        is_active: created_tenant.is_active,
        max_workspaces: created_tenant.max_workspaces,
        created_at: created_tenant.created_at.to_rfc3339(),
        updated_at: created_tenant.updated_at.to_rfc3339(),
    };

    tracing::info!(tenant_id = %created_tenant.tenant_id, "Created tenant");
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

    let create_request = CreateWorkspaceRequest {
        name: request.name.clone(),
        slug: request.slug.clone(),
        description: request.description.clone(),
        max_documents: request.max_documents,
    };

    // Store workspace via workspace service
    let workspace = state
        .workspace_service
        .create_workspace(tenant_id, create_request)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

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

    tracing::info!(
        workspace_id = %workspace.workspace_id,
        tenant_id = %tenant_id,
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
        .map(|ws| WorkspaceResponse {
            id: ws.workspace_id,
            tenant_id: ws.tenant_id,
            name: ws.name.clone(),
            slug: ws.slug.clone(),
            description: ws.description.clone(),
            is_active: ws.is_active,
            max_documents: ws.max_documents(),
            created_at: ws.created_at.to_rfc3339(),
            updated_at: ws.updated_at.to_rfc3339(),
        })
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
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&response);
        assert!(json.is_ok());
        assert!(json.unwrap().contains("test-tenant"));
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
