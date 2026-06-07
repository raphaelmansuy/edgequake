//! Execute RAG query handler.
//!
//! @implements UC0201 (Execute Query)
//! @implements FEAT0007 (Multi-Mode Query Execution)
//! @implements FEAT0101-0106 (Query modes)

use axum::{extract::State, Extension, Json};
use edgequake_audit::{AuditEvent, AuditEventType, AuditResult};
use edgequake_observability::{
    record_llm_request, scope_llm_provider, PropagationHeaders, QueryOutcomeGuard, RequestContext,
};
use tracing::debug;

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::services::{
    execute_sota_query_with_auth_fallback, record_audit, resolve_workspace_query_resources,
    validate_llm_override_pair, with_request_context,
};
use crate::state::AppState;
use crate::validation::validate_query;
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};

use super::{
    resolve_chunk_file_paths,
    workspace_resolve::{get_workspace_llm_info, resolve_query_workspace},
};
pub use crate::handlers::query_types::{QueryRequest, QueryResponse, QueryStats, SourceReference};

/// Execute a RAG query with multi-mode retrieval.
///
/// # Implements
///
/// - **UC0201**: Execute Query
/// - **FEAT0007**: Multi-Mode Query Execution
/// - **FEAT0101**: Naive mode (vector search only)
/// - **FEAT0102**: Local mode (entity-centric)
/// - **FEAT0103**: Global mode (community summaries)
/// - **FEAT0104**: Hybrid mode (local + global)
/// - **FEAT0105**: Mix mode (adaptive blend)
/// - **FEAT0106**: Bypass mode (direct LLM, no RAG)
///
/// # Enforces
///
/// - **BR0101**: Token budget enforcement
/// - **BR0103**: Mode validation
/// - **BR0201**: Tenant/workspace scoping
///
/// # Returns
///
/// - `response`: LLM-generated answer
/// - `sources`: Source references with document lineage
/// - `stats`: Retrieval statistics (chunks, entities, latency)
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
    Extension(req_ctx): Extension<RequestContext>,
    Extension(propagation): Extension<PropagationHeaders>,
    Json(request): Json<QueryRequest>,
) -> ApiResult<Json<QueryResponse>> {
    let query_mode_label = request.mode.as_deref().unwrap_or("hybrid").to_string();
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

    // Parse query mode
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Hybrid);

    // Build engine query request with conversation history and tenant context
    let mut engine_request = EngineQueryRequest::new(&request.query).with_mode(mode);

    // SPEC-004: Thread system prompt extension if provided
    if let Some(ref system_prompt) = request.system_prompt {
        engine_request = engine_request.with_system_prompt(system_prompt);
    }

    // OODA-231.1: Fetch workspace to get correct tenant_id for data queries.
    // WHY: The header tenant_id is an access-context hint, but stored graph and
    // vector data are scoped by the workspace's persisted tenant_id. If an
    // explicit workspace header is invalid, we must fail closed instead of
    // silently falling back to the default workspace.
    let workspace = resolve_query_workspace(&state, tenant_ctx.workspace_id.as_deref()).await?;

    // Use the workspace tenant_id when a workspace is selected; otherwise fall
    // back to the legacy header-only path for default-workspace requests.
    let data_tenant_id = workspace
        .as_ref()
        .map(|ws| ws.tenant_id.to_string())
        .or_else(|| tenant_ctx.tenant_id.clone());

    if let Some(ref tenant_id) = data_tenant_id {
        engine_request = engine_request.with_tenant_id(tenant_id.clone());
    }
    if let Some(ref workspace_id) = tenant_ctx.workspace_id {
        engine_request = engine_request.with_workspace_id(workspace_id.clone());
    }

    if request.context_only {
        engine_request = engine_request.context_only();
    }

    if request.prompt_only {
        engine_request = engine_request.prompt_only();
    }

    // Add rerank settings to engine request
    engine_request = engine_request.with_rerank(request.enable_rerank);
    if let Some(top_k) = request.rerank_top_k {
        engine_request = engine_request.with_rerank_top_k(top_k);
    }

    // SPEC-032: Add LLM provider/model overrides if provided in request
    // This allows query-time override of the LLM provider and model
    if let Some(ref provider) = request.llm_provider {
        debug!(provider = %provider, "Using LLM provider override from request");
        engine_request = engine_request.with_llm_provider(provider);
    }
    if let Some(ref model) = request.llm_model {
        debug!(model = %model, "Using LLM model override from request");
        engine_request = engine_request.with_llm_model(model);
    }

    // Add conversation history if provided
    if let Some(history) = &request.conversation_history {
        let engine_history: Vec<edgequake_query::ConversationMessage> = history
            .iter()
            .map(|m| edgequake_query::ConversationMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        engine_request = engine_request.with_conversation_history(engine_history);
    }

    // SPEC-005: Resolve document filter → allowed_document_ids
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
            engine_request = engine_request.with_allowed_document_ids(allowed_ids);
        }
    }

    // SPEC-032 & SPEC-033: Get workspace-specific embedding provider AND vector storage
    // If workspace has custom embedding config, use workspace-specific resources

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

    // Convert sources from context
    let mut sources = Vec::new();

    // Apply simple relevance-based reranking if enabled
    // In a production environment, this would call an external reranker service (e.g., Cohere)
    let reranked = request.enable_rerank;
    let rerank_time_ms = if reranked {
        // Simulate rerank time for now - actual implementation would call rerank API
        Some(5u64)
    } else {
        None
    };

    // Get rerank_top_k or default to all results
    let rerank_top_k = request.rerank_top_k.unwrap_or(usize::MAX);

    // Build chunk sources with rerank scores
    let mut ref_counter = 1usize;
    let mut chunk_sources: Vec<SourceReference> = result
        .context
        .chunks
        .iter()
        .map(|chunk| {
            // Calculate simulated rerank score based on original score
            let rerank_score = if reranked {
                // Normalize score to 0-1 range and apply slight boost
                Some((chunk.score.min(1.0) * 0.95 + 0.05).min(1.0))
            } else {
                None
            };

            let ref_id = ref_counter;
            ref_counter += 1;

            SourceReference {
                source_type: "chunk".to_string(),
                id: chunk.id.clone(),
                score: chunk.score,
                rerank_score,
                snippet: Some(chunk.content.chars().take(200).collect()),
                reference_id: Some(ref_id),
                document_id: chunk.document_id.clone(),
                file_path: None, // Resolved below via KV metadata lookup
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                chunk_index: chunk.chunk_index,
                entity_type: None,
                degree: None,
                source_chunk_ids: None,
            }
        })
        .collect();

    // Resolve document_id → file_path (document title) for chunk sources
    resolve_chunk_file_paths(state.storage.kv_storage.as_ref(), &mut chunk_sources).await;

    // SPEC-0002: Exclude injection artifacts from cited sources.
    // Injection chunks enrich LLM context but must NOT appear as source citations.
    chunk_sources.retain(|s| {
        !s.document_id
            .as_deref()
            .unwrap_or("")
            .starts_with("injection::")
    });

    // Sort by rerank score if reranking is enabled
    if reranked {
        chunk_sources.sort_by(|a, b| {
            b.rerank_score
                .unwrap_or(0.0)
                .partial_cmp(&a.rerank_score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        chunk_sources.truncate(rerank_top_k);
    }

    sources.extend(chunk_sources);

    for entity in &result.context.entities {
        // SPEC-0002: Skip injection-only entities from citations.
        // Injection source_document_id is "injection::{workspace_id}::{id}".
        // These entities enrich graph context but must not be cited as sources.
        if entity
            .source_document_id
            .as_deref()
            .unwrap_or("")
            .starts_with("injection::")
        {
            continue;
        }

        let ref_id = ref_counter;
        ref_counter += 1;

        sources.push(SourceReference {
            source_type: "entity".to_string(),
            id: entity.name.clone(),
            score: entity.score,
            rerank_score: None,
            snippet: Some(entity.description.chars().take(200).collect()),
            reference_id: Some(ref_id),
            document_id: entity.source_document_id.clone(),
            file_path: entity.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
            // SPEC-006: Enrich entity metadata
            entity_type: Some(entity.entity_type.clone()),
            degree: if entity.degree > 0 {
                Some(entity.degree)
            } else {
                None
            },
            source_chunk_ids: if entity.source_chunk_ids.is_empty() {
                None
            } else {
                Some(entity.source_chunk_ids.clone())
            },
        });
    }

    for rel in &result.context.relationships {
        // SPEC-0002: Skip injection-only relationships from citations.
        if rel
            .source_document_id
            .as_deref()
            .unwrap_or("")
            .starts_with("injection::")
        {
            continue;
        }

        let ref_id = ref_counter;
        ref_counter += 1;

        sources.push(SourceReference {
            source_type: "relationship".to_string(),
            id: format!("{}->{}", rel.source, rel.target),
            score: rel.score,
            rerank_score: None,
            snippet: Some(format!(
                "{} {} {}",
                rel.source, rel.relation_type, rel.target
            )),
            reference_id: Some(ref_id),
            document_id: rel.source_document_id.clone(),
            file_path: rel.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
        });
    }

    // Generate conversation ID if conversation history was provided
    let conversation_id = if request.conversation_history.is_some() {
        Some(uuid::Uuid::new_v4().to_string())
    } else {
        None
    };

    // SPEC-032 Item 18, 22: Get LLM provider/model info for lineage tracking
    let (llm_provider, llm_model) =
        get_workspace_llm_info(&state, tenant_ctx.workspace_id.as_deref()).await;

    // SPEC-032 Item 18: Calculate tokens per second
    let tokens_used = if result.stats.generated_tokens > 0 {
        Some(result.stats.generated_tokens)
    } else {
        None
    };

    let tokens_per_second =
        if result.stats.generation_time_ms > 0 && result.stats.generated_tokens > 0 {
            Some(
                (result.stats.generated_tokens as f32) / (result.stats.generation_time_ms as f32)
                    * 1000.0,
            )
        } else {
            None
        };

    let tenant_for_audit = data_tenant_id
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

    let response = QueryResponse {
        answer: result.answer,
        mode: result.mode.to_string(),
        sources,
        stats: QueryStats {
            embedding_time_ms: result.stats.embedding_time_ms,
            retrieval_time_ms: result.stats.retrieval_time_ms,
            generation_time_ms: result.stats.generation_time_ms,
            total_time_ms: result.stats.total_time_ms,
            sources_retrieved: result.context.chunks.len()
                + result.context.entities.len()
                + result.context.relationships.len(),
            rerank_time_ms,
            // SPEC-032 Item 18, 22: Token metrics and model lineage
            tokens_used,
            tokens_per_second,
            llm_provider,
            llm_model,
        },
        conversation_id,
        reranked,
    };

    Ok(Json(response))
}
