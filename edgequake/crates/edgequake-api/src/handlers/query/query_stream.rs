//! Streaming query handler (SSE).
//!
//! @implements UC0203 (Stream Query Response)
//! @implements FEAT0404 (Query Streaming Endpoint)
//! @implements SPEC-006 (Unified Streaming Response)

use axum::{extract::State, response::sse::Event, response::Response, Extension, Json};
use edgequake_observability::{
    record_llm_request, record_query_completed, record_query_root_io, scope_llm_provider,
    stamp_query_langfuse_identity, ErrorEvent, PropagationHeaders, QueryFailureGuard,
    RequestContext,
};
use futures::stream::StreamExt;
use serde_json::json;
use std::convert::Infallible;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info};

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::OptionalAuth;
use crate::handlers::chat::build_sources;
use crate::handlers::query::resolve_chunk_file_paths;
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::services::{
    build_engine_request, ensure_debug_granularity_allowed,
    execute_sota_query_stream_with_auth_fallback, resolve_workspace_query_resources,
    validate_llm_override_pair, QueryExecutionParams,
};
use crate::state::AppState;
use crate::streaming::{live_sse, StreamAccumulator};
use crate::validation::validate_query;
use edgequake_query::QueryMode;

use super::workspace_resolve::resolve_query_workspace;
pub use crate::handlers::query_types::{QueryStreamEvent, QueryStreamStats, StreamQueryRequest};

type BoxedSseStream = Pin<Box<dyn futures::Stream<Item = Result<Event, Infallible>> + Send>>;

