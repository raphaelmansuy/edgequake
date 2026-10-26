//! Document clearance HTTP API (SPEC-174).
//!
//! These handlers load and store labels. They do not decide allow or deny.
//! `allows` already ran when the request was bound, and again inside the
//! relabel and simulate services.

#![allow(clippy::needless_return)]
// `return` inside `#[cfg(feature = "postgres")]` blocks keeps both feature builds
// type-checking, so this lint is allowed for the module.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::{ApiAuthenticated, RequestAuthContext};
use crate::middleware::TenantContext;
use crate::services::clearance;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct ClearanceSettingsResponse {
    pub enforcement: bool,
    pub policy_epoch: i64,
    pub steward: bool,
    pub rank: String,
    /// Owner or admin. Other members do not see the Access editor.
    pub can_manage: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EnableClearanceRequest {
    #[serde(default)]
    pub strict_start: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClearanceTag {
    pub id: String,
    pub kind: String,
    pub key: String,
    pub value: String,
    pub rank: Option<i16>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTagRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SecurityRoleView {
    pub id: String,
    pub name: String,
    pub steward: bool,
    /// Ordered rules, so the editor can show the saved profile.
    pub rules: Vec<RuleView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RuleView {
    pub effect: String,
    pub match_kind: String,
    pub rank: Option<i16>,
    pub tag_ids: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberRolesResponse {
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    pub steward: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PutRulesRequest {
    pub rules: Vec<RuleBody>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RuleBody {
    pub effect: String,
    pub match_kind: String,
    pub rank: Option<u8>,
    pub tag_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignRolesRequest {
    pub role_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SimulateRequest {
    pub role_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SimulateResponse {
    pub titles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PatchLabelsRequest {
    pub classification: String,
    pub compartment_ids: Vec<Uuid>,
    pub expected_policy_epoch: i64,
    pub reason: Option<String>,
}

fn workspace_of(ctx: &TenantContext, path_id: Uuid) -> Result<Uuid, ApiError> {
    let header = ctx
        .workspace_id
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| ApiError::BadRequest("Workspace is required".into()))?;
    if header != path_id {
        return Err(ApiError::forbidden_reason(
            "Target workspace does not match authorized scope",
        ));
    }
    Ok(path_id)
}

fn require_owner_or_admin(role: &str) -> Result<(), ApiError> {
    if role == "owner" || role == "admin" {
        Ok(())
    } else {
        Err(ApiError::forbidden_reason(
            "Document labels require an owner or admin",
        ))
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/clearance",
    tag = "Clearance",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    responses((status = 200, description = "Clearance settings", body = ClearanceSettingsResponse))
)]
pub async fn get_clearance_settings(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<ClearanceSettingsResponse>> {
    let _ = tenant_ctx;
    let session = edgequake_storage::clearance_session::ClearanceSession::current()
        .unwrap_or_else(edgequake_storage::clearance_session::ClearanceSession::off);
    let rank = edgequake_core::clearance::rank_name(session.rank.max(0) as u8)
        .unwrap_or(if session.rank <= 0 {
            "none"
        } else {
            "unverified"
        })
        .to_string();
    let can_manage = caller_can_manage(&state, &auth, workspace_id).await;
    Ok(Json(ClearanceSettingsResponse {
        enforcement: session.enforced,
        policy_epoch: session.policy_epoch,
        steward: session.steward,
        rank,
        can_manage,
    }))
}

async fn caller_can_manage(
    state: &AppState,
    auth: &RequestAuthContext,
    workspace_id: Uuid,
) -> bool {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = state.optional_pg_pool() else {
            return false;
        };
        let Ok(user_id) = Uuid::parse_str(&auth.user_id) else {
            return false;
        };
        let role = clearance::membership_role(pool, user_id, workspace_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();
        return role == "owner" || role == "admin";
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, auth, workspace_id);
        false
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/clearance",
    tag = "Clearance",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    request_body = EnableClearanceRequest,
    responses((status = 200, description = "Enforcement is on"))
)]
pub async fn enable_clearance(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(body): Json<EnableClearanceRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let user_id = Uuid::parse_str(&auth.user_id)
            .map_err(|_| ApiError::forbidden_reason("Enable requires a member"))?;
        let role = clearance::membership_role(pool, user_id, workspace_id)
            .await?
            .unwrap_or_default();
        if role != "owner" {
            return Err(ApiError::forbidden_reason(
                "Only an owner can turn document labels on",
            ));
        }
        clearance::enable(pool, workspace_id, user_id, body.strict_start).await?;
        return Ok(Json(serde_json::json!({ "enforcement": true })));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, auth, body);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/clearance/tags",
    tag = "Clearance",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    responses((status = 200, description = "Vocabulary", body = Vec<ClearanceTag>))
)]
pub async fn list_clearance_tags(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<Vec<ClearanceTag>>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    require_tag_manager(&state, &auth, workspace_id).await?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let rows: Vec<(Uuid, String, String, String, Option<i16>)> = sqlx::query_as(
            "SELECT tag_id, kind, key, value, rank FROM public.clearance_tags
             WHERE workspace_id = $1 ORDER BY kind, rank NULLS LAST, value",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
        return Ok(Json(
            rows.into_iter()
                .map(|(id, kind, key, value, rank)| ClearanceTag {
                    id: id.to_string(),
                    kind,
                    key,
                    value,
                    rank,
                })
                .collect(),
        ));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        Ok(Json(Vec::new()))
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/clearance/tags",
    tag = "Clearance",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    request_body = CreateTagRequest,
    responses((status = 200, description = "Department created", body = ClearanceTag))
)]
pub async fn create_clearance_tag(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(body): Json<CreateTagRequest>,
) -> ApiResult<Json<ClearanceTag>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    require_tag_manager(&state, &auth, workspace_id).await?;
    if body.key.trim().is_empty() || body.value.trim().is_empty() {
        return Err(ApiError::BadRequest("Name is required".into()));
    }
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO public.clearance_tags (tag_id, workspace_id, kind, key, value)
             VALUES ($1, $2, 'compartment', $3, $4)",
        )
        .bind(id)
        .bind(workspace_id)
        .bind(body.key.trim())
        .bind(body.value.trim())
        .execute(pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
        return Ok(Json(ClearanceTag {
            id: id.to_string(),
            kind: "compartment".into(),
            key: body.key,
            value: body.value,
            rank: None,
        }));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{workspace_id}/clearance/tags/{tag_id}",
    tag = "Clearance",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace id"),
        ("tag_id" = Uuid, Path, description = "Tag id")
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 409, description = "Tag is still in use")
    )
)]
pub async fn delete_clearance_tag(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, tag_id)): Path<(Uuid, Uuid)>,
    tenant_ctx: TenantContext,
) -> ApiResult<axum::http::StatusCode> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    require_tag_manager(&state, &auth, workspace_id).await?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let kind: Option<(String,)> = sqlx::query_as(
            "SELECT kind FROM public.clearance_tags WHERE tag_id = $1 AND workspace_id = $2",
        )
        .bind(tag_id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
        let Some((kind,)) = kind else {
            return Err(ApiError::NotFound("Tag not found".into()));
        };
        if kind == "classification" {
            return Err(ApiError::Conflict("Classification names are fixed".into()));
        }
        let result = sqlx::query(
            "DELETE FROM public.clearance_tags WHERE tag_id = $1 AND workspace_id = $2",
        )
        .bind(tag_id)
        .bind(workspace_id)
        .execute(pool)
        .await;
        return match result {
            Ok(_) => Ok(axum::http::StatusCode::NO_CONTENT),
            Err(error) => {
                let message = error.to_string();
                if message.contains("foreign key") || message.contains("restrict") {
                    Err(ApiError::Conflict(
                        "Department is still on a document".into(),
                    ))
                } else {
                    Err(ApiError::Internal(message))
                }
            }
        };
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tag_id);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

/// One joined row per rule. Roles without rules still appear once, with `None` rule columns.
#[cfg(feature = "postgres")]
type RoleRuleRow = (
    Uuid,
    String,
    bool,
    Option<String>,
    Option<String>,
    Option<i16>,
    Option<Vec<Uuid>>,
);

/// Group ordered join rows into roles. Rows for one role must be contiguous.
#[cfg(feature = "postgres")]
fn group_role_rows(rows: Vec<RoleRuleRow>) -> Vec<SecurityRoleView> {
    let mut roles: Vec<SecurityRoleView> = Vec::new();
    for (role_id, name, steward, effect, match_kind, rank, tag_ids) in rows {
        let id = role_id.to_string();
        let is_new_role = roles.last().is_none_or(|role| role.id != id);
        if is_new_role {
            roles.push(SecurityRoleView {
                id,
                name,
                steward,
                rules: Vec::new(),
            });
        }
        if let (Some(effect), Some(match_kind), Some(role)) = (effect, match_kind, roles.last_mut())
        {
            role.rules.push(RuleView {
                effect,
                match_kind,
                rank,
                tag_ids: tag_ids
                    .unwrap_or_default()
                    .iter()
                    .map(Uuid::to_string)
                    .collect(),
            });
        }
    }
    roles
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/clearance/roles",
    tag = "Clearance",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    responses((status = 200, description = "Access profiles", body = Vec<SecurityRoleView>))
)]
pub async fn list_security_roles(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<Vec<SecurityRoleView>>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    require_tag_manager(&state, &auth, workspace_id).await?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let rows: Vec<RoleRuleRow> = sqlx::query_as(
            "SELECT r.role_id, r.name, r.steward, ru.effect, ru.match_kind, ru.rank, ru.tag_ids
             FROM public.security_roles r
             LEFT JOIN public.security_role_rules ru ON ru.role_id = r.role_id
             WHERE r.workspace_id = $1
             ORDER BY r.name, r.role_id, ru.position NULLS LAST",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
        return Ok(Json(group_role_rows(rows)));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = state;
        Ok(Json(Vec::new()))
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/clearance/roles",
    tag = "Clearance",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    request_body = CreateRoleRequest,
    responses((status = 200, description = "Profile created", body = SecurityRoleView))
)]
pub async fn create_security_role(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(body): Json<CreateRoleRequest>,
) -> ApiResult<Json<SecurityRoleView>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    require_tag_manager(&state, &auth, workspace_id).await?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO public.security_roles (role_id, workspace_id, name, steward) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(workspace_id)
        .bind(body.name.trim())
        .bind(body.steward)
        .execute(pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
        return Ok(Json(SecurityRoleView {
            id: id.to_string(),
            name: body.name,
            steward: body.steward,
            rules: Vec::new(),
        }));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, body);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/clearance/roles/{role_id}/rules",
    tag = "Clearance",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace id"),
        ("role_id" = Uuid, Path, description = "Role id")
    ),
    request_body = PutRulesRequest,
    responses((status = 200, description = "Rules replaced"))
)]
pub async fn put_security_role_rules(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, role_id)): Path<(Uuid, Uuid)>,
    tenant_ctx: TenantContext,
    Json(body): Json<PutRulesRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    require_tag_manager(&state, &auth, workspace_id).await?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        sqlx::query("DELETE FROM public.security_role_rules WHERE role_id = $1")
            .bind(role_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        for (position, rule) in body.rules.iter().enumerate() {
            sqlx::query(
                "INSERT INTO public.security_role_rules
                   (rule_id, role_id, position, effect, match_kind, rank, tag_ids)
                 VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6)",
            )
            .bind(role_id)
            .bind(position as i32)
            .bind(&rule.effect)
            .bind(&rule.match_kind)
            .bind(rule.rank.map(i16::from))
            .bind(rule.tag_ids.clone().unwrap_or_default())
            .execute(&mut *tx)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        }
        sqlx::query(
            "UPDATE public.workspace_clearance_settings SET policy_epoch = policy_epoch + 1 WHERE workspace_id = $1",
        )
        .bind(workspace_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
        tx.commit()
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        let _ = auth;
        return Ok(Json(serde_json::json!({ "ok": true })));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, auth, role_id, body);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/members/{user_id}/clearance-roles",
    tag = "Clearance",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace id"),
        ("user_id" = Uuid, Path, description = "Member user id")
    ),
    responses(
        (status = 200, description = "Profiles the member holds", body = MemberRolesResponse),
        (status = 404, description = "Member not found in this workspace")
    )
)]
pub async fn get_member_clearance_roles(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, user_id)): Path<(Uuid, Uuid)>,
    tenant_ctx: TenantContext,
) -> ApiResult<Json<MemberRolesResponse>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    require_tag_manager(&state, &auth, workspace_id).await?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let membership_id = membership_id_for(pool, workspace_id, user_id).await?;
        let role_ids: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT role_id FROM public.membership_security_roles
             WHERE membership_id = $1 ORDER BY role_id",
        )
        .bind(membership_id)
        .fetch_all(pool)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
        return Ok(Json(MemberRolesResponse {
            role_ids: role_ids.into_iter().map(|(id,)| id.to_string()).collect(),
        }));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, user_id);
        Ok(Json(MemberRolesResponse {
            role_ids: Vec::new(),
        }))
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/members/{user_id}/clearance-roles",
    tag = "Clearance",
    params(
        ("workspace_id" = Uuid, Path, description = "Workspace id"),
        ("user_id" = Uuid, Path, description = "Member user id")
    ),
    request_body = AssignRolesRequest,
    responses(
        (status = 200, description = "Profiles saved"),
        (status = 404, description = "Member not found in this workspace")
    )
)]
pub async fn put_member_clearance_roles(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, user_id)): Path<(Uuid, Uuid)>,
    tenant_ctx: TenantContext,
    Json(body): Json<AssignRolesRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    require_tag_manager(&state, &auth, workspace_id).await?;
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let membership_id = membership_id_for(pool, workspace_id, user_id).await?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        sqlx::query("DELETE FROM public.membership_security_roles WHERE membership_id = $1")
            .bind(membership_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        for role_id in &body.role_ids {
            sqlx::query(
                "INSERT INTO public.membership_security_roles (membership_id, role_id) VALUES ($1, $2)",
            )
            .bind(membership_id)
            .bind(role_id)
            .execute(&mut *tx)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        }
        tx.commit()
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?;
        return Ok(Json(serde_json::json!({ "ok": true })));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, user_id, body);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

