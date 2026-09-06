//! Workspace roles CRUD (SPEC-146 M1b).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiAuthenticated;
use crate::state::AppState;

use super::helpers::{bump_generation, require_manage, require_pool, require_read};
use super::types::{
    CreateRoleRequest, CreateRoleResponse, ListRolesResponse, WorkspaceRoleDto,
};

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/authz/roles",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    responses(
        (status = 200, description = "Workspace roles", body = ListRolesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — missing capability"),
        (status = 404, description = "Workspace not found (existence-hiding where applicable)"),
    )
)]
pub async fn list_workspace_roles(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ListRolesResponse>> {
    require_read(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        ensure_builtin_workspace_roles(pool, ws).await?;
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, bool, Vec<String>)>(
            r#"
            SELECT role_id, workspace_id, name, is_builtin, permissions
            FROM workspace_roles
            WHERE workspace_id = $1
            ORDER BY name
            "#,
        )
        .bind(ws)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("list roles: {e}")))?;

        let roles = rows
            .into_iter()
            .map(|(role_id, workspace_id, name, is_builtin, permissions)| WorkspaceRoleDto {
                role_id,
                workspace_id,
                name,
                is_builtin,
                permissions,
            })
            .collect();
        return Ok(Json(ListRolesResponse { roles }));
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws,);
        require_pool(&state)?;
        unreachable!()
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/authz/roles",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Role created", body = CreateRoleResponse),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Role name exists"),
    )
)]
pub async fn create_workspace_role(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
    Json(req): Json<CreateRoleRequest>,
) -> ApiResult<(StatusCode, Json<CreateRoleResponse>)> {
    require_manage(&auth)?;
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let role_id = Uuid::new_v4();
        let result = sqlx::query_as::<_, (Uuid, Uuid, String, bool, Vec<String>)>(
            r#"
            INSERT INTO workspace_roles (role_id, workspace_id, name, is_builtin, permissions)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING role_id, workspace_id, name, is_builtin, permissions
            "#,
        )
        .bind(role_id)
        .bind(ws)
        .bind(req.name.trim())
        .bind(req.is_builtin)
        .bind(&req.permissions)
        .fetch_one(pool)
        .await;

        let (role_id, workspace_id, name, is_builtin, permissions) = match result {
            Ok(r) => r,
            Err(sqlx::Error::Database(db)) if db.constraint().is_some() => {
                return Err(ApiError::Conflict("Role name already exists".into()));
            }
            Err(e) => return Err(ApiError::Internal(format!("create role: {e}"))),
        };

        let _ = bump_generation(&state, ws).await?;
        Ok((
            StatusCode::CREATED,
            Json(CreateRoleResponse {
                role: WorkspaceRoleDto {
                    role_id,
                    workspace_id,
                    name,
                    is_builtin,
                    permissions,
                },
            }),
        ))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, req);
        require_pool(&state)?;
        unreachable!()
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{workspace_id}/authz/roles/{role_id}",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(
        ("workspace_id" = String, Path, description = "Workspace UUID"),
        ("role_id" = String, Path, description = "Role UUID"),
    ),
    responses(
        (status = 204, description = "Role deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn delete_workspace_role(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, role_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    require_manage(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;
    let rid = Uuid::parse_str(&role_id)
        .map_err(|_| ApiError::BadRequest("Invalid role_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let rows = sqlx::query(
            r#"
            DELETE FROM workspace_roles
            WHERE workspace_id = $1 AND role_id = $2 AND is_builtin = FALSE
            "#,
        )
        .bind(ws)
        .bind(rid)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("delete role: {e}")))?
        .rows_affected();

        if rows == 0 {
            return Err(ApiError::NotFound("Role not found or is builtin".into()));
        }
        let _ = bump_generation(&state, ws).await?;
        Ok(StatusCode::NO_CONTENT)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, rid);
        require_pool(&state)?;
        unreachable!()
    }
}

/// Seed viewer / editor / admin builtin roles on first list (empty Select fix).
#[cfg(feature = "postgres")]
async fn ensure_builtin_workspace_roles(pool: &sqlx::PgPool, workspace_id: Uuid) -> ApiResult<()> {
    let builtins: &[(&str, &[&str])] = &[
        (
            "viewer",
            &["document:list_meta", "document:read", "graph:read"],
        ),
        (
            "editor",
            &[
                "document:list_meta",
                "document:read",
                "document:write",
                "graph:read",
            ],
        ),
        (
            "admin",
            &[
                "document:list_meta",
                "document:read",
                "document:write",
                "document:delete",
                "graph:read",
                "policy:manage",
                "system:break_glass",
            ],
        ),
    ];
    for (name, perms) in builtins {
        let perms_vec: Vec<String> = perms.iter().map(|s| (*s).to_string()).collect();
        sqlx::query(
            r#"
            INSERT INTO workspace_roles (role_id, workspace_id, name, is_builtin, permissions)
            VALUES (gen_random_uuid(), $1, $2, TRUE, $3)
            ON CONFLICT (workspace_id, name) DO NOTHING
            "#,
        )
        .bind(workspace_id)
        .bind(*name)
        .bind(&perms_vec)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("seed builtin role {name}: {e}")))?;
    }
    Ok(())
}
