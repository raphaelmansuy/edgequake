//! Attribute catalog + principal attributes — SPEC-146 M1b.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiAuthenticated;
use crate::state::AppState;

use super::helpers::{bump_generation, require_manage, require_pool, require_read};
use super::types::{
    AttributeDefinitionDto, CreateAttributeDefinitionRequest, CreateAttributeDefinitionResponse,
    ListAttributeDefinitionsResponse, ListPrincipalAttributesResponse, PrincipalAttributeDto,
    UpsertPrincipalAttributeRequest, UpsertPrincipalAttributeResponse,
};

#[derive(Debug, Deserialize)]
pub struct PrincipalAttrQuery {
    pub principal_kind: Option<String>,
    pub principal_id: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/authz/attribute-definitions",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    responses(
        (status = 200, description = "Attribute definitions", body = ListAttributeDefinitionsResponse),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn list_attribute_definitions(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ListAttributeDefinitionsResponse>> {
    require_read(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                String,
                Option<serde_json::Value>,
                Vec<String>,
            ),
        >(
            r#"
            SELECT attr_id, workspace_id, scope, name, value_type, enum_values,
                   COALESCE(required_for_share_modes, '{}')
            FROM attribute_definitions
            WHERE workspace_id = $1
            ORDER BY scope, name
            "#,
        )
        .bind(ws)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("list attr defs: {e}")))?;

