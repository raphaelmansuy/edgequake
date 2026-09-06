//! Query context retrieval service (SPEC-028 SSOT).
//!
//! Retrieval-only path: prepare → retrieve → map → ContextBundle.
//! Never calls answer LLM.

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_llm::traits::LLMProvider;
use edgequake_observability::PropagationHeaders;
use edgequake_query::{QueryMode, QueryResponse};
use tracing::debug;

use crate::error::{ApiError, ApiResult};
use crate::handlers::context_types::{
    ContentGranularity, ContextRetrievalRequest, ContextRetrievalResponse, ContextSearchRequest,
    ContextSearchResponse, ContextSearchResult, ModeSelection, SubgraphBundle,
};
use crate::handlers::query::resolve_query_workspace;
use crate::handlers::query_types::{
    MixWeightRequest, QueryResponse as LegacyQueryResponse, SourceReference,
};
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::services::context_bundle_mapper::{
    build_agent_hints, build_retrieval_stats, build_search_graph_metadata, build_truncation_info,
    compute_retrieval_fingerprint, compute_retrieval_quality, map_engine_response_to_bundle,
    map_query_context_to_subgraph, DocumentMeta, MappingOptions,
};
use crate::services::query_execution::{
    execute_sota_query_with_auth_fallback, resolve_workspace_query_resources,
    validate_llm_override_pair,
};
use crate::services::query_request_builder::{build_engine_request, QueryExecutionParams};
use crate::services::retrieval_id_cache::{global_retrieval_cache, new_retrieval_id};
use crate::services::source_reference_builder::build_sources_from_context;
use crate::state::AppState;
use crate::validation::validate_query;

async fn resolve_document_meta_map(
    kv_storage: &dyn edgequake_storage::traits::KVStorage,
    document_ids: &[String],
) -> HashMap<String, DocumentMeta> {
    if document_ids.is_empty() {
        return HashMap::new();
    }

    let mut result = HashMap::new();
    for doc_id in document_ids {
        let metadata_key =
            crate::services::document_metadata_scan::metadata_key_for_document(doc_id);
        if let Ok(Some(metadata)) = kv_storage.get_by_id(&metadata_key).await {
            let title = metadata
                .get("title")
                .or_else(|| metadata.get("file_name"))
                .and_then(|v| v.as_str())
                .unwrap_or(doc_id)
                .to_string();
            let mime_type = metadata
                .get("mime_type")
                .and_then(|v| v.as_str())
                .map(String::from);
            let created_at = metadata
                .get("created_at")
                .and_then(|v| v.as_str())
                .map(String::from);
            result.insert(
                doc_id.clone(),
                DocumentMeta {
                    title,
                    mime_type,
                    created_at,
                },
            );
        }
    }
    result
}