/// Resolve the active membership row for a member of this workspace.
#[cfg(feature = "postgres")]
async fn membership_id_for(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    user_id: Uuid,
) -> Result<Uuid, ApiError> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT membership_id FROM public.memberships
         WHERE workspace_id = $1 AND user_id = $2 AND is_active LIMIT 1",
    )
    .bind(workspace_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    row.map(|(id,)| id)
        .ok_or_else(|| ApiError::NotFound("Member not found".into()))
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/clearance/simulate",
    tag = "Clearance",
    params(("workspace_id" = Uuid, Path, description = "Workspace id")),
    request_body = SimulateRequest,
    responses((status = 200, description = "Titles this profile can open", body = SimulateResponse))
)]
pub async fn simulate_clearance(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    tenant_ctx: TenantContext,
    Json(body): Json<SimulateRequest>,
) -> ApiResult<Json<SimulateResponse>> {
    let workspace_id = workspace_of(&tenant_ctx, workspace_id)?;
    let session = edgequake_storage::clearance_session::ClearanceSession::current()
        .unwrap_or_else(edgequake_storage::clearance_session::ClearanceSession::off);
    if !session.steward {
        return Err(ApiError::forbidden_reason(
            "Only a steward can preview an access profile",
        ));
    }
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let titles = simulate_titles(pool, workspace_id, &body.role_ids).await?;
        crate::services::audit::record_compliance_event(
            &state,
            tenant_ctx.tenant_id.clone().unwrap_or_default(),
            edgequake_audit::AuditEventType::Authorization,
            "clearance.simulate",
            edgequake_audit::AuditResult::Success,
            Some(workspace_id.to_string()),
            Some(auth.user_id.clone()),
            None,
        );
        // simulate_titles fetches at most 21 rows: an exact count up to 20, else unknown.
        let count = if titles.len() <= 20 {
            Some(titles.len() as i64)
        } else {
            None
        };
        let titles = titles.into_iter().take(20).collect();
        return Ok(Json(SimulateResponse { titles, count }));
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, auth, body);
        Ok(Json(SimulateResponse {
            titles: Vec::new(),
            count: Some(0),
        }))
    }
}

