//! Workspace role bindings (member ↔ role) — SPEC-146 M1b.

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
    CreateRoleBindingRequest, CreateRoleBindingResponse, ListRoleBindingsResponse, RoleBindingDto,
};

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/authz/members",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    responses(
        (status = 200, description = "Role bindings", body = ListRoleBindingsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — missing capability"),
        (status = 404, description = "Workspace not found (existence-hiding where applicable)"),
    )
)]
pub async fn list_role_bindings(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ListRoleBindingsResponse>> {
    require_read(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let rows = sqlx::query_as::<_, (Uuid, String, String, Uuid, Option<String>)>(
            r#"
            SELECT b.workspace_id, b.principal_kind, b.principal_id, b.role_id, r.name
            FROM workspace_role_bindings b
            LEFT JOIN workspace_roles r ON r.role_id = b.role_id
            WHERE b.workspace_id = $1
            ORDER BY b.principal_kind, b.principal_id
            "#,
        )
        .bind(ws)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("list bindings: {e}")))?;

        let bindings = rows
            .into_iter()
            .map(
                |(workspace_id, principal_kind, principal_id, role_id, role_name)| RoleBindingDto {
                    workspace_id,
                    principal_kind,
                    principal_id,
                    role_id,
                    role_name,
                },
            )
            .collect();
        return Ok(Json(ListRoleBindingsResponse { bindings }));
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
    path = "/api/v1/workspaces/{workspace_id}/authz/members",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    request_body = CreateRoleBindingRequest,
    responses(
        (status = 201, description = "Binding created", body = CreateRoleBindingResponse),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Already bound"),
    )
)]
pub async fn create_role_binding(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
    Json(req): Json<CreateRoleBindingRequest>,
) -> ApiResult<(StatusCode, Json<CreateRoleBindingResponse>)> {
    require_manage(&auth)?;
    if req.principal_kind.trim().is_empty() || req.principal_id.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "principal_kind and principal_id are required".into(),
        ));
    }
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        // Ensure role belongs to workspace
        let role_ok: Option<String> = sqlx::query_scalar(
            "SELECT name FROM workspace_roles WHERE workspace_id = $1 AND role_id = $2",
        )
        .bind(ws)
        .bind(req.role_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("lookup role: {e}")))?;

        let role_name = role_ok.ok_or_else(|| ApiError::NotFound("Role not found".into()))?;

        let result = sqlx::query(
            r#"
            INSERT INTO workspace_role_bindings
              (workspace_id, principal_kind, principal_id, role_id)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(ws)
        .bind(req.principal_kind.trim())
        .bind(req.principal_id.trim())
        .bind(req.role_id)
        .execute(pool)
        .await;

        if let Err(sqlx::Error::Database(db)) = &result {
            if db.constraint().is_some() {
                return Err(ApiError::Conflict("Binding already exists".into()));
            }
        }
        result.map_err(|e| ApiError::Internal(format!("create binding: {e}")))?;

        let _ = bump_generation(&state, ws).await?;
        Ok((
            StatusCode::CREATED,
            Json(CreateRoleBindingResponse {
                binding: RoleBindingDto {
                    workspace_id: ws,
                    principal_kind: req.principal_kind.trim().to_string(),
                    principal_id: req.principal_id.trim().to_string(),
                    role_id: req.role_id,
                    role_name: Some(role_name),
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
    path = "/api/v1/workspaces/{workspace_id}/authz/members/{principal_kind}/{principal_id}/{role_id}",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(
        ("workspace_id" = String, Path, description = "Workspace UUID"),
        ("principal_kind" = String, Path, description = "Principal kind"),
        ("principal_id" = String, Path, description = "Principal id"),
        ("role_id" = String, Path, description = "Role UUID"),
    ),
    responses(
        (status = 204, description = "Binding deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn delete_role_binding(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, principal_kind, principal_id, role_id)): Path<(
        String,
        String,
        String,
        String,
    )>,
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
            DELETE FROM workspace_role_bindings
            WHERE workspace_id = $1
              AND principal_kind = $2
              AND principal_id = $3
              AND role_id = $4
            "#,
        )
        .bind(ws)
        .bind(&principal_kind)
        .bind(&principal_id)
        .bind(rid)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("delete binding: {e}")))?
        .rows_affected();

        if rows == 0 {
            return Err(ApiError::NotFound("Binding not found".into()));
        }
        let _ = bump_generation(&state, ws).await?;
        Ok(StatusCode::NO_CONTENT)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, principal_kind, principal_id, rid);
        require_pool(&state)?;
        unreachable!()
    }
}
