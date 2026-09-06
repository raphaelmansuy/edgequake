//! Document ACL grant/revoke — SPEC-146 M1b.

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
    DocumentAclEntryDto, GrantDocumentAclRequest, GrantDocumentAclResponse,
    ListDocumentAclResponse, RevokeDocumentAclRequest, RevokeDocumentAclResponse,
};

#[utoipa::path(
    get,
    path = "/api/v1/documents/{document_id}/acl",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("document_id" = String, Path, description = "Document UUID")),
    responses((status = 200, description = "ACL entries", body = ListDocumentAclResponse))
)]
pub async fn list_document_acl(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(document_id): Path<String>,
) -> ApiResult<Json<ListDocumentAclResponse>> {
    require_read(&auth)?;
    let doc_id = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::BadRequest("Invalid document_id".into()))?;

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
                Option<String>,
                Option<String>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            SELECT document_id, principal_kind, principal_id, permission,
                   granted_by_kind, granted_by_id, created_at
            FROM document_acl
            WHERE document_id = $1
            ORDER BY principal_kind, principal_id, permission
            "#,
        )
        .bind(doc_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("list document acl: {e}")))?;

        let entries = rows
            .into_iter()
            .map(
                |(
                    document_id,
                    principal_kind,
                    principal_id,
                    permission,
                    granted_by_kind,
                    granted_by_id,
                    created_at,
                )| DocumentAclEntryDto {
                    document_id,
                    principal_kind,
                    principal_id,
                    permission,
                    granted_by_kind,
                    granted_by_id,
                    created_at: created_at.to_rfc3339(),
                },
            )
            .collect();
        return Ok(Json(ListDocumentAclResponse { entries }));
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (doc_id,);
        require_pool(&state)?;
        unreachable!()
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/documents/{document_id}/acl",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("document_id" = String, Path, description = "Document UUID")),
    request_body = GrantDocumentAclRequest,
    responses((status = 201, description = "ACL granted", body = GrantDocumentAclResponse))
)]
pub async fn grant_document_acl(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(document_id): Path<String>,
    Json(req): Json<GrantDocumentAclRequest>,
) -> ApiResult<(StatusCode, Json<GrantDocumentAclResponse>)> {
    require_manage(&auth)?;
    if req.principal_kind.trim().is_empty()
        || req.principal_id.trim().is_empty()
        || req.permission.trim().is_empty()
    {
        return Err(ApiError::BadRequest(
            "principal_kind, principal_id, and permission are required".into(),
        ));
    }
    let doc_id = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::BadRequest("Invalid document_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let ws: Option<Uuid> =
            sqlx::query_scalar("SELECT workspace_id FROM documents WHERE id = $1")
                .bind(doc_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::Internal(format!("lookup document: {e}")))?;
        let ws = ws.ok_or_else(|| ApiError::NotFound("Document not found".into()))?;

        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                Option<String>,
                Option<String>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            r#"
            INSERT INTO document_acl
              (document_id, principal_kind, principal_id, permission,
               granted_by_kind, granted_by_id, created_at)
            VALUES ($1, $2, $3, $4, 'user', $5, NOW())
            ON CONFLICT (document_id, principal_kind, principal_id, permission) DO UPDATE
              SET granted_by_kind = EXCLUDED.granted_by_kind,
                  granted_by_id = EXCLUDED.granted_by_id
            RETURNING document_id, principal_kind, principal_id, permission,
                      granted_by_kind, granted_by_id, created_at
            "#,
        )
        .bind(doc_id)
        .bind(req.principal_kind.trim())
        .bind(req.principal_id.trim())
        .bind(req.permission.trim())
        .bind(&auth.user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("grant acl: {e}")))?;

        let policy_generation = bump_generation(&state, ws).await?;
        let (
            document_id,
            principal_kind,
            principal_id,
            permission,
            granted_by_kind,
            granted_by_id,
            created_at,
        ) = row;
        Ok((
            StatusCode::CREATED,
            Json(GrantDocumentAclResponse {
                entry: DocumentAclEntryDto {
                    document_id,
                    principal_kind,
                    principal_id,
                    permission,
                    granted_by_kind,
                    granted_by_id,
                    created_at: created_at.to_rfc3339(),
                },
                policy_generation,
            }),
        ))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (doc_id, req, auth);
        require_pool(&state)?;
        unreachable!()
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/documents/{document_id}/acl",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("document_id" = String, Path, description = "Document UUID")),
    request_body = RevokeDocumentAclRequest,
    responses((status = 200, description = "ACL revoked", body = RevokeDocumentAclResponse))
)]
pub async fn revoke_document_acl(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(document_id): Path<String>,
    Json(req): Json<RevokeDocumentAclRequest>,
) -> ApiResult<Json<RevokeDocumentAclResponse>> {
    require_manage(&auth)?;
    let doc_id = Uuid::parse_str(&document_id)
        .map_err(|_| ApiError::BadRequest("Invalid document_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let ws: Option<Uuid> =
            sqlx::query_scalar("SELECT workspace_id FROM documents WHERE id = $1")
                .bind(doc_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| ApiError::Internal(format!("lookup document: {e}")))?;
        let ws = ws.ok_or_else(|| ApiError::NotFound("Document not found".into()))?;

        let rows = sqlx::query(
            r#"
            DELETE FROM document_acl
            WHERE document_id = $1
              AND principal_kind = $2
              AND principal_id = $3
              AND permission = $4
            "#,
        )
        .bind(doc_id)
        .bind(req.principal_kind.trim())
        .bind(req.principal_id.trim())
        .bind(req.permission.trim())
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("revoke acl: {e}")))?
        .rows_affected();

        let policy_generation = if rows > 0 {
            bump_generation(&state, ws).await?
        } else {
            0
        };
        Ok(Json(RevokeDocumentAclResponse {
            revoked: rows > 0,
            policy_generation,
        }))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (doc_id, req, auth);
        require_pool(&state)?;
        unreachable!()
    }
}
