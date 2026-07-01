//! Execute RAG query handler.
//!
//! @implements UC0201 (Execute Query)
//! @implements FEAT0007 (Multi-Mode Query Execution)
//! @implements SPEC-028: Uses QueryContextService DRY helpers for source mapping

use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue},
    Extension, Json,
};
use edgequake_audit::{AuditEvent, AuditEventType, AuditResult};
use edgequake_observability::{
    record_llm_request, scope_llm_provider, PropagationHeaders, QueryOutcomeGuard, RequestContext,
};
use tracing::debug;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::OptionalAuth;
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::services::{
    build_engine_request, build_legacy_query_response, build_legacy_query_sources,
    ensure_debug_granularity_allowed, execute_sota_query_with_auth_fallback, record_audit,
    resolve_workspace_query_resources, validate_llm_override_pair, with_request_context,
    QueryExecutionParams,
};
use crate::state::AppState;
use crate::validation::validate_query;
use edgequake_query::QueryMode;

use super::workspace_resolve::{get_workspace_llm_info, resolve_query_workspace};
pub use crate::handlers::query_types::{QueryRequest, QueryResponse};

#[utoipa::path(
    post,
    path = "/api/v1/query",
    tag = "Query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = QueryResponse),
        (status = 400, description = "Invalid query")
    )
)]
#[tracing::instrument(
    name = "query_execute",
    skip(state, tenant_ctx, propagation, request),
    fields(
        request_id = %req_ctx.request_id,
        query.mode = tracing::field::Empty,
        otel.name = "query_execute",
    )
)]
pub async fn execute_query(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    OptionalAuth(auth_user): OptionalAuth,
    Extension(req_ctx): Extension<RequestContext>,
    Extension(propagation): Extension<PropagationHeaders>,
    Json(request): Json<QueryRequest>,
) -> ApiResult<(HeaderMap, Json<QueryResponse>)> {
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Mix);
    let query_mode_label = mode.to_string();
    tracing::Span::current().record("query.mode", query_mode_label.as_str());
    let query_obs =
        QueryOutcomeGuard::with_request_id(&query_mode_label, Some(req_ctx.request_id.clone()));

    debug!(
        request_id = %req_ctx.request_id,
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        query.mode = %query_mode_label,
        query = %request.query,
        "Executing query with tenant context"
    );

    validate_query(&request.query, state.config.max_query_length)?;
    ensure_debug_granularity_allowed(
        request.content_granularity,
        auth_user.as_ref().map(|u| u.role.clone()),
    )?;

    let workspace = resolve_query_workspace(&state, tenant_ctx.workspace_id.as_deref()).await?;

    let data_tenant_id = workspace
        .as_ref()
        .map(|ws| ws.tenant_id.to_string())
        .or_else(|| tenant_ctx.tenant_id.clone());

    let mut allowed_document_ids = None;
    if let Some(ref filter) = request.document_filter {
        if let Some(allowed_ids) = super::document_filter_resolver::resolve_document_filter(
            state.storage.kv_storage.as_ref(),
            filter,
            &data_tenant_id,
            &tenant_ctx.workspace_id,
        )
        .await?
        {
            debug!(
                matched_doc_count = allowed_ids.len(),
                "Document filter resolved — restricting query scope"
            );
            allowed_document_ids = Some(allowed_ids);
        }
    }

    validate_llm_override_pair(
        request.llm_provider.as_deref(),
        request.llm_model.as_deref(),
    )?;

    let resolver = WorkspaceProviderResolver::from_app_state(&state);
    let extra_headers = propagation.merge_with(request.extra_headers.clone());
    let llm_request = LlmResolutionRequest {
        provider: request.llm_provider.clone(),
        model: request.llm_model.clone(),
        extra_headers,
    };
    let llm_override =
        match resolver.resolve_llm_provider_with_workspace(workspace.as_ref(), &llm_request) {
            Ok(Some(resolved)) => Some(resolved.provider),
            Ok(None) => None,
            Err(e) => return Err(ApiError::from(e)),
        };

    let params = QueryExecutionParams {
        query: request.query.clone(),
        mode,
        max_results: request.max_results,
        context_only: request.context_only,
        prompt_only: request.prompt_only,
        enable_rerank: request.enable_rerank,
        rerank_top_k: request.rerank_top_k,
        mix_weights: request.mix_weights.clone(),
        conversation_history: request.conversation_history.clone(),
        system_prompt: request.system_prompt.clone(),
        allowed_document_ids,
        data_tenant_id,
        workspace_id: tenant_ctx.workspace_id.clone(),
        llm_provider: request.llm_provider.clone(),
        llm_model: request.llm_model.clone(),
    };

    let engine_request = build_engine_request(&params);
    let resources =
        resolve_workspace_query_resources(&state, tenant_ctx.workspace_id.as_deref()).await?;

    let obs_llm_provider = request
        .llm_provider
        .clone()
        .or_else(|| workspace.as_ref().map(|w| w.llm_provider.clone()))
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| "default".to_string());

    let result = scope_llm_provider(
        obs_llm_provider,
        execute_sota_query_with_auth_fallback(&state, engine_request, resources, llm_override),
    )
    .await?;

    let reranker_configured = state.query.engine_impl.has_reranker();
    let reranked = request.enable_rerank && reranker_configured;

    let sources = build_legacy_query_sources(
        &state,
        &result,
        request.include_references,
        request.enable_rerank,
        request.rerank_top_k,
        request.content_granularity,
    )
    .await;

    let conversation_id = if request.conversation_history.is_some() {
        Some(uuid::Uuid::new_v4().to_string())
    } else {
        None
    };

    let (llm_provider, llm_model) =
        get_workspace_llm_info(&state, tenant_ctx.workspace_id.as_deref()).await;

    let tenant_for_audit = params
        .data_tenant_id
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let mut audit_event = AuditEvent::new(
        tenant_for_audit,
        AuditEventType::DocumentQuery,
        "execute_query".to_string(),
        AuditResult::Success,
    );
    if let Some(ref ws) = tenant_ctx.workspace_id {
        audit_event = audit_event.with_workspace(ws.clone());
    }
    record_audit(&state, with_request_context(audit_event, &req_ctx));

    query_obs.mark_success(result.stats.total_time_ms as f64 / 1000.0);

    if result.stats.generation_time_ms > 0 {
        record_llm_request(
            llm_provider.as_deref().unwrap_or("unknown"),
            "query_generation",
            "success",
            result.stats.generation_time_ms as f64 / 1000.0,
        );
    }

    let response = build_legacy_query_response(
        result,
        sources,
        conversation_id,
        reranked,
        llm_provider,
        llm_model,
        request.include_subgraph,
        request.rerank_top_k,
    );

    let mut headers = HeaderMap::new();
    if request.context_only {
        if let Ok(link) =
            HeaderValue::from_str("</api/v1/query/context>; rel=\"successor-version\"")
        {
            headers.insert("Deprecation", HeaderValue::from_static("true"));
            headers.insert("Link", link);
        }
    }

    Ok((headers, Json(response)))
}