fn collect_document_ids(result: &QueryResponse) -> Vec<String> {
    let mut ids: Vec<String> = result
        .context
        .chunks
        .iter()
        .filter_map(|c| c.document_id.clone())
        .collect();
    for entity in &result.context.entities {
        if let Some(ref id) = entity.source_document_id {
            ids.push(id.clone());
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

struct PreparedContextRun {
    result: QueryResponse,
    reranked: bool,
    requested_mode: String,
    workspace_id: Option<String>,
    filter_json: Option<String>,
}

#[allow(clippy::too_many_arguments)]
async fn run_context_retrieval(
    state: &AppState,
    tenant_ctx: &TenantContext,
    query: &str,
    mode: QueryMode,
    max_results: Option<usize>,
    mix_weights: Option<MixWeightRequest>,
    enable_rerank: bool,
    rerank_top_k: Option<usize>,
    conversation_history: Option<Vec<crate::handlers::query_types::ConversationMessage>>,
    document_filter: Option<crate::handlers::query_types::DocumentFilter>,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> ApiResult<PreparedContextRun> {
    validate_query(query, state.config.max_query_length)?;

    let requested_mode = mode.to_string();
    let workspace = resolve_query_workspace(state, tenant_ctx.workspace_id.as_deref()).await?;

    let data_tenant_id = workspace
        .as_ref()
        .map(|ws| ws.tenant_id.to_string())
        .or_else(|| tenant_ctx.tenant_id.clone());

    let filter_json = document_filter
        .as_ref()
        .and_then(|f| serde_json::to_string(f).ok());

    let mut client_filter_ids = None;
    if let Some(ref filter) = document_filter {
        if let Some(allowed_ids) =
            crate::handlers::query::document_filter_resolver::resolve_document_filter(
                state.storage.kv_storage.as_ref(),
                filter,
                &data_tenant_id,
                &tenant_ctx.workspace_id,
            )
            .await?
        {
            client_filter_ids = Some(allowed_ids);
        }
    }

    let (allowed_document_ids, authz_ctx, allow_set) =
        crate::services::spec146_authz::resolve_query_allowed_document_ids(
            state,
            tenant_ctx,
            tenant_ctx.user_id.as_deref(),
            client_filter_ids,
        )
        .await?;
    let (authz_principal, policy_generation, allow_fingerprint) = match (&authz_ctx, &allow_set) {
        (Some(ctx), Some(allow)) => (
            Some(format!("{}:{}", ctx.principal.kind_str(), ctx.principal.id_str())),
            Some(ctx.policy_generation),
            Some(allow.fingerprint()),
        ),
        _ => (None, None, None),
    };

    let params = QueryExecutionParams {
        query: query.to_string(),
        mode,
        max_results,
        context_only: true,
        prompt_only: false,
        enable_rerank,
        rerank_top_k,
        mix_weights,
        conversation_history,
        system_prompt: None,
        question_type: None,
        hl_keywords: None,
        ll_keywords: None,
        response_type: None,
        allowed_document_ids,
        data_tenant_id,
        workspace_id: tenant_ctx.workspace_id.clone(),
        llm_provider: None,
        llm_model: None,
        reasoning_effort: None,
        authz_principal,
        policy_generation,
        allow_fingerprint,
    };

    let engine_request = build_engine_request(&params);
    let resources =
        resolve_workspace_query_resources(state, tenant_ctx.workspace_id.as_deref()).await?;

    let result =
        execute_sota_query_with_auth_fallback(state, engine_request, resources, llm_override)
            .await?;

    let reranker_configured = state.query.engine_impl.has_reranker();
    let reranked = enable_rerank && reranker_configured;

    Ok(PreparedContextRun {
        result,
        reranked,
        requested_mode,
        workspace_id: tenant_ctx.workspace_id.clone(),
        filter_json,
    })
}

fn build_context_response(
    request: &ContextRetrievalRequest,
    run: PreparedContextRun,
    document_meta: HashMap<String, DocumentMeta>,
    retrieval_id: String,
    cached: bool,
) -> ContextRetrievalResponse {
    let mapping = MappingOptions {
        granularity: request.content_granularity,
        include_lineage: request.include_lineage,
        include_documents: request.include_documents,
        include_agent_hints: request.include_agent_hints,
        include_subgraph: request.include_subgraph,
        rerank_top_k: request.rerank_top_k,
        reranked: run.reranked,
    };

    let bundle = map_engine_response_to_bundle(&run.result, &mapping, &document_meta);
    let retrieval_quality = compute_retrieval_quality(&run.result.context);
    let truncation = build_truncation_info(&run.result.context);
    let agent_hints = if request.include_agent_hints {
        Some(build_agent_hints(&run.result.context, &bundle))
    } else {
        None
    };

    let fingerprint = compute_retrieval_fingerprint(
        &request.query,
        &run.result.mode.to_string(),
        run.workspace_id.as_deref(),
        run.filter_json.as_deref(),
    );

    ContextRetrievalResponse {
        retrieval_id,
        query: request.query.clone(),
        mode: run.result.mode.to_string(),
        mode_selection: ModeSelection {
            requested: run.requested_mode,
            effective: run.result.mode.to_string(),
            adaptive: false,
            intent: None,
        },
        bundle,
        stats: build_retrieval_stats(&run.result, run.reranked),
        retrieval_quality,
        truncation,
        agent_hints,
        retrieval_fingerprint: fingerprint,
        cached,
    }
}

pub async fn retrieve_context(
    state: &AppState,
    tenant_ctx: &TenantContext,
    request: ContextRetrievalRequest,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> ApiResult<ContextRetrievalResponse> {
    let mode = QueryExecutionParams::parse_mode(request.mode.as_ref(), QueryMode::Mix)
        .map_err(|_| ApiError::BadRequest("Invalid query mode; bypass is not allowed".into()))?;
    QueryExecutionParams::reject_bypass(mode)
        .map_err(|_| ApiError::BadRequest("Invalid query mode; bypass is not allowed".into()))?;

    let run = run_context_retrieval(
        state,
        tenant_ctx,
        &request.query,
        mode,
        request.max_results,
        request.mix_weights.clone(),
        request.enable_rerank,
        request.rerank_top_k,
        request.conversation_history.clone(),
        request.document_filter.clone(),
        llm_override,
    )
    .await?;

    let doc_ids = collect_document_ids(&run.result);
    let document_meta =
        resolve_document_meta_map(state.storage.kv_storage.as_ref(), &doc_ids).await;

    let retrieval_id = new_retrieval_id();
    let response = build_context_response(&request, run, document_meta, retrieval_id, false);

    // SPEC-146: bind ret_* to principal when ABAC on.
    let (principal, policy_gen) = if state.security.doc_abac {
        let uid = tenant_ctx.user_id.as_deref();
        let principal = uid.map(|u| {
            let p = edgequake_authz::PrincipalId::from_auth_user_id(u);
            format!("{}:{}", p.kind_str(), p.id_str())
        });
        let policy_gen = if let Ok(Some(ws)) =
            crate::services::spec146_authz::parse_workspace_uuid(tenant_ctx)
        {
            crate::services::spec146_authz::load_policy_generation(
                state.allow_set_provider.as_ref(),
                ws,
            )
            .await
            .ok()
        } else {
            None
        };
        (principal, policy_gen)
    } else {
        (None, None)
    };
    global_retrieval_cache().store_for(response.clone(), principal, policy_gen);
    Ok(response)
}

pub async fn search_context(
    state: &AppState,
    tenant_ctx: &TenantContext,
    request: ContextSearchRequest,
    llm_override: Option<Arc<dyn LLMProvider>>,
) -> ApiResult<ContextSearchResponse> {
    let mode = QueryExecutionParams::parse_mode(request.mode.as_ref(), QueryMode::Mix)
        .map_err(|_| ApiError::BadRequest("Invalid query mode".into()))?;
    QueryExecutionParams::reject_bypass(mode)
        .map_err(|_| ApiError::BadRequest("Invalid query mode".into()))?;

    let full_request = ContextRetrievalRequest {
        query: request.query.clone(),
        mode: request.mode.clone(),
        content_granularity: ContentGranularity::Agent,
        max_results: request.max_results,
        conversation_history: None,
        document_filter: request.document_filter.clone(),
        mix_weights: None,
        enable_rerank: true,
        rerank_model: None,
        rerank_top_k: None,
        include_lineage: true,
        include_documents: true,
        include_agent_hints: false,
        include_subgraph: true,
    };

    let response = retrieve_context(state, tenant_ctx, full_request, llm_override).await?;

    let workspace = tenant_ctx.workspace_id.as_deref().unwrap_or("default");
    let title = response
        .bundle
        .chunks
        .first()
        .map(|c| c.content.chars().take(80).collect::<String>())
        .or_else(|| {
            response
                .bundle
                .subgraph
                .entities
                .first()
                .map(|e| e.name.clone())
        })
        .unwrap_or_else(|| request.query.clone());

    let snippet = response
        .bundle
        .chunks
        .first()
        .map(|c| c.content.chars().take(200).collect())
        .unwrap_or_default();

    Ok(ContextSearchResponse {
        results: vec![ContextSearchResult {
            retrieval_id: response.retrieval_id.clone(),
            title,
            snippet,
            url: format!(
                "edgequake://workspace/{}/retrieval/{}",
                workspace, response.retrieval_id
            ),
            score: response.retrieval_quality.coverage_score,
            metadata: Some(build_search_graph_metadata(
                &response.bundle,
                &response.mode,
            )),
        }],
    })
}

/// Options when fetching a cached retrieval (REST + MCP SSOT).
#[derive(Debug, Clone, Default)]
pub struct FetchContextOptions {
    pub granularity: ContentGranularity,
    pub include_subgraph: bool,
    /// SPEC-146: principal binding for `ret_*` handles.
    pub principal: Option<String>,
    pub policy_generation: Option<u64>,
}

impl FetchContextOptions {
    pub fn basic(granularity: ContentGranularity, include_subgraph: bool) -> Self {
        Self {
            granularity,
            include_subgraph,
            principal: None,
            policy_generation: None,
        }
    }
}

pub fn fetch_context_by_id(
    retrieval_id: &str,
    options: FetchContextOptions,
) -> ApiResult<ContextRetrievalResponse> {
    if !retrieval_id.starts_with("ret_") {
        return Err(ApiError::BadRequest("Invalid retrieval_id".into()));
    }

    if global_retrieval_cache().is_expired(retrieval_id) {
        return Err(ApiError::Gone("Retrieval expired — re-run search".into()));
    }

    let mut response = match global_retrieval_cache().get_for(
        retrieval_id,
        options.principal.as_deref(),
        options.policy_generation,
    ) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err(ApiError::NotFound(format!(
                "Retrieval not found: {}",
                retrieval_id
            )));
        }
        // SPEC-146: principal / policy_generation mismatch → existence-hiding 404.
        Err(()) => {
            return Err(ApiError::NotFound(format!(
                "Retrieval not found: {}",
                retrieval_id
            )));
        }
    };

    debug!(retrieval_id, "fetch returning cached bundle");
    response.cached = true;
    if !options.include_subgraph {
        response.bundle.subgraph = SubgraphBundle::default();
    }
    Ok(response)
}

pub async fn build_legacy_query_sources(
    state: &AppState,
    result: &QueryResponse,
    include_references: bool,
    enable_rerank: bool,
    rerank_top_k: Option<usize>,
    granularity: ContentGranularity,
) -> Vec<SourceReference> {
    let reranker_configured = state.query.engine_impl.has_reranker();
    let reranked = enable_rerank && reranker_configured;
    // SPEC-142: always stamp reference_id so verified rewrite / UI chips work.
    let _ = include_references;
    let mut sources =
        build_sources_from_context(&result.context, true, rerank_top_k, reranked, granularity);
    crate::handlers::query::resolve_chunk_file_paths(
        state.storage.kv_storage.as_ref(),
        &mut sources,
    )
    .await;
    sources
}

/// Resolve Query-role (answer) LLM override for context/MCP paths.
///
/// Name historically said "keyword"; Keyword role is resolved separately via
/// [`resolve_workspace_query_resources`] → `keyword_llm` (SPEC-046 EQ-046-13).
pub async fn resolve_query_llm_override(
    state: &AppState,
    workspace: Option<&edgequake_core::Workspace>,
    propagation: &PropagationHeaders,
    llm_provider: Option<String>,
    llm_model: Option<String>,
) -> ApiResult<Option<Arc<dyn LLMProvider>>> {
    validate_llm_override_pair(llm_provider.as_deref(), llm_model.as_deref())?;
    let resolver = WorkspaceProviderResolver::from_app_state(state);
    let extra_headers = propagation.clone().merge_with(None);
    let llm_request = LlmResolutionRequest {
        provider: llm_provider,
        model: llm_model,
        extra_headers,
    };
    // SPEC-123: load tenant then resolve (LAW-123-2).
    match resolver
        .resolve_llm_provider_for_workspace(workspace, &llm_request)
        .await
    {
        Ok(resolved) => Ok(resolved.map(|r| r.provider)),
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Deprecated alias — use [`resolve_query_llm_override`].
#[deprecated(note = "renamed to resolve_query_llm_override (Query role, not Keyword)")]
pub async fn resolve_keyword_llm_override(
    state: &AppState,
    workspace: Option<&edgequake_core::Workspace>,
    propagation: &PropagationHeaders,
    llm_provider: Option<String>,
    llm_model: Option<String>,
) -> ApiResult<Option<Arc<dyn LLMProvider>>> {
    resolve_query_llm_override(state, workspace, propagation, llm_provider, llm_model).await
}

pub fn build_query_response_subgraph(
    result: &QueryResponse,
    include_subgraph: bool,
    rerank_top_k: Option<usize>,
    reranked: bool,
) -> Option<crate::handlers::context_types::SubgraphBundle> {
    if !include_subgraph {
        return None;
    }
    Some(map_query_context_to_subgraph(
        &result.context,
        &MappingOptions {
            granularity: ContentGranularity::Citation,
            include_lineage: true,
            include_documents: false,
            include_agent_hints: false,
            include_subgraph: true,
            rerank_top_k,
            reranked,
        },
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn build_legacy_query_response(
    result: QueryResponse,
    sources: Vec<SourceReference>,
    conversation_id: Option<String>,
    reranked: bool,
    llm_provider: Option<String>,
    llm_model: Option<String>,
    include_subgraph: bool,
    rerank_top_k: Option<usize>,
) -> LegacyQueryResponse {
    let subgraph = build_query_response_subgraph(&result, include_subgraph, rerank_top_k, reranked);
    // SPEC-142: attach verified document+page links in the answer body.
    let answer = crate::services::verified_citations::verified_answer(&result.answer, &sources);

    LegacyQueryResponse {
        answer,
        mode: result.mode.to_string(),
        sources,
        subgraph,
        stats: crate::services::query_stats_mapper::from_engine_stats(
            &result.stats,
            &result.context,
            llm_provider,
            llm_model,
        ),
        conversation_id,
        reranked,
        explain: result.explain.map(Into::into),
        trace_id: None,
    }
}
