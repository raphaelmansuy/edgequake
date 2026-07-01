//! Streaming query handler (SSE).
//!
//! @implements UC0203 (Stream Query Response)
//! @implements FEAT0404 (Query Streaming Endpoint)
//! @implements SPEC-006 (Unified Streaming Response)

use axum::{
    extract::State,
    response::sse::{Event, KeepAliveStream, Sse},
    Extension, Json,
};
use edgequake_observability::{
    record_llm_request, record_query_completed, scope_llm_provider, ErrorEvent, PropagationHeaders,
    QueryFailureGuard, RequestContext,
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
use crate::streaming::StreamAccumulator;
use crate::validation::validate_query;
use edgequake_query::QueryMode;

use super::workspace_resolve::resolve_query_workspace;
pub use crate::handlers::query_types::{QueryStreamEvent, QueryStreamStats, StreamQueryRequest};

type BoxedSseStream = Pin<Box<dyn futures::Stream<Item = Result<Event, Infallible>> + Send>>;

type SseStream = KeepAliveStream<BoxedSseStream>;

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
) -> ApiResult<Sse<SseStream>> {
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

    let mut allowed_document_ids = None;
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
                allowed_document_ids = Some(allowed_ids);
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
        allowed_document_ids,
        data_tenant_id: data_tenant_id.clone(),
        workspace_id: tenant_ctx.workspace_id.clone(),
        llm_provider: request.llm_provider.clone(),
        llm_model: request.llm_model.clone(),
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

    let (llm_override, used_provider, used_model) =
        match resolver.resolve_llm_provider_with_workspace(workspace.as_ref(), &llm_request) {
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
        return Ok(Sse::new(sse_stream).keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keep-alive"),
        ));
    }

    // SPEC-006: v2 structured event streaming
    let (tx, rx) = mpsc::channel::<QueryStreamEvent>(100);
    let state_clone = state.clone();
    let stream_mode = query_mode_label.clone();
    let stream_request_id = request_id.clone();
    let stream_include_subgraph = request.include_subgraph;
    let stream_use_v3 = use_v3;
    let stream_content_granularity = request.content_granularity;

    let spawn_provider = used_provider
        .clone()
        .unwrap_or_else(|| "default".to_string());
    tokio::spawn(async move {
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
                Ok((context, used_mode, mut stream)) => {
                    let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;

                    // Build and enrich sources
                    let mut sources = build_sources(&context, stream_content_granularity);
                    resolve_chunk_file_paths(state_clone.storage.kv_storage.as_ref(), &mut sources)
                        .await;

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

                    // Stream tokens
                    let gen_start = std::time::Instant::now();
                    let mut accumulator = StreamAccumulator::new();

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(text) => {
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
                            stats: QueryStreamStats {
                                embedding_time_ms: 0, // Included in retrieval_time_ms
                                retrieval_time_ms,
                                generation_time_ms,
                                total_time_ms,
                                sources_retrieved: context.chunks.len()
                                    + context.entities.len()
                                    + context.relationships.len(),
                                tokens_used,
                                tokens_per_second,
                                query_mode: used_mode.to_string(),
                            },
                            llm_provider: used_provider,
                            llm_model: used_model,
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
    });

    // Convert channel to SSE stream
    let sse_stream: BoxedSseStream = Box::pin(ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok::<_, Infallible>(Event::default().data(json))
    }));

    query_guard.dismiss();
    Ok(Sse::new(sse_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
