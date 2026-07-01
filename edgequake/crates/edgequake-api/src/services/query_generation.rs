//! Query orchestration — retrieval + generation path (SPEC-028 Phase 3).
//!
//! Wraps engine execution and legacy response assembly so handlers stay thin.

use std::sync::Arc;

use edgequake_llm::traits::LLMProvider;
use edgequake_query::QueryResponse;

use crate::error::ApiResult;
use crate::handlers::context_types::ContentGranularity;
use crate::handlers::query::resolve_query_workspace;
use crate::handlers::query_types::QueryResponse as LegacyQueryResponse;
use crate::middleware::TenantContext;
use crate::services::query_context::{build_legacy_query_response, build_legacy_query_sources};
use crate::services::query_execution::{
    execute_sota_query_with_auth_fallback, resolve_workspace_query_resources,
};
use crate::services::query_request_builder::{build_engine_request, QueryExecutionParams};
use crate::state::AppState;

/// Execute full RAG query (retrieval + optional LLM generation) via engine SSOT.
pub async fn execute_full_query(
    state: &AppState,
    tenant_ctx: &TenantContext,
    params: QueryExecutionParams,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> ApiResult<QueryResponse> {
    // Fail closed on explicit workspace when invalid (same as /query).
    let _workspace = resolve_query_workspace(state, tenant_ctx.workspace_id.as_deref()).await?;

    let engine_request = build_engine_request(&params);
    let resources =
        resolve_workspace_query_resources(state, tenant_ctx.workspace_id.as_deref()).await?;

    execute_sota_query_with_auth_fallback(state, engine_request, resources, llm_override).await
}

/// Execute query and assemble legacy HTTP `QueryResponse` (sources + stats).
#[allow(clippy::too_many_arguments)]
pub async fn execute_legacy_query_response(
    state: &AppState,
    tenant_ctx: &TenantContext,
    params: QueryExecutionParams,
    llm_override: Option<Arc<dyn LLMProvider>>,
    include_references: bool,
    include_subgraph: bool,
    conversation_id: Option<String>,
    llm_provider: Option<String>,
    llm_model: Option<String>,
) -> ApiResult<LegacyQueryResponse> {
    let reranker_configured = state.query.engine_impl.has_reranker();
    let reranked = params.enable_rerank && reranker_configured;
    let result = execute_full_query(state, tenant_ctx, params.clone(), llm_override).await?;
    let sources = build_legacy_query_sources(
        state,
        &result,
        include_references,
        params.enable_rerank,
        params.rerank_top_k,
        ContentGranularity::Citation,
    )
    .await;

    Ok(build_legacy_query_response(
        result,
        sources,
        conversation_id,
        reranked,
        llm_provider,
        llm_model,
        include_subgraph,
        params.rerank_top_k,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_query::QueryMode;

    #[test]
    fn query_execution_params_default_generation_flags() {
        let params = QueryExecutionParams {
            query: "test".into(),
            mode: QueryMode::Mix,
            max_results: None,
            context_only: false,
            prompt_only: false,
            enable_rerank: true,
            rerank_top_k: None,
            mix_weights: None,
            conversation_history: None,
            system_prompt: None,
            allowed_document_ids: None,
            data_tenant_id: None,
            workspace_id: None,
            llm_provider: None,
            llm_model: None,
        };
        assert!(!params.context_only);
    }
}
