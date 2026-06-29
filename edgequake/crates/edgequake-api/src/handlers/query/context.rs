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
    ContentGranularity, ContextArtifactResponse, ContextRetrievalRequest, ContextRetrievalResponse,
    ContextSearchRequest, ContextSearchResponse,
};
use crate::middleware::TenantContext;
use crate::services::artifact_retrieval::{retrieve_artifact, ArtifactRetrievalOptions};
use crate::services::query_context::{
    fetch_context_by_id, resolve_keyword_llm_override, retrieve_context, search_context,
    FetchContextOptions,
};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct FetchContextQuery {
    #[serde(default)]
    pub content_granularity: Option<ContentGranularity>,
    /// Include query-matched graph in `bundle.subgraph` (default true).
    #[serde(default = "default_include_subgraph_fetch")]
    pub include_subgraph: bool,
}

fn default_include_subgraph_fetch() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct ArtifactQuery {
    /// Parent document ID — required when `artifact_type=figure`.
    #[serde(default)]
    pub document_id: Option<String>,
    /// Include full document body (document artifacts only).
    #[serde(default)]
    pub include_content: Option<bool>,
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
    let workspace =
        crate::handlers::query::resolve_query_workspace(&state, tenant_ctx.workspace_id.as_deref())
            .await?;
    let llm_override =
        resolve_keyword_llm_override(&state, workspace.as_ref(), &propagation, None, None)?;

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
    let workspace =
        crate::handlers::query::resolve_query_workspace(&state, tenant_ctx.workspace_id.as_deref())
            .await?;
    let llm_override =
        resolve_keyword_llm_override(&state, workspace.as_ref(), &propagation, None, None)?;

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
    let granularity = query
        .content_granularity
        .unwrap_or(ContentGranularity::Agent);
    Ok(Json(fetch_context_by_id(
        &retrieval_id,
        FetchContextOptions {
            granularity,
            include_subgraph: query.include_subgraph,
        },
    )?))
}

#[utoipa::path(
    get,
    path = "/api/v1/query/context/artifacts/{artifact_type}/{artifact_id}",
    tag = "Query Context",
    params(
        ("artifact_type" = String, Path, description = "document | chunk | figure | markdown | pdf"),
        ("artifact_id" = String, Path, description = "Stable artifact ID from bundle lineage"),
        ("document_id" = Option<String>, Query, description = "Required for figure; resolves pdf by document"),
        ("include_content" = Option<bool>, Query, description = "Include markdown (document) or markdown_content (pdf)")
    ),
    responses(
        (status = 200, description = "Artifact payload", body = ContextArtifactResponse),
        (status = 400, description = "Invalid artifact type or missing document_id"),
        (status = 404, description = "Artifact not found")
    )
)]
pub async fn get_context_artifact(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Path((artifact_type, artifact_id)): Path<(String, String)>,
    Query(query): Query<ArtifactQuery>,
) -> ApiResult<Json<ContextArtifactResponse>> {
    Ok(Json(
        retrieve_artifact(
            &state,
            &tenant_ctx,
            &artifact_type,
            &artifact_id,
            ArtifactRetrievalOptions {
                document_id: query.document_id,
                include_content: query.include_content.unwrap_or(false),
            },
        )
        .await?,
    ))
}
