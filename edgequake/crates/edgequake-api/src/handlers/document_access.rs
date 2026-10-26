//! SPEC-180 adapters; all authorization and persistence lives in access services.
#[cfg(feature = "postgres")]
use crate::services::document_access as service;
#[cfg(feature = "postgres")]
use service::{
    AccessMutationRequest, AccessPreviewRequest, AccessPreviewResponse, AssignmentOptions,
    DocumentAccessView,
};
#[cfg(not(feature = "postgres"))]
type AssignmentOptions = Value;
#[cfg(not(feature = "postgres"))]
type DocumentAccessView = Value;
#[cfg(not(feature = "postgres"))]
type AccessPreviewRequest = Value;
#[cfg(not(feature = "postgres"))]
type AccessPreviewResponse = Value;
#[cfg(not(feature = "postgres"))]
type AccessMutationRequest = Value;
use crate::{
    error::{ApiError, ApiResult},
    handlers::auth::ApiAuthenticated,
    middleware::TenantContext,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Default, Deserialize)]
pub struct AssignmentQuery {
    pub q: Option<String>,
    pub cursor: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/workspaces/{workspace_id}/clearance/members",tag="Clearance",params(("workspace_id"=Uuid,Path,description="Workspace")),responses((status=200,body=Value,description="Bounded steward-only member lookup")))]
pub async fn search_access_members(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    mut ctx: TenantContext,
    Query(query): Query<AssignmentQuery>,
) -> ApiResult<Json<Value>> {
    ctx.user_id = Some(auth.user_id);
    #[cfg(feature = "postgres")]
    {
        let actor = service::AccessActor::from_context(&ctx)?;
        if workspace_id != actor.workspace {
            return Err(ApiError::forbidden_reason(
                "Workspace does not match authorized scope",
            ));
        }
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        Ok(Json(
            json!({"members":service::search_members(pool,&actor,query.q.as_deref().unwrap_or("")).await?}),
        ))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, ctx, workspace_id, query);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(get,path = "/api/v1/workspaces/{workspace_id}/clearance/assignment-options",tag="Clearance",params(("workspace_id"=Uuid,Path,description="Workspace")),responses((status=200,body=AssignmentOptions,description="Assignable access options")))]
pub async fn get_access_assignment_options(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    mut ctx: TenantContext,
    Query(query): Query<AssignmentQuery>,
) -> ApiResult<Json<Value>> {
    ctx.user_id = Some(auth.user_id);
    #[cfg(feature = "postgres")]
    {
        let actor = service::AccessActor::from_context(&ctx)?;
        if actor.workspace != workspace_id {
            return Err(ApiError::forbidden_reason(
                "Workspace does not match authorized scope",
            ));
        }
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        return Ok(Json(
            serde_json::to_value(
                service::assignment_options(
                    pool,
                    &actor,
                    query.q.as_deref().unwrap_or(""),
                    query.cursor.as_deref(),
                )
                .await?,
            )
            .map_err(|e| ApiError::Internal(e.to_string()))?,
        ));
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, ctx, query, workspace_id);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(get,path = "/api/v1/documents/{document_id}/access",tag="Clearance",params(("document_id"=Uuid,Path,description="Document")),responses((status=200,body=DocumentAccessView,description="Saved access and capabilities"),(status=404,description="Document not found")))]
pub async fn get_document_access(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(document_id): Path<Uuid>,
    mut ctx: TenantContext,
) -> ApiResult<(HeaderMap, Json<Value>)> {
    ctx.user_id = Some(auth.user_id);
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let view = service::read_access(
            pool,
            &service::AccessActor::from_context(&ctx)?,
            document_id,
        )
        .await?;
        let mut headers = HeaderMap::new();
        headers.insert("cache-control", "private, no-store".parse().unwrap());
        headers.insert(
            "etag",
            format!("\"label-{}\"", view.label_version)
                .parse()
                .map_err(|_| ApiError::Internal("Invalid label version".into()))?,
        );
        Ok((
            headers,
            Json(serde_json::to_value(view).map_err(|e| ApiError::Internal(e.to_string()))?),
        ))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, ctx, document_id);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(post,path = "/api/v1/workspaces/{workspace_id}/clearance/preview",tag="Clearance",params(("workspace_id"=Uuid,Path,description="Workspace")),request_body=AccessPreviewRequest,responses((status=200,body=AccessPreviewResponse,description="Exact access preview")))]
