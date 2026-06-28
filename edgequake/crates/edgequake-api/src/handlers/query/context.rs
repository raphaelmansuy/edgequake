//! Query context HTTP handlers (SPEC-028).

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Extension, Json,
};
use edgequake_observability::PropagationHeaders;
use serde::Deserialize;

use crate::error::ApiResult;
use crate::handlers::context_types::{
    ContentGranularity, ContextRetrievalRequest, ContextRetrievalResponse, ContextSearchRequest,
    ContextSearchResponse,
};
use crate::middleware::TenantContext;
use crate::services::query_context::{
    fetch_context_by_id, resolve_keyword_llm_override, retrieve_context, search_context,
};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct FetchContextQuery {
    #[serde(default)]
    pub content_granularity: Option<ContentGranularity>,
}

#[utoipa::path(
    post,
    path = "/api/v1/query/context",
    tag = "Query Context",
    request_body = ContextRetrievalRequest,
    responses(
        (status = 200, description = "Structured context bundle", body = ContextRetrievalResponse),
        (status = 400, description = "Invalid query or mode")
    )
)]
pub async fn retrieve_query_context(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Extension(propagation): Extension<PropagationHeaders>,
    Json(request): Json<ContextRetrievalRequest>,
) -> ApiResult<(HeaderMap, Json<ContextRetrievalResponse>)> {
    let workspace = crate::handlers::query::resolve_query_workspace(
        &state,
        tenant_ctx.workspace_id.as_deref(),
    )
    .await?;
    let llm_override = resolve_keyword_llm_override(
        &state,
        workspace.as_ref(),
        &propagation,
        None,
        None,
    )?;

    let response = retrieve_context(&state, &tenant_ctx, request, llm_override).await?;

    let mut headers = HeaderMap::new();
    if let Ok(val) = response.retrieval_id.parse() {
        headers.insert("X-Retrieval-Id", val);
    }
    if let Ok(val) = response.retrieval_fingerprint.parse() {
        headers.insert("X-Retrieval-Fingerprint", val);
    }

    Ok((headers, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/v1/query/context/search",
    tag = "Query Context",
    request_body = ContextSearchRequest,
    responses(
        (status = 200, description = "Search summaries with retrieval handles", body = ContextSearchResponse)
    )
)]
pub async fn search_query_context(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Extension(propagation): Extension<PropagationHeaders>,
    Json(request): Json<ContextSearchRequest>,
) -> ApiResult<Json<ContextSearchResponse>> {
    let workspace = crate::handlers::query::resolve_query_workspace(
        &state,
        tenant_ctx.workspace_id.as_deref(),
    )
    .await?;
    let llm_override = resolve_keyword_llm_override(
        &state,
        workspace.as_ref(),
        &propagation,
        None,
        None,
    )?;

    Ok(Json(
        search_context(&state, &tenant_ctx, request, llm_override).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/query/context/{retrieval_id}",
    tag = "Query Context",
    params(
        ("retrieval_id" = String, Path, description = "Retrieval handle from search"),
        ("content_granularity" = Option<ContentGranularity>, Query, description = "Payload tier")
    ),
    responses(
        (status = 200, description = "Full cached context bundle", body = ContextRetrievalResponse),
        (status = 404, description = "Unknown retrieval_id"),
        (status = 410, description = "Retrieval expired")
    )
)]
pub async fn fetch_query_context(
    Path(retrieval_id): Path<String>,
    Query(query): Query<FetchContextQuery>,
) -> ApiResult<Json<ContextRetrievalResponse>> {
    let granularity = query.content_granularity.unwrap_or(ContentGranularity::Agent);
    Ok(Json(fetch_context_by_id(&retrieval_id, granularity)?))
}