#[utoipa::path(
    patch,
    path = "/api/v1/documents/{document_id}/labels",
    tag = "Clearance",
    params(("document_id" = Uuid, Path, description = "Document id")),
    request_body = PatchLabelsRequest,
    responses(
        (status = 204, description = "Label saved"),
        (status = 404, description = "Document not found")
    )
)]
pub async fn patch_document_labels(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(document_id): Path<Uuid>,
    mut tenant_ctx: TenantContext,
    headers: HeaderMap,
    Json(body): Json<PatchLabelsRequest>,
) -> ApiResult<(HeaderMap, axum::http::StatusCode)> {
    tenant_ctx.user_id = Some(auth.user_id);
    #[cfg(feature = "postgres")]
    {
        use crate::services::{document_access as access, document_access_types::DocumentAccess};
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let version = headers
            .get("if-match")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("\"label-"))
            .and_then(|v| v.strip_suffix('"'))
            .and_then(|v| v.parse::<i64>().ok())
            .ok_or_else(|| {
                access::coded(
                    axum::http::StatusCode::PRECONDITION_REQUIRED,
                    "LABEL_PRECONDITION_REQUIRED",
                    "Refresh access and send its current version",
                )
            })?;
        let operation_id = headers
            .get("idempotency-key")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| Uuid::parse_str(v).ok())
            .ok_or_else(|| {
                ApiError::bad_request_coded(
                    "INVALID_ACCESS_PAYLOAD",
                    "An operation key is required",
                )
            })?;
        let outcome = access::mutate(
            pool,
            &access::AccessActor::from_context(&tenant_ctx)?,
            access::AccessMutation {
                items: vec![access::AccessMutationItem {
                    document_id,
                    label_version: version,
                    access: DocumentAccess {
                        classification: body.classification,
                        compartment_ids: body.compartment_ids,
                        expected_policy_epoch: body.expected_policy_epoch,
                    },
                }],
                reason: body.reason,
                operation_id,
            },
        )
        .await?;
        let mut response_headers = HeaderMap::new();
        response_headers.insert(
            "etag",
            format!(
                "\"label-{}\"",
                outcome["items"][0]["label_version"]
                    .as_i64()
                    .unwrap_or(version)
            )
            .parse()
            .map_err(|_| ApiError::Internal("Invalid access version".into()))?,
        );
        response_headers.insert(
            "x-policy-epoch",
            outcome["policy_epoch"]
                .to_string()
                .parse()
                .map_err(|_| ApiError::Internal("Invalid policy epoch".into()))?,
        );
        return Ok((response_headers, axum::http::StatusCode::NO_CONTENT));
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, tenant_ctx, headers, body, document_id);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

