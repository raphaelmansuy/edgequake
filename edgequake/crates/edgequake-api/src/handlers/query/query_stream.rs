//! Streaming query handler (SSE).
//!
//! @implements UC0203 (Stream Query Response)
//! @implements FEAT0404 (Query Streaming Endpoint)
//! @implements SPEC-006 (Unified Streaming Response)

use axum::{
    extract::State,
    response::sse::{Event, KeepAliveStream, Sse},
    Json,
};
use futures::stream::StreamExt;
use std::convert::Infallible;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};

use crate::error::{ApiError, ApiResult};
use crate::handlers::chat::build_sources;
use crate::handlers::query::resolve_chunk_file_paths;
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::state::AppState;
use crate::streaming::StreamAccumulator;
use crate::validation::validate_query;
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};

use super::workspace_resolve::resolve_query_workspace;
use crate::handlers::query::{get_workspace_embedding_provider, get_workspace_vector_storage};
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
        (status = 200, description = "Streaming query started"),
        (status = 400, description = "Invalid query")
    )
)]
pub async fn stream_query(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<StreamQueryRequest>,
) -> ApiResult<Sse<SseStream>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        query = %request.query,
        "Executing streaming query with tenant context"
    );

    validate_query(&request.query, state.config.max_query_length)?;

    // SPEC-006 FR-004: Check if client wants v1 (raw text) format
    let use_v1 = request
        .stream_format
        .as_deref()
        .map(|f| f == "v1")
        .unwrap_or(false);

    // Parse query mode
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Hybrid);

    // Build engine query request with tenant context
    let mut engine_request = EngineQueryRequest::new(&request.query).with_mode(mode);

    // SPEC-004: Thread system prompt extension if provided
    if let Some(ref system_prompt) = request.system_prompt {
        engine_request = engine_request.with_system_prompt(system_prompt);
    }

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

    if let Some(ref tenant_id) = data_tenant_id {
        engine_request = engine_request.with_tenant_id(tenant_id.clone());
    }
    if let Some(ref workspace_id) = tenant_ctx.workspace_id {
        engine_request = engine_request.with_workspace_id(workspace_id.clone());
    }

    // SPEC-005 + SPEC-006: Resolve document filter
    if let Some(ref filter) = request.document_filter {
        let ws_id_str = tenant_ctx.workspace_id.clone();
        let tenant_filter = data_tenant_id.clone();
        match crate::handlers::query::document_filter_resolver::resolve_document_filter(
            state.kv_storage.as_ref(),
            filter,
            &tenant_filter,
            &ws_id_str,
        )
        .await
        {
            Ok(Some(allowed_ids)) => {
                engine_request = engine_request.with_allowed_document_ids(allowed_ids);
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

    // SPEC-006 + SPEC-032: Resolve LLM provider override
    let workspace_id_str = tenant_ctx.workspace_id.clone();
    let resolver = WorkspaceProviderResolver::new(state.workspace_service.clone());
    let llm_request = LlmResolutionRequest::from_provider_string(
        request.llm_provider.clone(),
        request.llm_model.clone(),
    );

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
            Err(e) => {
                return Err(ApiError::Internal(format!(
                    "LLM provider resolution failed: {}",
                    e
                )));
            }
        };

    // SPEC-006: v1 backward-compatible mode - raw text streaming
    if use_v1 {
        let stream = state
            .sota_engine
            .query_stream(engine_request)
            .await
            .map_err(|e| ApiError::Internal(format!("Streaming query failed: {}", e)))?;

        let sse_stream: BoxedSseStream = Box::pin(stream.map(|res| match res {
            Ok(text) => Ok(Event::default().data(text)),
            Err(e) => Ok(Event::default().data(format!("Error: {}", e))),
        }));

        return Ok(Sse::new(sse_stream).keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keep-alive"),
        ));
    }

    // SPEC-006: v2 structured event streaming
    let (tx, rx) = mpsc::channel::<QueryStreamEvent>(100);
    let state_clone = state.clone();

    tokio::spawn(async move {
        let retrieval_start = std::time::Instant::now();

        // Resolve workspace-specific providers
        let (ws_embedding_provider, ws_vector_storage) = if let Some(ref ws_id_str) =
            workspace_id_str
        {
            let embed_provider =
                match get_workspace_embedding_provider(&state_clone, ws_id_str).await {
                    Ok(Some(p)) => Some(p),
                    Ok(None) => None,
                    Err(e) => {
                        error!(error = %e, "Cannot create workspace embedding provider");
                        let _ = tx
                            .send(QueryStreamEvent::Error {
                                message: format!("Embedding provider error: {}", e),
                                code: "EMBEDDING_PROVIDER_CONFIG_ERROR".to_string(),
                            })
                            .await;
                        return;
                    }
                };

            let vector_storage = match get_workspace_vector_storage(&state_clone, ws_id_str).await {
                Ok(Some(s)) => Some(s),
                Ok(None) => None,
                Err(e) => {
                    error!(error = %e, "Cannot get workspace vector storage");
                    let _ = tx
                        .send(QueryStreamEvent::Error {
                            message: format!("Vector storage error: {}", e),
                            code: "VECTOR_STORAGE_ERROR".to_string(),
                        })
                        .await;
                    return;
                }
            };

            (embed_provider, vector_storage)
        } else {
            (None, None)
        };

        // Execute streaming query with context - dispatch based on available providers
        let stream_result = match (&ws_embedding_provider, &ws_vector_storage) {
            (Some(embed), Some(vector)) => {
                state_clone
                    .sota_engine
                    .query_stream_with_full_config(
                        engine_request,
                        embed.clone(),
                        vector.clone(),
                        llm_override.clone(),
                    )
                    .await
            }
            (Some(embed), None) => {
                state_clone
                    .sota_engine
                    .query_stream_with_full_config(
                        engine_request,
                        embed.clone(),
                        state_clone.vector_storage.clone(),
                        llm_override.clone(),
                    )
                    .await
            }
            _ => {
                if let Some(ref llm) = llm_override {
                    state_clone
                        .sota_engine
                        .query_stream_with_context_and_llm(engine_request, llm.clone())
                        .await
                } else {
                    state_clone
                        .sota_engine
                        .query_stream_with_context(engine_request)
                        .await
                }
            }
        };

        match stream_result {
            Ok((context, used_mode, mut stream)) => {
                let retrieval_time_ms = retrieval_start.elapsed().as_millis() as u64;

                // Build and enrich sources
                let mut sources = build_sources(&context);
                resolve_chunk_file_paths(state_clone.kv_storage.as_ref(), &mut sources).await;

                // SPEC-006 FR-001: Emit context event BEFORE tokens
                let context_event = QueryStreamEvent::Context {
                    sources,
                    query_mode: used_mode.to_string(),
                    retrieval_time_ms,
                };
                if tx.send(context_event).await.is_err() {
                    warn!("Client disconnected before receiving context event");
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
                                warn!("Client disconnected during streaming");
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Streaming error: {}", e);
                            let _ = tx
                                .send(QueryStreamEvent::Error {
                                    message: e.to_string(),
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
                error!("Failed to start streaming query: {}", e);
                let _ = tx
                    .send(QueryStreamEvent::Error {
                        message: e.to_string(),
                        code: "QUERY_FAILED".to_string(),
                    })
                    .await;
            }
        }
    });

    // Convert channel to SSE stream
    let sse_stream: BoxedSseStream = Box::pin(ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok::<_, Infallible>(Event::default().data(json))
    }));

    Ok(Sse::new(sse_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
