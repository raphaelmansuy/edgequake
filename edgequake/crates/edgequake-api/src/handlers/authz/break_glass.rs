//! Break-glass sessions — SPEC-146 M5 (TTL default 15m, max 60m, audited).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use edgequake_audit::{AuditEvent, AuditEventType, AuditResult, AuditSeverity};
use serde_json::json;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::ApiAuthenticated;
use crate::services::audit::record_audit;
use crate::state::AppState;

use super::helpers::{require_break_glass, require_manage, require_pool, require_read};
use super::types::{
    resolve_break_glass_ttl, BreakGlassSessionDto, CreateBreakGlassRequest,
    CreateBreakGlassResponse, ListBreakGlassResponse, RevokeBreakGlassResponse,
};

fn audit_break_glass(
    state: &AppState,
    workspace_id: Uuid,
    user_id: &str,
    action: &str,
    session_id: Option<Uuid>,
    result: AuditResult,
    metadata: serde_json::Value,
) {
    let mut event = AuditEvent::new(
        workspace_id.to_string(),
        AuditEventType::Authorization,
        action.to_string(),
        result,
    )
    .with_workspace(workspace_id.to_string())
    .with_user(user_id.to_string());
    event.severity = AuditSeverity::High;
    event.metadata = metadata;
    if let Some(sid) = session_id {
        event = event.with_resource("break_glass_session".into(), sid.to_string());
    }
    record_audit(state, event);
}