async fn require_tag_manager(
    state: &AppState,
    auth: &RequestAuthContext,
    workspace_id: Uuid,
) -> Result<(), ApiError> {
    #[cfg(feature = "postgres")]
    {
        let Some(pool) = state.optional_pg_pool() else {
            return Ok(());
        };
        let Ok(user_id) = Uuid::parse_str(&auth.user_id) else {
            return Err(ApiError::forbidden_reason(
                "Document labels require an owner or admin",
            ));
        };
        let role = clearance::membership_role(pool, user_id, workspace_id)
            .await?
            .unwrap_or_default();
        return require_owner_or_admin(&role);
    }

    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, auth, workspace_id);
        Ok(())
    }
}

/// One joined row: role id, steward placeholder, then one rule's columns.
#[cfg(feature = "postgres")]
type RuleJoinRow = (
    Uuid,
    bool,
    Option<String>,
    Option<String>,
    Option<i16>,
    Option<Vec<Uuid>>,
);

#[cfg(feature = "postgres")]
async fn simulate_titles(
    pool: &sqlx::PgPool,
    workspace_id: Uuid,
    role_ids: &[Uuid],
) -> Result<Vec<String>, ApiError> {
    let rows: Vec<RuleJoinRow> = sqlx::query_as(
        "SELECT r.role_id, false AS steward, ru.effect, ru.match_kind, ru.rank, ru.tag_ids
         FROM public.security_roles r
         LEFT JOIN public.security_role_rules ru ON ru.role_id = r.role_id
         WHERE r.workspace_id = $1 AND r.role_id = ANY($2)
         ORDER BY r.role_id, ru.position",
    )
    .bind(workspace_id)
    .bind(role_ids)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let _ = rows;
    // Compile without the steward bit, then let clearance_covers do the read.
    let grants = compiled_without_steward(&rows);
    let mut subject = edgequake_core::clearance::compile_subject(&grants, None);
    subject.enforcement = true;
    subject.break_glass = false;
    subject.steward = false;
    subject.holds_all_compartments = false;
    let vocabulary: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT tag_id FROM public.clearance_tags WHERE workspace_id = $1 AND kind = 'compartment'",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    let vocabulary: Vec<Uuid> = vocabulary.into_iter().map(|(id,)| id).collect();
    let scope = edgequake_core::clearance::VisibilityScope::new(subject, 0);
    let mut session = edgequake_storage::clearance_session::ClearanceSession::off();
    session.enforced = true;
    session.rank = i16::from(scope.subject.rank);
    session.compartments =
        edgequake_core::clearance::guc_compartment_ids(&scope.subject, &vocabulary)
            .into_iter()
            .collect();
    session.allow_empty = scope.subject.allow_empty_compartments;
    session.steward = false;
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    edgequake_storage::adapters::postgres::rls::apply_clearance_session(&mut tx, &session)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    let titles: Vec<(String,)> = sqlx::query_as(
        "SELECT COALESCE(NULLIF(title, ''), id::text)
         FROM public.documents
         WHERE workspace_id = $1
           AND public.clearance_covers(classification_rank, compartment_ids)
         ORDER BY title
         LIMIT 21",
    )
    .bind(workspace_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|error| ApiError::Internal(error.to_string()))?;
    tx.commit()
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(titles.into_iter().map(|(title,)| title).collect())
}

