//! Workspace-scoped grants (SPEC-181 LAW-181-10).
//!
//! The pre-181 product expressed a workspace grant in storage (`memberships.workspace_id`),
//! enforced it (`Membership::applies_to`), and reported it (`me/access`), but had **no route
//! that could write one**. The only way to let someone administer one workspace was to make them
//! an organization Admin, which also gave them every other workspace
//! ([F-181-05](../../../../specs/181-admin/04-findings.md)).
//!
//! These routes close that: list, add, change and remove a grant in either scope, with the same
//! rank rules the rest of the member surface uses.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use edgequake_core::MembershipRole;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::directory::{actor_for, grant_scope_ok, require_tenant, tenant_memberships};
use super::guards::check_assign;
use crate::error::ApiError;
use crate::handlers::auth::ApiAuthenticated;
use crate::handlers::workspaces::tenant_access::{require_tenant_access, TenantAccess};
use crate::state::AppState;

/// One grant, as the grant editor renders it.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GrantView {
    pub membership_id: String,
    pub user_id: String,
    /// `organization` or `workspace`.
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    pub role: String,
    pub level: u8,
    pub is_active: bool,
    /// `true` when an IdP owns the role and a manual edit would pin it.
    pub sso_managed: bool,
    pub joined_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListGrantsQuery {
    pub user_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateGrantRequest {
    pub user_id: Uuid,
    /// Omit for an organization grant.
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    /// `owner`, `admin`, `member`, `readonly`.
    pub role: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateGrantRequest {
    pub role: String,
}

fn view(m: &edgequake_core::Membership, workspace_name: Option<String>) -> GrantView {
    GrantView {
        membership_id: m.membership_id.to_string(),
        user_id: m.user_id.to_string(),
        scope: if m.workspace_id.is_some() {
            "workspace".into()
        } else {
            "organization".into()
        },
        workspace_id: m.workspace_id.map(|w| w.to_string()),
        workspace_name,
        role: m.role.to_string(),
        level: m.role.level(),
        is_active: m.is_active,
        sso_managed: m.is_sso_managed(),
        joined_at: m.joined_at.to_rfc3339(),
    }
}

/// `GET /api/v1/tenants/{tenant_id}/grants`
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/grants",
    tag = "Members",
    params(
        ("tenant_id" = Uuid, Path, description = "Organization id"),
        ("user_id" = Option<Uuid>, Query, description = "Only this person's grants"),
        ("workspace_id" = Option<Uuid>, Query, description = "Only grants on this workspace"),
    ),
    responses((status = 200, description = "Grants", body = Vec<GrantView>)),
    security(("bearer_auth" = []))
)]
pub async fn list_grants(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    Query(query): Query<ListGrantsQuery>,
) -> Result<Json<Vec<GrantView>>, ApiError> {
    require_tenant_access(&state, auth.context(), tenant_id, TenantAccess::Manage).await?;
    let memberships = tenant_memberships(&state, tenant_id).await?;
    let workspaces = state
        .workspace_service
        .list_workspaces(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let rows: Vec<GrantView> = memberships
        .iter()
        .filter(|m| m.is_active)
        .filter(|m| query.user_id.is_none_or(|u| m.user_id == u))
        .filter(|m| query.workspace_id.is_none_or(|w| m.workspace_id == Some(w)))
        .map(|m| {
            let name = m
                .workspace_id
                .and_then(|w| workspaces.iter().find(|x| x.workspace_id == w))
                .map(|w| w.name.clone());
            view(m, name)
        })
        .collect();
    Ok(Json(rows))
}

/// `POST /api/v1/tenants/{tenant_id}/grants`
#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/grants",
    tag = "Members",
    params(("tenant_id" = Uuid, Path, description = "Organization id")),
    request_body = CreateGrantRequest,
    responses(
        (status = 201, description = "Grant created", body = GrantView),
        (status = 409, description = "The person already holds this grant"),
        (status = 400, description = "Scope does not belong to this organization"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_grant(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<CreateGrantRequest>,
) -> Result<(StatusCode, Json<GrantView>), ApiError> {
    let ctx = auth.context();
    require_tenant_access(&state, ctx, tenant_id, TenantAccess::Manage).await?;
    require_tenant(&state, tenant_id).await?;

    let role: MembershipRole = body.role.parse().map_err(ApiError::BadRequest)?;
    // EC-181-24: a workspace grant must name a workspace of *this* organization, or a caller
    // could attach a grant to a sibling that the rank check would then happily permit.
    grant_scope_ok(&state, tenant_id, body.workspace_id).await?;

    let actor = actor_for(&state, ctx, tenant_id).await?;
    // LAW-181-10: you may grant at or below your own rank in the target scope. An organization
    // Admin may therefore grant workspace Admin without being able to grant organization Owner.
    check_assign(&actor, MembershipRole::Readonly, role, false)?;

    let mut membership = edgequake_core::Membership::new(body.user_id, tenant_id, role);
    if let Some(ws) = body.workspace_id {
        membership = membership.for_workspace(ws);
    }
    let created = state
        .workspace_service
        .add_membership(membership)
        .await
        .map_err(|e| match e {
            edgequake_core::Error::Validation(m) if m.contains("already exists") => {
                ApiError::Conflict("grant_exists".into())
            }
            other => ApiError::BadRequest(other.to_string()),
        })?;

    let name = match created.workspace_id {
        Some(ws) => state
            .workspace_service
            .get_workspace(ws)
            .await
            .ok()
            .flatten()
            .map(|w| w.name),
        None => None,
    };
    Ok((StatusCode::CREATED, Json(view(&created, name))))
}

/// `PATCH /api/v1/tenants/{tenant_id}/grants/{membership_id}`
#[utoipa::path(
    patch,
    path = "/api/v1/tenants/{tenant_id}/grants/{membership_id}",
    tag = "Members",
    params(
        ("tenant_id" = Uuid, Path, description = "Organization id"),
        ("membership_id" = Uuid, Path, description = "Grant id"),
    ),
    request_body = UpdateGrantRequest,
    responses(
        (status = 200, description = "Grant updated", body = GrantView),
        (status = 409, description = "Would orphan the organization"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_grant(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path((tenant_id, membership_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateGrantRequest>,
) -> Result<Json<GrantView>, ApiError> {
    let ctx = auth.context();
    require_tenant_access(&state, ctx, tenant_id, TenantAccess::Manage).await?;

    let memberships = tenant_memberships(&state, tenant_id).await?;
    let current = memberships
        .iter()
        .find(|m| m.membership_id == membership_id && m.is_active)
        .ok_or_else(|| ApiError::NotFound("Grant not found".into()))?;

    let new_role: MembershipRole = body.role.parse().map_err(ApiError::BadRequest)?;
    let actor = actor_for(&state, ctx, tenant_id).await?;
    let is_last_owner = super::directory::last_owner(&memberships) == Some(current.user_id);
    check_assign(&actor, current.role, new_role, is_last_owner)?;

    let updated = state
        .workspace_service
        .assign_membership_role(membership_id, new_role)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(view(&updated, None)))
}

/// `DELETE /api/v1/tenants/{tenant_id}/grants/{membership_id}`
#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{tenant_id}/grants/{membership_id}",
    tag = "Members",
    params(
        ("tenant_id" = Uuid, Path, description = "Organization id"),
        ("membership_id" = Uuid, Path, description = "Grant id"),
    ),
    responses(
        (status = 204, description = "Grant removed"),
        (status = 409, description = "Would orphan the organization"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_grant(
    State(state): State<AppState>,
    auth: ApiAuthenticated,
    Path((tenant_id, membership_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let ctx = auth.context();
    require_tenant_access(&state, ctx, tenant_id, TenantAccess::Manage).await?;

    let memberships = tenant_memberships(&state, tenant_id).await?;
    let current = memberships
        .iter()
        .find(|m| m.membership_id == membership_id && m.is_active)
        .ok_or_else(|| ApiError::NotFound("Grant not found".into()))?;

    let actor = actor_for(&state, ctx, tenant_id).await?;
    let is_last_owner = super::directory::last_owner(&memberships) == Some(current.user_id);
    let is_self = ctx.user_id == current.user_id.to_string();
    super::guards::check_remove(&actor, current.role, is_last_owner, is_self)?;

    state
        .workspace_service
        .remove_membership(membership_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}