#[utoipa::path(
    get,
    path = "/api/v1/workspaces/{workspace_id}/authz/break-glass",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    responses(
        (status = 200, description = "Break-glass sessions", body = ListBreakGlassResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — missing capability"),
        (status = 404, description = "Workspace not found (existence-hiding where applicable)"),
    )
)]
pub async fn list_break_glass_sessions(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
) -> ApiResult<Json<ListBreakGlassResponse>> {
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
                chrono::DateTime<chrono::Utc>,
                Option<Vec<Uuid>>,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
        >(
            r#"
            SELECT session_id, workspace_id, principal_kind, principal_id, reason,
                   expires_at, scope_doc_ids, created_at, revoked_at
            FROM break_glass_sessions
            WHERE workspace_id = $1
            ORDER BY created_at DESC
            LIMIT 100
            "#,
        )
        .bind(ws)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("list break-glass: {e}")))?;

        let sessions = rows
            .into_iter()
            .map(
                |(
                    session_id,
                    workspace_id,
                    principal_kind,
                    principal_id,
                    reason,
                    expires_at,
                    scope_doc_ids,
                    created_at,
                    revoked_at,
                )| BreakGlassSessionDto {
                    session_id,
                    workspace_id,
                    principal_kind,
                    principal_id,
                    reason,
                    expires_at: expires_at.to_rfc3339(),
                    scope_doc_ids,
                    created_at: created_at.to_rfc3339(),
                    revoked_at: revoked_at.map(|t| t.to_rfc3339()),
                },
            )
            .collect();
        return Ok(Json(ListBreakGlassResponse { sessions }));
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
    path = "/api/v1/workspaces/{workspace_id}/authz/break-glass",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(("workspace_id" = String, Path, description = "Workspace UUID")),
    request_body = CreateBreakGlassRequest,
    responses(
        (status = 201, description = "Session created", body = CreateBreakGlassResponse),
        (status = 400, description = "Invalid TTL or reason"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — BreakGlass capability required"),
        (status = 404, description = "Workspace not found (existence-hiding where applicable)"),
    )
)]
pub async fn create_break_glass_session(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<String>,
    Json(req): Json<CreateBreakGlassRequest>,
) -> ApiResult<(StatusCode, Json<CreateBreakGlassResponse>)> {
    require_break_glass(&auth)?;
    if req.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("reason is required".into()));
    }
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;

    let ttl = resolve_break_glass_ttl(req.ttl_minutes).map_err(ApiError::BadRequest)?;
    let expires_at = Utc::now() + Duration::minutes(ttl as i64);
    let principal_kind = req
        .principal_kind
        .as_deref()
        .unwrap_or("user")
        .trim()
        .to_string();
    let principal_id = req
        .principal_id
        .clone()
        .unwrap_or_else(|| auth.user_id.clone());

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let session_id = Uuid::new_v4();
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<Vec<Uuid>>,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
        >(
            r#"
            INSERT INTO break_glass_sessions
              (session_id, workspace_id, principal_kind, principal_id, reason,
               expires_at, scope_doc_ids, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING session_id, workspace_id, principal_kind, principal_id, reason,
                      expires_at, scope_doc_ids, created_at, revoked_at
            "#,
        )
        .bind(session_id)
        .bind(ws)
        .bind(&principal_kind)
        .bind(&principal_id)
        .bind(req.reason.trim())
        .bind(expires_at)
        .bind(req.scope_doc_ids.as_ref())
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("create break-glass: {e}")))?;

        let (
            session_id,
            workspace_id,
            principal_kind,
            principal_id,
            reason,
            expires_at,
            scope_doc_ids,
            created_at,
            revoked_at,
        ) = row;

        audit_break_glass(
            &state,
            workspace_id,
            &auth.user_id,
            "break_glass.create",
            Some(session_id),
            AuditResult::Success,
            json!({
                "ttl_minutes": ttl,
                "expires_at": expires_at.to_rfc3339(),
                "principal_kind": principal_kind,
                "principal_id": principal_id,
                "scope_doc_ids": scope_doc_ids,
            }),
        );

        Ok((
            StatusCode::CREATED,
            Json(CreateBreakGlassResponse {
                session: BreakGlassSessionDto {
                    session_id,
                    workspace_id,
                    principal_kind,
                    principal_id,
                    reason,
                    expires_at: expires_at.to_rfc3339(),
                    scope_doc_ids,
                    created_at: created_at.to_rfc3339(),
                    revoked_at: revoked_at.map(|t| t.to_rfc3339()),
                },
            }),
        ))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, req, principal_kind, principal_id, expires_at, ttl);
        require_pool(&state)?;
        unreachable!()
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/workspaces/{workspace_id}/authz/break-glass/{session_id}",
    tag = "Authz",
    security(("bearer_auth" = [])),
    params(
        ("workspace_id" = String, Path, description = "Workspace UUID"),
        ("session_id" = String, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Session revoked", body = RevokeBreakGlassResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — BreakGlass or PolicyManage required"),
        (status = 404, description = "Session not found (existence-hiding)"),
    )
)]
pub async fn revoke_break_glass_session(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, session_id)): Path<(String, String)>,
) -> ApiResult<Json<RevokeBreakGlassResponse>> {
    // Revoke requires manage or break-glass.
    if require_break_glass(&auth).is_err() {
        require_manage(&auth)?;
    }
    let ws = Uuid::parse_str(&workspace_id)
        .map_err(|_| ApiError::BadRequest("Invalid workspace_id".into()))?;
    let sid = Uuid::parse_str(&session_id)
        .map_err(|_| ApiError::BadRequest("Invalid session_id".into()))?;

    #[cfg(feature = "postgres")]
    {
        let pool = require_pool(&state)?;
        let rows = sqlx::query(
            r#"
            UPDATE break_glass_sessions
            SET revoked_at = NOW()
            WHERE workspace_id = $1 AND session_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(ws)
        .bind(sid)
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("revoke break-glass: {e}")))?
        .rows_affected();

        let revoked = rows > 0;
        audit_break_glass(
            &state,
            ws,
            &auth.user_id,
            "break_glass.revoke",
            Some(sid),
            if revoked {
                AuditResult::Success
            } else {
                AuditResult::Failure
            },
            json!({ "revoked": revoked }),
        );

        Ok(Json(RevokeBreakGlassResponse { revoked }))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (ws, sid);
        require_pool(&state)?;
        unreachable!()
    }
}
