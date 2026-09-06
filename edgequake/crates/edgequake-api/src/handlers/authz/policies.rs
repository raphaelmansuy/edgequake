//! Cedar policy store (PAP) — SPEC-146 M1b.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use edgequake_authz::DEFAULT_CEDAR_SCHEMA;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiAuthenticated;
use crate::state::AppState;

use super::helpers::{bump_generation, require_manage, require_pool, require_read, sha256_hex};
use super::types::{
    CreatePolicyRequest, CreatePolicyResponse, ListPoliciesResponse, ListPolicyVersionsResponse,
    PolicyDto, PolicyVersionDto, PublishPolicyVersionRequest, PublishPolicyVersionResponse,
};

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/authz/policies",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    responses(
        (status = 200, description = "Policies", body = ListPoliciesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — missing capability"),
        (status = 404, description = "Workspace not found (existence-hiding where applicable)"),
    )
)]
pub async fn list_policies(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ListPoliciesResponse>> {
    require_read(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, i64)>(
            "SELECT policy_id, workspace_id, name, active_version FROM policies WHERE workspace_id = $1 ORDER BY name",
        )
        .bind(ws)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("list policies: {e}")))?;

        let policies = rows
            .into_iter()
            .map(|(policy_id, workspace_id, name, active_version)| PolicyDto {
                policy_id,
                workspace_id,
                name,
                active_version,
            })
            .collect();
        return Ok(Json(ListPoliciesResponse { policies }));
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
    path = "/api/v1/workspaces/{workspace_id}/authz/policies",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    request_body = CreatePolicyRequest,
    responses((status = 201, description = "Policy created", body = CreatePolicyResponse))
)]
pub async fn create_policy(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
    Json(req): Json<CreatePolicyRequest>,
) -> ApiResult<(StatusCode, Json<CreatePolicyResponse>)> {
    require_manage(&auth)?;
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let policy_id = Uuid::new_v4();
        let result = sqlx::query_as::<_, (Uuid, Uuid, String, i64)>(
            r#"
            INSERT INTO policies (policy_id, workspace_id, name, active_version)
            VALUES ($1, $2, $3, 0)
            RETURNING policy_id, workspace_id, name, active_version
            "#,
        )
        .bind(policy_id)
        .bind(ws)
        .bind(req.name.trim())
        .fetch_one(pool)
        .await;

        let (policy_id, workspace_id, name, active_version) = match result {
            Ok(r) => r,
            Err(sqlx::Error::Database(db)) if db.constraint().is_some() => {
                return Err(ApiError::Conflict("Policy name already exists".into()));
            }
            Err(e) => return Err(ApiError::Internal(format!("create policy: {e}"))),
        };

        Ok((
            StatusCode::CREATED,
            Json(CreatePolicyResponse {
                policy: PolicyDto {
                    policy_id,
                    workspace_id,
                    name,
                    active_version,
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
    get,
    path = "/api/v1/workspaces/{workspace_id}/authz/policies/{policy_id}/versions",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(
        ("workspace_id" = String, Path, description = "Workspace UUID"),
        ("policy_id" = String, Path, description = "Policy UUID"),
    ),
    responses((status = 200, description = "Policy versions", body = ListPolicyVersionsResponse))
)]
pub async fn list_policy_versions(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, policy_id)): Path<(String, String)>,
) -> ApiResult<Json<ListPolicyVersionsResponse>> {
    require_read(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;
    let pid = Uuid::parse_str(&policy_id)
        .map_err(|_| ApiError::BadRequest("Invalid policy_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        // Scope to workspace
        let owned: Option<Uuid> =
            sqlx::query_scalar("SELECT policy_id FROM policies WHERE policy_id = $1 AND workspace_id = $2")
                .bind(pid)
                .bind(ws)
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::Internal(format!("lookup policy: {e}")))?;
        if owned.is_none() {
            return Err(ApiError::NotFound("Policy not found".into()));
        }

        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                i64,
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            SELECT policy_id, version, cedar_text, cedar_hash, schema_hash, created_at
            FROM policy_versions
            WHERE policy_id = $1
            ORDER BY version DESC
            "#,
        )
        .bind(pid)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("list policy versions: {e}")))?;

        let versions = rows
            .into_iter()
            .map(
                |(policy_id, version, cedar_text, cedar_hash, schema_hash, created_at)| {
                    PolicyVersionDto {
                        policy_id,
                        version,
                        cedar_text,
                        cedar_hash,
                        schema_hash,
                        created_at: created_at.to_rfc3339(),
                    }
                },
            )
            .collect();
        return Ok(Json(ListPolicyVersionsResponse { versions }));
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, pid);
        require_pool(&state)?;
        unreachable!()
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/workspaces/{workspace_id}/authz/policies/{policy_id}/versions",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(
        ("workspace_id" = String, Path, description = "Workspace UUID"),
        ("policy_id" = String, Path, description = "Policy UUID"),
    ),
    request_body = PublishPolicyVersionRequest,
    responses((status = 201, description = "Version published", body = PublishPolicyVersionResponse))
)]
pub async fn publish_policy_version(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, policy_id)): Path<(String, String)>,
    Json(req): Json<PublishPolicyVersionRequest>,
) -> ApiResult<(StatusCode, Json<PublishPolicyVersionResponse>)> {
    require_manage(&auth)?;
    if req.cedar_text.trim().is_empty() {
        return Err(ApiError::BadRequest("cedar_text is required".into()));
    }
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;
    let pid = Uuid::parse_str(&policy_id)
        .map_err(|_| ApiError::BadRequest("Invalid policy_id".into()))?;

    edgequake_authz::parse_cedar_policy_text(req.cedar_text.trim())
        .map_err(|e| ApiError::BadRequest(format!("Invalid Cedar policy text: {e}")))?;

    let cedar_hash = sha256_hex(req.cedar_text.trim());
    let schema_hash = sha256_hex(DEFAULT_CEDAR_SCHEMA);

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| ApiError::Internal(format!("begin tx: {e}")))?;

        let owned: Option<i64> = sqlx::query_scalar(
            "SELECT active_version FROM policies WHERE policy_id = $1 AND workspace_id = $2 FOR UPDATE",
        )
        .bind(pid)
        .bind(ws)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(format!("lookup policy: {e}")))?;

        let active = owned.ok_or_else(|| ApiError::NotFound("Policy not found".into()))?;
        let next_version = active + 1;

        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                i64,
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            INSERT INTO policy_versions
              (policy_id, version, cedar_text, cedar_hash, schema_hash, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            RETURNING policy_id, version, cedar_text, cedar_hash, schema_hash, created_at
            "#,
        )
        .bind(pid)
        .bind(next_version)
        .bind(req.cedar_text.trim())
        .bind(&cedar_hash)
        .bind(&schema_hash)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ApiError::Internal(format!("insert policy version: {e}")))?;

        sqlx::query("UPDATE policies SET active_version = $1 WHERE policy_id = $2")
            .bind(next_version)
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::Internal(format!("activate policy version: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| ApiError::Internal(format!("commit: {e}")))?;

        let policy_generation = bump_generation(&state, ws).await?;
        let (policy_id, version, cedar_text, cedar_hash, schema_hash, created_at) = row;
        Ok((
            StatusCode::CREATED,
            Json(PublishPolicyVersionResponse {
                version: PolicyVersionDto {
                    policy_id,
                    version,
                    cedar_text,
                    cedar_hash,
                    schema_hash,
                    created_at: created_at.to_rfc3339(),
                },
                policy_generation,
            }),
        ))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, pid, cedar_hash, schema_hash, req);
        require_pool(&state)?;
        unreachable!()
    }
}