pub async fn preview_document_access(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path(workspace_id): Path<Uuid>,
    mut ctx: TenantContext,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    ctx.user_id = Some(auth.user_id);
    #[cfg(feature = "postgres")]
    {
        let actor = service::AccessActor::from_context(&ctx)?;
        if actor.workspace != workspace_id {
            return Err(ApiError::forbidden_reason(
                "Workspace does not match authorized scope",
            ));
        }
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let request = serde_json::from_value(body).map_err(|_| {
            ApiError::bad_request_coded("INVALID_ACCESS_PAYLOAD", "Invalid access preview")
        })?;
        return Ok(Json(service::preview(pool, &actor, request).await?));
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, ctx, body, workspace_id);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(post,path = "/api/v1/documents/labels/batch",tag="Clearance",request_body=AccessMutationRequest,responses((status=200,body=Value,description="Access changes committed atomically")))]
pub async fn batch_document_access(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    mut ctx: TenantContext,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    ctx.user_id = Some(auth.user_id);
    #[cfg(feature = "postgres")]
    {
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let actor = service::AccessActor::from_context(&ctx)?;
        let outcome = if body.get("operation").is_some() {
            let intent = serde_json::from_value(body).map_err(|_| {
                ApiError::bad_request_coded("INVALID_ACCESS_PAYLOAD", "Invalid bulk access change")
            })?;
            service::mutate_bulk(pool, &actor, intent).await?
        } else {
            let request = serde_json::from_value(body).map_err(|_| {
                ApiError::bad_request_coded("INVALID_ACCESS_PAYLOAD", "Invalid bulk access change")
            })?;
            service::mutate(pool, &actor, request).await?
        };
        Ok(Json(outcome))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, ctx, body);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}

#[utoipa::path(get,path = "/api/v1/workspaces/{workspace_id}/clearance/operations/{operation_id}",tag="Clearance",params(("workspace_id"=Uuid,Path,description="Workspace"),("operation_id"=Uuid,Path,description="Operation")),responses((status=200,body=Value,description="Operation outcome")))]
pub async fn get_access_operation(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Path((workspace_id, operation_id)): Path<(Uuid, Uuid)>,
    mut ctx: TenantContext,
) -> ApiResult<Json<Value>> {
    ctx.user_id = Some(auth.user_id);
    #[cfg(feature = "postgres")]
    {
        let actor = service::AccessActor::from_context(&ctx)?;
        if actor.workspace != workspace_id {
            return Err(ApiError::forbidden_reason(
                "Workspace does not match authorized scope",
            ));
        }
        let pool = state
            .optional_pg_pool()
            .ok_or_else(|| ApiError::Internal("PostgreSQL is required".into()))?;
        let (mut tx, _) = service::scoped_transaction(pool, &actor, false).await?;
        let available: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
            .bind(service::operation_lock(&actor, operation_id))
            .fetch_one(&mut *tx)
            .await
            .map_err(service::database)?;
        let found:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public.document_access_operations WHERE operation_id=$1 AND workspace_id=$2 AND actor_id=$3)")
            .bind(operation_id).bind(workspace_id).bind(&actor.actor).fetch_one(&mut *tx).await.map_err(service::database)?;
        tx.commit().await.map_err(service::database)?;
        Ok(Json(
            json!({"status":if found{"committed"}else if !available{"pending"}else{"unknown"},"operation_id":operation_id}),
        ))
    }
    #[cfg(not(feature = "postgres"))]
    {
        let _ = (state, ctx, workspace_id, operation_id);
        Err(ApiError::Internal("PostgreSQL is required".into()))
    }
}
