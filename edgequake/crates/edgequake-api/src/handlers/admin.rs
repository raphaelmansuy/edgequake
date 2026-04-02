//! Admin quota management handlers — SPEC-0001
//!
//! Provides admin-only endpoints for managing tenant workspace quotas and
//! server-wide default workspace limits at runtime (without redeployment).
//!
//! ## Implements
//!
//! - **SPEC-0001**: Tenant Workspace Limits (Issue #133)
//!
//! ## Endpoints
//!
//! | Method | Path                                          | Purpose                           |
//! |--------|-----------------------------------------------|-----------------------------------|
//! | PATCH  | `/api/v1/admin/tenants/:tenant_id/quota`      | Update a tenant's max_workspaces  |
//! | PATCH  | `/api/v1/admin/config/defaults`               | Set server-wide default for new tenants |
//! | GET    | `/api/v1/admin/config/defaults`               | Get current server-wide default   |

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

// ── Request / Response types ──────────────────────────────────────────────────

/// Request body for updating a tenant's workspace quota.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTenantQuotaRequest {
    /// New maximum number of workspaces for this tenant.
    ///
    /// Must be > 0, ≤ 10000, and ≥ current workspace count.
    pub max_workspaces: usize,
}

/// Response for a successful tenant quota update.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateTenantQuotaResponse {
    /// The tenant whose quota was updated.
    pub tenant_id: Uuid,
    /// New max_workspaces value.
    pub max_workspaces: usize,
    /// Previous max_workspaces value.
    pub previous_max_workspaces: usize,
    /// Current number of workspaces (used during validation).
    pub current_workspace_count: usize,
}

/// Request body for updating server-wide defaults.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateServerDefaultsRequest {
    /// New default max_workspaces for newly created tenants.
    ///
    /// Must be > 0 and ≤ 10000. Not retroactive — only affects new tenants.
    pub default_max_workspaces: usize,
}

/// Response for server-wide defaults.
#[derive(Debug, Serialize, ToSchema)]
pub struct ServerDefaultsResponse {
    /// Current server-wide default max_workspaces for new tenants.
    pub default_max_workspaces: usize,
    /// Note about retroactivity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Update workspace quota for a specific tenant.
///
/// # Validation (SPEC-0001)
/// - V1: `max_workspaces > 0`
/// - V2: `max_workspaces >= current workspace count`
/// - V3: `max_workspaces <= 10000`
///
/// # Concurrency
///
/// Uses `SELECT FOR UPDATE` (PostgreSQL) to prevent TOCTOU race conditions.
///
/// PATCH /api/v1/admin/tenants/:tenant_id/quota
#[utoipa::path(
    patch,
    path = "/api/v1/admin/tenants/{tenant_id}/quota",
    params(
        ("tenant_id" = Uuid, Path, description = "Tenant ID")
    ),
    request_body = UpdateTenantQuotaRequest,
    responses(
        (status = 200, description = "Quota updated", body = UpdateTenantQuotaResponse),
        (status = 400, description = "Invalid value (zero or exceeds limit)"),
        (status = 404, description = "Tenant not found"),
        (status = 409, description = "Cannot reduce below current workspace count"),
    ),
    tags = ["admin"]
)]
pub async fn update_tenant_quota(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Json(request): Json<UpdateTenantQuotaRequest>,
) -> Result<Json<UpdateTenantQuotaResponse>, ApiError> {
    let result = state
        .workspace_service
        .update_tenant_quota(tenant_id, request.max_workspaces)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("not found") {
                ApiError::NotFound(msg)
            } else if msg.contains("Cannot reduce") {
                ApiError::Conflict(msg)
            } else {
                ApiError::BadRequest(msg)
            }
        })?;

    tracing::info!(
        tenant_id = %tenant_id,
        previous = result.previous_max_workspaces,
        new = result.max_workspaces,
        current_count = result.current_workspace_count,
        "Admin updated tenant workspace quota"
    );

    Ok(Json(UpdateTenantQuotaResponse {
        tenant_id: result.tenant_id,
        max_workspaces: result.max_workspaces,
        previous_max_workspaces: result.previous_max_workspaces,
        current_workspace_count: result.current_workspace_count,
    }))
}

/// Get the server-wide default max_workspaces for new tenants.
///
/// GET /api/v1/admin/config/defaults
#[utoipa::path(
    get,
    path = "/api/v1/admin/config/defaults",
    responses(
        (status = 200, description = "Current server defaults", body = ServerDefaultsResponse),
    ),
    tags = ["admin"]
)]
pub async fn get_server_defaults(
    State(state): State<AppState>,
) -> Result<Json<ServerDefaultsResponse>, ApiError> {
    let default_max = state
        .workspace_service
        .get_server_default_max_workspaces()
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ServerDefaultsResponse {
        default_max_workspaces: default_max,
        note: None,
    }))
}

/// Update the server-wide default max_workspaces for new tenants.
///
/// Only affects newly created tenants. Not retroactive.
///
/// PATCH /api/v1/admin/config/defaults
#[utoipa::path(
    patch,
    path = "/api/v1/admin/config/defaults",
    request_body = UpdateServerDefaultsRequest,
    responses(
        (status = 200, description = "Server defaults updated", body = ServerDefaultsResponse),
        (status = 400, description = "Invalid value"),
    ),
    tags = ["admin"]
)]
pub async fn update_server_defaults(
    State(state): State<AppState>,
    Json(request): Json<UpdateServerDefaultsRequest>,
) -> Result<Json<ServerDefaultsResponse>, ApiError> {
    let new_default = state
        .workspace_service
        .set_server_default_max_workspaces(request.default_max_workspaces)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    tracing::info!(
        default = new_default,
        "Admin updated server default max_workspaces"
    );

    Ok(Json(ServerDefaultsResponse {
        default_max_workspaces: new_default,
        note: Some("Applies to newly created tenants only. Not retroactive.".to_string()),
    }))
}