        let attributes = rows
            .into_iter()
            .map(
                |(
                    attr_id,
                    workspace_id,
                    scope,
                    name,
                    value_type,
                    enum_values,
                    required_for_share_modes,
                )| AttributeDefinitionDto {
                    attr_id,
                    workspace_id,
                    scope,
                    name,
                    value_type,
                    enum_values,
                    required_for_share_modes,
                },
            )
            .collect();
        return Ok(Json(ListAttributeDefinitionsResponse { attributes }));
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
    path = "/api/v1/workspaces/{workspace_id}/authz/attribute-definitions",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    request_body = CreateAttributeDefinitionRequest,
    responses(
        (status = 201, description = "Attribute definition created", body = CreateAttributeDefinitionResponse),
        (status = 403, description = "Forbidden"),
    )
)]
pub async fn create_attribute_definition(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
    Json(req): Json<CreateAttributeDefinitionRequest>,
) -> ApiResult<(StatusCode, Json<CreateAttributeDefinitionResponse>)> {
    require_manage(&auth)?;
    if req.name.trim().is_empty() || req.scope.trim().is_empty() {
        return Err(ApiError::BadRequest("scope and name are required".into()));
    }
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let attr_id = Uuid::new_v4();
        let result = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                String,
                Option<serde_json::Value>,
                Vec<String>,
            ),
        >(
            r#"
            INSERT INTO attribute_definitions
              (attr_id, workspace_id, scope, name, value_type, enum_values, required_for_share_modes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING attr_id, workspace_id, scope, name, value_type, enum_values,
                      COALESCE(required_for_share_modes, '{}')
            "#,
        )
        .bind(attr_id)
        .bind(ws)
        .bind(req.scope.trim())
        .bind(req.name.trim())
        .bind(req.value_type.trim())
        .bind(&req.enum_values)
        .bind(&req.required_for_share_modes)
        .fetch_one(pool)
        .await;

        let row = match result {
            Ok(r) => r,
            Err(sqlx::Error::Database(db)) if db.constraint().is_some() => {
                return Err(ApiError::Conflict("Attribute definition already exists".into()));
            }
            Err(e) => return Err(ApiError::Internal(format!("create attr def: {e}"))),
        };

        let _ = bump_generation(&state, ws).await?;
        let (
            attr_id,
            workspace_id,
            scope,
            name,
            value_type,
            enum_values,
            required_for_share_modes,
        ) = row;
        Ok((
            StatusCode::CREATED,
            Json(CreateAttributeDefinitionResponse {
                attribute: AttributeDefinitionDto {
                    attr_id,
                    workspace_id,
                    scope,
                    name,
                    value_type,
                    enum_values,
                    required_for_share_modes,
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
    path = "/api/v1/workspaces/{workspace_id}/authz/attribute-definitions/{attr_id}",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(
        ("workspace_id" = String, Path, description = "Workspace UUID"),
        ("attr_id" = String, Path, description = "Attribute definition UUID"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn delete_attribute_definition(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, attr_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    require_manage(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;
    let aid = Uuid::parse_str(&attr_id)
        .map_err(|_| ApiError::BadRequest("Invalid attr_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let rows = sqlx::query(
            "DELETE FROM attribute_definitions WHERE workspace_id = $1 AND attr_id = $2",
        )
        .bind(ws)
        .bind(aid)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("delete attr def: {e}")))?
        .rows_affected();
        if rows == 0 {
            return Err(ApiError::NotFound("Attribute definition not found".into()));
        }
        let _ = bump_generation(&state, ws).await?;
        Ok(StatusCode::NO_CONTENT)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, aid);
        require_pool(&state)?;
        unreachable!()
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/authz/principal-attributes",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(
        ("workspace_id" = String, Path, description = "Workspace UUID"),
        ("principal_kind" = Option<String>, Query, description = "Filter by principal kind"),
        ("principal_id" = Option<String>, Query, description = "Filter by principal id"),
    ),
    responses(
        (status = 200, description = "Principal attributes", body = ListPrincipalAttributesResponse),
    )
)]
pub async fn list_principal_attributes(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
    Query(q): Query<PrincipalAttrQuery>,
) -> ApiResult<Json<ListPrincipalAttributesResponse>> {
    require_read(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                serde_json::Value,
                String,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            SELECT workspace_id, principal_kind, principal_id, name, value, source, updated_at
            FROM principal_attributes
            WHERE workspace_id = $1
              AND ($2::text IS NULL OR principal_kind = $2)
              AND ($3::text IS NULL OR principal_id = $3)
            ORDER BY principal_kind, principal_id, name
            "#,
        )
        .bind(ws)
        .bind(q.principal_kind.as_deref())
        .bind(q.principal_id.as_deref())
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("list principal attrs: {e}")))?;

        let attributes = rows
            .into_iter()
            .map(
                |(
                    workspace_id,
                    principal_kind,
                    principal_id,
                    name,
                    value,
                    source,
                    updated_at,
                )| PrincipalAttributeDto {
                    workspace_id,
                    principal_kind,
                    principal_id,
                    name,
                    value,
                    source,
                    updated_at: updated_at.to_rfc3339(),
                },
            )
            .collect();
        return Ok(Json(ListPrincipalAttributesResponse { attributes }));
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, q);
        require_pool(&state)?;
        unreachable!()
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/workspaces/{workspace_id}/authz/principal-attributes",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    request_body = UpsertPrincipalAttributeRequest,
    responses(
        (status = 200, description = "Attribute upserted", body = UpsertPrincipalAttributeResponse),
    )
)]
pub async fn upsert_principal_attribute(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
    Json(req): Json<UpsertPrincipalAttributeRequest>,
) -> ApiResult<Json<UpsertPrincipalAttributeResponse>> {
    require_manage(&auth)?;
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name is required".into()));
    }
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                serde_json::Value,
                String,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            INSERT INTO principal_attributes
              (workspace_id, principal_kind, principal_id, name, value, source, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (workspace_id, principal_kind, principal_id, name) DO UPDATE
              SET value = EXCLUDED.value,
                  source = EXCLUDED.source,
                  updated_at = NOW()
            RETURNING workspace_id, principal_kind, principal_id, name, value, source, updated_at
            "#,
        )
        .bind(ws)
        .bind(req.principal_kind.trim())
        .bind(req.principal_id.trim())
        .bind(req.name.trim())
        .bind(&req.value)
        .bind(if req.source.is_empty() {
            "manual"
        } else {
            req.source.trim()
        })
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("upsert principal attr: {e}")))?;

        let _ = bump_generation(&state, ws).await?;
        let (workspace_id, principal_kind, principal_id, name, value, source, updated_at) = row;
        Ok(Json(UpsertPrincipalAttributeResponse {
            attribute: PrincipalAttributeDto {
                workspace_id,
                principal_kind,
                principal_id,
                name,
                value,
                source,
                updated_at: updated_at.to_rfc3339(),
            },
        }))
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
    path = "/api/v1/workspaces/{workspace_id}/authz/principal-attributes/{principal_kind}/{principal_id}/{name}",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(
        ("workspace_id" = String, Path, description = "Workspace UUID"),
        ("principal_kind" = String, Path, description = "Principal kind"),
        ("principal_id" = String, Path, description = "Principal id"),
        ("name" = String, Path, description = "Attribute name"),
    ),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn delete_principal_attribute(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, principal_kind, principal_id, name)): Path<(
        String,
        String,
        String,
        String,
    )>,
) -> ApiResult<StatusCode> {
    require_manage(&auth)?;
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let rows = sqlx::query(
            r#"
            DELETE FROM principal_attributes
            WHERE workspace_id = $1
              AND principal_kind = $2
              AND principal_id = $3
              AND name = $4
            "#,
        )
        .bind(ws)
        .bind(&principal_kind)
        .bind(&principal_id)
        .bind(&name)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("delete principal attr: {e}")))?
        .rows_affected();
        if rows == 0 {
            return Err(ApiError::NotFound("Principal attribute not found".into()));
        }
        let _ = bump_generation(&state, ws).await?;
        Ok(StatusCode::NO_CONTENT)
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, principal_kind, principal_id, name);
        require_pool(&state)?;
        unreachable!()
    }
}