/// Execute a streaming query.
///
/// SPEC-006: Emits structured SSE events with context, tokens, and statistics.
/// Supports backward-compatible v1 (raw text) via `stream_format` parameter.
#[utoipa::path(
    post,
    path = "/api/v1/query/stream",
    tag = "Query",
    request_body = StreamQueryRequest,
    responses(
        (status = 200, description = "Streaming query SSE (QueryStreamEvent payloads)",
            content(
                (QueryStreamEvent = "text/event-stream")
            )
        ),
        (status = 400, description = "Invalid query")
    )
)]
#[tracing::instrument(
    name = "query_stream",
    skip(state, tenant_ctx, propagation, request),
    fields(
        request_id = %req_ctx.request_id,
        query.mode = tracing::field::Empty,
        stream.format = tracing::field::Empty,
        otel.name = "query_stream",
    )
)]
pub async fn stream_query(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    OptionalAuth(auth_user): OptionalAuth,
    Extension(req_ctx): Extension<RequestContext>,
    Extension(propagation): Extension<PropagationHeaders>,
    Json(request): Json<StreamQueryRequest>,
) -> ApiResult<Response> {
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Mix);
    let query_mode_label = mode.to_string();
    tracing::Span::current().record("query.mode", query_mode_label.as_str());
    let query_guard = QueryFailureGuard::new(&query_mode_label);
    let request_id = req_ctx.request_id.clone();

    debug!(
        request_id = %request_id,
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        query.mode = %query_mode_label,
        query = %request.query,
        "Executing streaming query with tenant context"
    );

    validate_query(&request.query, state.config.max_query_length)?;
    ensure_debug_granularity_allowed(
        request.content_granularity,
        auth_user.as_ref().map(|u| u.role.clone()),
    )?;

    // SPEC-006 FR-004: Check stream format
    let use_v1 = request
        .stream_format
        .as_deref()
        .map(|f| f == "v1")
        .unwrap_or(false);
    let use_v3 = request.stream_format.as_deref() == Some("v3");
    tracing::Span::current().record("stream.format", if use_v1 { "v1" } else { "v2" });

    // OODA-231.1: Resolve the workspace before the SSE stream starts.
    // WHY: If the client explicitly names a workspace and it is invalid or
    // missing, returning a normal 200 stream would hide an isolation failure.
    let workspace = resolve_query_workspace(&state, tenant_ctx.workspace_id.as_deref()).await?;

    // Use workspace's tenant_id for data queries, fall back to header tenant_id
    // only for the legacy no-workspace path.
    let data_tenant_id = workspace
        .as_ref()
        .map(|ws| ws.tenant_id.to_string())
        .or_else(|| tenant_ctx.tenant_id.clone());

    // SPEC-124: optional client session — never synthesize from request_id.
    let langfuse_id = super::workspace_resolve::langfuse_query_identity(
        &state,
        request.session_id.as_deref(),
        tenant_ctx.user_id.as_deref(),
        data_tenant_id
            .as_deref()
            .or(tenant_ctx.tenant_id.as_deref()),
        workspace.as_ref(),
    )
    .await;
    let _langfuse_identity = stamp_query_langfuse_identity(langfuse_id.clone());

    let mut client_filter_ids = None;
    // SPEC-005 + SPEC-006: Resolve document filter
    if let Some(ref filter) = request.document_filter {
        let ws_id_str = tenant_ctx.workspace_id.clone();
        let tenant_filter = data_tenant_id.clone();
        match crate::handlers::query::document_filter_resolver::resolve_document_filter(
            state.storage.kv_storage.as_ref(),
            filter,
            &tenant_filter,
            &ws_id_str,
        )
        .await
        {
            Ok(Some(allowed_ids)) => {
                client_filter_ids = Some(allowed_ids);
            }
            Ok(None) => {}
            Err(e) => {
                return Err(ApiError::Internal(format!(
                    "Document filter resolution failed: {}",
                    e
                )));
            }
        }
    }

    let user_id = auth_user
        .as_ref()
        .map(|u| u.user_id.to_string())
        .or_else(|| tenant_ctx.user_id.clone());
    let (allowed_document_ids, authz_ctx, allow_set) =
        crate::services::spec146_authz::resolve_query_allowed_document_ids(
            &state,
            &tenant_ctx,
            user_id.as_deref(),
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
        query: request.query.clone(),
        mode,
        max_results: None,
        context_only: false,
        prompt_only: false,
        enable_rerank: true,
        rerank_top_k: None,
        mix_weights: None,
        conversation_history: None,
        system_prompt: request.system_prompt.clone(),
        question_type: request.question_type.clone(),
        hl_keywords: request.hl_keywords.clone(),
        ll_keywords: request.ll_keywords.clone(),
        response_type: request.response_type.clone(),
        allowed_document_ids: allowed_document_ids.clone(),
        data_tenant_id: data_tenant_id.clone(),
        workspace_id: tenant_ctx.workspace_id.clone(),
        llm_provider: request.llm_provider.clone(),
        llm_model: request.llm_model.clone(),
        reasoning_effort: {
            let provider_for_effort = request
                .llm_provider
                .as_deref()
                .or_else(|| workspace.as_ref().map(|w| w.llm_provider.as_str()))
                .unwrap_or("openai");
            let model_for_effort = request
                .llm_model
                .as_deref()
                .or_else(|| workspace.as_ref().map(|w| w.llm_model.as_str()))
                .unwrap_or("");
            crate::services::resolve_query_reasoning_effort(
                workspace.as_ref(),
                provider_for_effort,
                model_for_effort,
                request.reasoning_effort.as_deref(),
                None,
            )
        },
        authz_principal,
        policy_generation,
        allow_fingerprint,
    };
    let engine_request = build_engine_request(&params);

    // SPEC-006 + SPEC-032: Resolve LLM provider override
    validate_llm_override_pair(
        request.llm_provider.as_deref(),
        request.llm_model.as_deref(),
    )?;

    let workspace_id_str = tenant_ctx.workspace_id.clone();
    let resolver = WorkspaceProviderResolver::from_app_state(&state);
    let extra_headers = propagation.merge_with(request.extra_headers.clone());
    let llm_request = LlmResolutionRequest {
        provider: request.llm_provider.clone(),
        model: request.llm_model.clone(),
        extra_headers,
    };

    let (llm_override, used_provider, used_model) = match resolver
        .resolve_llm_provider_for_workspace(workspace.as_ref(), &llm_request)
        .await
    {
        Ok(Some(resolved)) => {
            info!(
                provider = %resolved.provider_name,
                model = %resolved.model_name,
                source = ?resolved.source,
                "Resolved LLM provider for streaming query"
            );
            (
                Some(resolved.provider),
                Some(resolved.provider_name),
                Some(resolved.model_name),
            )
        }
        Ok(None) => (None, None, None),
        Err(e) => return Err(ApiError::from(e)),
    };

    // Honesty: always record effective provider/model (incl. server-default path).
    let effective_llm = llm_override
        .as_ref()
        .map(|p| p.as_ref())
        .unwrap_or_else(|| state.query.llm_provider.as_ref());
    let (used_provider, used_model) = {
        let (p, m) = crate::handlers::chat::coalesce_effective_llm_lineage(
            used_provider,
            used_model,
            effective_llm,
        );
        (Some(p), Some(m))
    };

    // SPEC-006: v1 backward-compatible mode - raw text streaming (workspace-aware)
    if use_v1 {
        let resources =
            resolve_workspace_query_resources(&state, tenant_ctx.workspace_id.as_deref()).await?;

        let provider_label = used_provider
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let (_, _, stream) = scope_llm_provider(
            provider_label,
            execute_sota_query_stream_with_auth_fallback(
                &state,
                engine_request,
                resources,
                llm_override.clone(),
            ),
        )
        .await
        .map_err(ApiError::from)?;

        let v1_request_id = request_id.clone();
        let v1_mode = query_mode_label.clone();
        let v1_provider = used_provider
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let sse_stream: BoxedSseStream = Box::pin(stream.map(move |res| match res {
            Ok(text) => Ok(Event::default().data(text)),
            Err(e) => {
                let msg = e.to_string();
                record_llm_request(&v1_provider, "query_stream_v1", "failure", 0.0);
                ErrorEvent::log_stream_error(
                    &v1_request_id,
                    "query_stream",
                    "STREAM_ERROR_V1",
                    &msg,
                    json!({ "phase": "v1_token", "mode": v1_mode }),
                );
                Ok(Event::default().data(format!("Error: {msg}")))
            }
        }));

        query_guard.dismiss();
        return Ok(live_sse(sse_stream));
    }

    // SPEC-006: v2 structured event streaming
    let (tx, rx) = mpsc::channel::<QueryStreamEvent>(100);
    let state_clone = state.clone();
    let stream_mode = query_mode_label.clone();
    let stream_request_id = request_id.clone();
    let stream_include_subgraph = request.include_subgraph;
    let stream_use_v3 = use_v3;
    let stream_content_granularity = request.content_granularity;
    let stream_query_text = request.query.clone();
    let stream_allowed_document_ids = allowed_document_ids.clone();

    let spawn_provider = used_provider
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let langfuse_id_spawn = langfuse_id.clone();
    tokio::spawn(async move {
        edgequake_observability::bind_langfuse_trace_identity_async(langfuse_id_spawn, async {
        scope_llm_provider(spawn_provider.clone(), async {
            let retrieval_start = std::time::Instant::now();

            let resources =
                match resolve_workspace_query_resources(&state_clone, workspace_id_str.as_deref())
                    .await
                {
                    Ok(resources) => resources,
                    Err(e) => {
                        let msg = e.to_string();
                        ErrorEvent::log_stream_error(
                            &stream_request_id,
                            "query_stream",
                            "WORKSPACE_QUERY_CONFIG_ERROR",
                            &msg,
                            json!({ "phase": "workspace_resolve" }),
                        );
                        record_query_completed(
                            &stream_mode,
                            "failure",
                            retrieval_start.elapsed().as_secs_f64(),
                        );
                        let _ = tx
                            .send(QueryStreamEvent::Error {
                                message: msg,
                                code: "WORKSPACE_QUERY_CONFIG_ERROR".to_string(),
                            })
                            .await;
                        return;
                    }
                };

            let stream_result = execute_sota_query_stream_with_auth_fallback(
                &state_clone,
                engine_request,
                resources,
                llm_override.clone(),
            )
            .await;

            match stream_result {
                Ok((mut context, used_mode, mut stream)) => {
                    let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;

                    // SPEC-083 X-22: emit Thinking before Context (do not delete variant).
                    let thinking = QueryStreamEvent::Thinking {
                        content: format!(
                            "Retrieved context via {used_mode} ({} chunks, {} entities, {} relationships)",
                            context.chunks.len(),
                            context.entities.len(),
                            context.relationships.len()
                        ),
                    };
                    if tx.send(thinking).await.is_err() {
                        ErrorEvent::log_stream_disconnect(
                            &stream_request_id,
                            "query_stream",
                            "thinking_event",
                        );
                        return;
                    }

                    // SPEC-146: allow-set safety net before citations/SSE (no stream-then-redact).
                    if let Some(ref ids) = stream_allowed_document_ids {
                        edgequake_query::context_filter::filter_context_by_document_ids(
                            &mut context,
                            Some(ids.as_slice()),
                        );
                    }

                    // Build and enrich sources
                    let mut sources = build_sources(&context, stream_content_granularity);
                    resolve_chunk_file_paths(state_clone.storage.kv_storage.as_ref(), &mut sources)
                        .await;
                    let sources_for_verify = sources.clone();

                    // SPEC-006 FR-001: Emit context event BEFORE tokens
                    let mapping_opts = crate::services::context_bundle_mapper::MappingOptions {
                        granularity: stream_content_granularity,
                        include_lineage: true,
                        include_documents: false,
                        include_agent_hints: false,
                        include_subgraph: stream_include_subgraph,
                        rerank_top_k: None,
                        reranked: false,
                    };
                    let bundle = if stream_use_v3 {
                        Some(
                            crate::services::context_bundle_mapper::map_query_context_to_bundle(
                                &context,
                                &mapping_opts,
                                &std::collections::HashMap::new(),
                            ),
                        )
                    } else {
                        None
                    };
                    let subgraph = if stream_use_v3 || !stream_include_subgraph {
                        None
                    } else {
                        Some(
                            crate::services::context_bundle_mapper::map_query_context_to_subgraph(
                                &context,
                                &mapping_opts,
                            ),
                        )
                    };
                    let context_event = QueryStreamEvent::Context {
                        sources,
                        query_mode: used_mode.to_string(),
                        retrieval_time_ms,
                        subgraph,
                        bundle,
                    };
                    if tx.send(context_event).await.is_err() {
                        ErrorEvent::log_stream_disconnect(
                            &stream_request_id,
                            "query_stream",
                            "context_event",
                        );
                        return;
                    }

                    info!(
                        entities = context.entities.len(),
                        relationships = context.relationships.len(),
                        chunks = context.chunks.len(),
                        mode = %used_mode,
                        "Sent context event for streaming query"
                    );

                    // Stream tokens (064: stamp TTFT on first non-empty token)
                    let gen_start = std::time::Instant::now();
                    let mut accumulator = StreamAccumulator::new();
                    let mut ttft_ms: Option<u64> = None;
                    let mut ux_ttft_ms: Option<u64> = None;

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(text) => {
                                if ttft_ms.is_none() && !text.is_empty() {
                                    ttft_ms = Some(gen_start.elapsed().as_millis() as u64);
                                    ux_ttft_ms = Some(retrieval_start.elapsed().as_millis() as u64);
                                }
                                accumulator.append_content(&text);
                                let event = QueryStreamEvent::Token {
                                    content: text.clone(),
                                };
                                if tx.send(event).await.is_err() {
                                    ErrorEvent::log_stream_disconnect(
                                        &stream_request_id,
                                        "query_stream",
                                        "token_stream",
                                    );
                                    break;
                                }
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                record_llm_request(
                                    &spawn_provider,
                                    "query_stream_token",
                                    "failure",
                                    0.0,
                                );
                                ErrorEvent::log_stream_error(
                                    &stream_request_id,
                                    "query_stream",
                                    "STREAM_ERROR",
                                    &msg,
                                    json!({ "phase": "token_stream" }),
                                );
                                record_query_completed(
                                    &stream_mode,
                                    "failure",
                                    retrieval_start.elapsed().as_secs_f64(),
                                );
                                let _ = tx
                                    .send(QueryStreamEvent::Error {
                                        message: msg,
                                        code: "STREAM_ERROR".to_string(),
                                    })
                                    .await;
                                return;
                            }
                        }
                    }

                    // SPEC-006 FR-003: Emit done event with stats
                    let generation_time_ms = gen_start.elapsed().as_millis() as u64;
                    let tokens_used = accumulator.estimated_tokens();
                    let full_answer = accumulator.content().to_string();
                    // SPEC-142: verified links from retrieval catalog (not LLM prose).
                    let verified = crate::services::verified_citations::verified_answer(
                        &full_answer,
                        &sources_for_verify,
                    );
                    record_query_root_io(&stream_query_text, &verified);
                    let total_time_ms = retrieval_time_ms + generation_time_ms;
                    let tokens_per_second = if generation_time_ms > 0 {
                        Some(tokens_used as f32 / (generation_time_ms as f32 / 1000.0))
                    } else {
                        None
                    };

                    record_query_completed(&stream_mode, "success", total_time_ms as f64 / 1000.0);

                    if generation_time_ms > 0 {
                        record_llm_request(
                            used_provider.as_deref().unwrap_or("unknown"),
                            "query_stream_generation",
                            "success",
                            generation_time_ms as f64 / 1000.0,
                        );
                    }

                    let _ = tx
                        .send(QueryStreamEvent::Done {
                            stats: crate::services::query_stats_mapper::stream_stats_from_context(
                                &context,
                                crate::services::query_stats_mapper::StreamStatsInput {
                                    query_mode: used_mode.to_string(),
                                    retrieval_time_ms,
                                    generation_time_ms,
                                    ttft_ms,
                                    ux_ttft_ms,
                                    tokens_used,
                                    tokens_per_second,
                                    llm_provider: used_provider.clone(),
                                    llm_model: used_model.clone(),
                                },
                            ),
                            llm_provider: used_provider,
                            llm_model: used_model,
                            answer: Some(verified),
                        })
                        .await;
                }
                Err(e) => {
                    let msg = e.to_string();
                    record_llm_request(&spawn_provider, "query_stream_start", "failure", 0.0);
                    ErrorEvent::log_stream_error(
                        &stream_request_id,
                        "query_stream",
                        "QUERY_FAILED",
                        &msg,
                        json!({ "phase": "stream_start" }),
                    );
                    record_query_completed(
                        &stream_mode,
                        "failure",
                        retrieval_start.elapsed().as_secs_f64(),
                    );
                    let _ = tx
                        .send(QueryStreamEvent::Error {
                            message: msg,
                            code: "QUERY_FAILED".to_string(),
                        })
                        .await;
                }
            }
        })
        .await;
        })
        .await;
    });

    // Convert channel to SSE stream
    let sse_stream: BoxedSseStream = Box::pin(ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok::<_, Infallible>(Event::default().data(json))
    }));

    query_guard.dismiss();
    Ok(live_sse(sse_stream))
}