#[cfg(feature = "postgres")]
fn compiled_without_steward(rows: &[RuleJoinRow]) -> Vec<edgequake_core::clearance::RoleGrant> {
    use edgequake_core::clearance::{ClearanceRule, Effect, MatchKind, RoleGrant};
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<Uuid, RoleGrant> = BTreeMap::new();
    for (role_id, _, effect, kind, rank, tags) in rows {
        let grant = grouped.entry(*role_id).or_insert(RoleGrant {
            steward: false,
            rules: Vec::new(),
        });
        let Some(effect) = effect.as_deref() else {
            continue;
        };
        let effect = match effect {
            "allow" => Effect::Allow,
            "deny" => Effect::Deny,
            _ => continue,
        };
        let kind = match kind.as_deref() {
            Some("classification_at_most") => MatchKind::ClassificationAtMost {
                rank: rank.and_then(|rank| u8::try_from(rank).ok()).unwrap_or(0),
            },
            Some("any_of") => MatchKind::AnyOf {
                tag_ids: tags.clone().unwrap_or_default().into_iter().collect(),
            },
            Some("all_of") => MatchKind::AllOf {
                tag_ids: tags.clone().unwrap_or_default().into_iter().collect(),
            },
            Some("untagged") => MatchKind::Untagged,
            _ => continue,
        };
        grant.rules.push(ClearanceRule { effect, kind });
    }
    grouped.into_values().collect()
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;

    fn rule_row(role: Uuid, name: &str, effect: Option<&str>, kind: Option<&str>) -> RoleRuleRow {
        (
            role,
            name.to_string(),
            false,
            effect.map(str::to_string),
            kind.map(str::to_string),
            Some(2),
            Some(vec![Uuid::nil()]),
        )
    }

    #[test]
    fn group_role_rows_keeps_rules_in_order_per_role() {
        let hr = Uuid::from_u128(1);
        let rd = Uuid::from_u128(2);
        let roles = group_role_rows(vec![
            rule_row(
                hr,
                "HR reader",
                Some("allow"),
                Some("classification_at_most"),
            ),
            rule_row(hr, "HR reader", Some("allow"), Some("any_of")),
            rule_row(hr, "HR reader", Some("deny"), Some("any_of")),
            rule_row(rd, "R&D reader", None, None),
        ]);
        assert_eq!(roles.len(), 2);
        assert_eq!(roles[0].name, "HR reader");
        assert_eq!(roles[0].rules.len(), 3);
        assert_eq!(roles[0].rules[2].effect, "deny");
        assert_eq!(roles[0].rules[0].tag_ids, vec![Uuid::nil().to_string()]);
    }

    #[test]
    fn group_role_rows_lists_a_role_with_no_rules() {
        let rd = Uuid::from_u128(2);
        let roles = group_role_rows(vec![rule_row(rd, "R&D reader", None, None)]);
        assert_eq!(roles.len(), 1);
        assert!(roles[0].rules.is_empty());
    }
}
