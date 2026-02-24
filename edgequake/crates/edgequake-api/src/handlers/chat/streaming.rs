//! Streaming chat completion handler (SSE).

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::Json;
use futures::stream::StreamExt;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::query::{
    get_workspace_embedding_provider, get_workspace_vector_storage, resolve_chunk_file_paths,
};
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::state::AppState;
use crate::streaming::StreamAccumulator;
use edgequake_core::types::{
    CreateConversationRequest, CreateMessageRequest, MessageContext, MessageRole,
    UpdateMessageRequest,
};
use edgequake_query::QueryRequest as EngineQueryRequest;

use super::{
    build_sources, enrich_query_with_language, parse_mode, parse_query_mode,
    sources_to_message_context, ChatCompletionRequest, ChatStreamEvent,
};

/// Execute a streaming chat completion.
///
/// Creates conversation and saves user message BEFORE streaming,
/// then saves assistant message AFTER streaming completes.
#[utoipa::path(
    post,
    path = "/api/v1/chat/completions/stream",
    tag = "Chat",
    request_body = ChatCompletionRequest,
    responses(
        (status = 200, description = "Streaming chat completion started"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn chat_completion_stream(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<ChatCompletionRequest>,
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>> {
    // Validate request
    if request.message.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Message cannot be empty".to_string(),
        ));
    }

    let tenant_id = tenant_ctx
        .tenant_id
        .ok_or(ApiError::Unauthorized)?
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("Invalid tenant ID".to_string()))?;
    let user_id = tenant_ctx
        .user_id
        .ok_or(ApiError::Unauthorized)?
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;
    let workspace_id = tenant_ctx
        .workspace_id
        .map(|s| s.parse::<Uuid>())
        .transpose()
        .map_err(|_| ApiError::BadRequest("Invalid workspace ID".to_string()))?;

    debug!(
        tenant_id = %tenant_id,
        user_id = %user_id,
        conversation_id = ?request.conversation_id,
        "Processing streaming chat completion"
    );

    // Ensure user exists in PostgreSQL (auto-create if not)
    // This is necessary because the frontend generates random UUIDs for anonymous users
    #[cfg(feature = "postgres")]
    if let Some(ref pool) = state.pg_pool {
        sqlx::query(
            r#"
            INSERT INTO users (user_id, tenant_id, username, email, password_hash, role, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'anonymous', 'user', TRUE, NOW(), NOW())
            ON CONFLICT (user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(tenant_id)
        .bind(format!("anon_{}", &user_id.to_string()[..8]))
        .bind(format!("{}@anonymous.local", &user_id.to_string()[..8]))
        .execute(pool)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to ensure user exists: {}", e)))?;
    }

    // Validate workspace_id exists in database (may be stale from localStorage)
    // Also store workspace for LLM provider fallback (SPEC-032)
    let (workspace_id, workspace) = if let Some(ws_id) = workspace_id {
        match state.workspace_service.get_workspace(ws_id).await {
            Ok(Some(ws)) => (Some(ws_id), Some(ws)),
            Ok(None) => {
                warn!(workspace_id = %ws_id, "Workspace not found in streaming handler, ignoring stale workspace_id");
                (None, None)
            }
            Err(e) => {
                warn!(workspace_id = %ws_id, error = %e, "Failed to validate workspace in streaming handler, ignoring");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let mode = parse_mode(&request.mode);
    let query_mode = parse_query_mode(&request.mode);

    // FEAT0505: Track whether this is a new conversation for auto-title generation
    let is_new_conversation = request.conversation_id.is_none();

    // 1. Get or create conversation (BEFORE streaming)
    let conversation_id = if let Some(id) = request.conversation_id {
        let conv = state
            .conversation_service
            .get_conversation(id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to get conversation: {}", e)))?
            .ok_or_else(|| ApiError::NotFound(format!("Conversation {} not found", id)))?;

        if conv.tenant_id != tenant_id {
            return Err(ApiError::Forbidden);
        }
        id
    } else {
        let conv = state
            .conversation_service
            .create_conversation(
                tenant_id,
                user_id,
                workspace_id,
                CreateConversationRequest {
                    title: None,
                    mode: Some(mode),
                    folder_id: None,
                },
            )
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create conversation: {}", e)))?;

        info!(conversation_id = %conv.conversation_id, "Created new conversation for streaming");
        conv.conversation_id
    };

    // 2. Save user message (BEFORE streaming)
    let user_message = state
        .conversation_service
        .create_message(
            conversation_id,
            CreateMessageRequest {
                content: request.message.clone(),
                role: MessageRole::User,
                parent_id: request.parent_id,
                stream: true,
            },
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to save user message: {}", e)))?;

    debug!(message_id = %user_message.message_id, "Saved user message before streaming");

    // 3. Create channel for SSE events
    let (tx, rx) = mpsc::channel::<ChatStreamEvent>(100);

    // 4. Clone state for async task
    let state_clone = state.clone();
    let message_content = request.message.clone();
    let user_message_id = user_message.message_id;
    // SPEC-032: Clone provider, model, and workspace for async task
    let request_provider = request.provider.clone();
    let request_model = request.model.clone();
    let workspace_clone = workspace.clone();
    // Clone language for async task - used to enrich query with language directive
    let request_language = request.language.clone();
    // FEAT0505: Clone for auto-title generation
    let first_message_for_title = request.message.clone();

    // 5. Send initial conversation event
    let initial_event = ChatStreamEvent::Conversation {
        conversation_id,
        user_message_id,
    };

    // 6. Spawn background task for LLM streaming
    tokio::spawn(async move {
        // Send initial event
        if tx.send(initial_event).await.is_err() {
            warn!("Client disconnected before receiving initial event");
            return;
        }

        // Use StreamAccumulator for proper token tracking
        let mut accumulator = StreamAccumulator::new();
        // Track message context for saving after streaming completes
        #[allow(unused_assignments)]
        let mut saved_message_context: Option<MessageContext> = None;

        // Build query request
        // OODA-231: Use workspace's tenant_id for graph queries, not header tenant_id.
        // WHY: Header tenant_id is for authentication (random UUID from frontend).
        // But the graph data was ingested with the workspace's actual tenant_id.
        // Using header tenant_id causes 0 results because of tenant_id mismatch.
        let enriched_query = enrich_query_with_language(&message_content, &request_language);
        let mut engine_request = EngineQueryRequest::new(&enriched_query).with_mode(query_mode);
        let data_tenant_id = workspace_clone
            .as_ref()
            .map(|ws| ws.tenant_id.to_string())
            .unwrap_or_else(|| tenant_id.to_string());
        engine_request = engine_request.with_tenant_id(data_tenant_id);
        if let Some(ref ws_id) = workspace_id {
            engine_request = engine_request.with_workspace_id(ws_id.to_string());
        }

        // SPEC-032 + OODA-227: Unified provider resolution with safety limits (streaming)
        // Priority order:
        //   1. Request-specified provider/model (explicit user selection)
        //   2. Workspace-configured provider/model (workspace settings)
        //   3. Server default (sota_engine's default provider)
        // Supports both formats:
        //   - Legacy format: provider="provider/model" (e.g., "ollama/gemma3:12b")
        //   - New format: provider="provider", model="model_name"
        let resolver = WorkspaceProviderResolver::new(state_clone.workspace_service.clone());
        let llm_request = LlmResolutionRequest::from_provider_string(
            request_provider.clone(),
            request_model.clone(),
        );

        // OODA-260: Add detailed logging for LLM provider selection debugging
        debug!(
            request_provider = ?request_provider,
            request_model = ?request_model,
            workspace_id = ?workspace_clone.as_ref().map(|w| &w.workspace_id),
            workspace_llm_provider = ?workspace_clone.as_ref().map(|w| &w.llm_provider),
            workspace_llm_model = ?workspace_clone.as_ref().map(|w| &w.llm_model),
            "LLM provider resolution inputs (streaming)"
        );

        let (llm_override, used_provider, used_model) = match resolver
            .resolve_llm_provider_with_workspace(workspace_clone.as_ref(), &llm_request)
        {
            Ok(Some(resolved)) => {
                info!(
                    provider = %resolved.provider_name,
                    model = %resolved.model_name,
                    source = ?resolved.source,
                    request_provider = ?request_provider,
                    request_model = ?request_model,
                    "✅ [QUERY] Resolved LLM provider (streaming) - using user selection or workspace override"
                );
                (
                    Some(resolved.provider),
                    Some(resolved.provider_name),
                    Some(resolved.model_name),
                )
            }
            Ok(None) => {
                // No provider resolved - will use server default
                info!(
                    request_provider = ?request_provider,
                    request_model = ?request_model,
                    workspace_llm_provider = ?workspace_clone.as_ref().map(|w| &w.llm_provider),
                    "⚠️ [QUERY] Using server default LLM provider (streaming) - neither request nor workspace specified a provider"
                );
                (None, None, None)
            }
            Err(e) => {
                // Explicit provider request failed - send error to client via SSE
                error!(error = %e, "Failed to resolve LLM provider (streaming)");
                let error_msg = e.to_string();
                let _ = tx
                    .send(ChatStreamEvent::Error {
                        message: error_msg,
                        code: "PROVIDER_CONFIG_ERROR".to_string(),
                    })
                    .await;
                return; // Exit task early with error sent
            }
        };

        // Execute streaming query with context using SOTA engine (LightRAG-style)
        // OODA-228: Get workspace embedding provider and vector storage for proper isolation
        let workspace_id_str = workspace_id.as_ref().map(|id| id.to_string());
        let (ws_embedding_provider, ws_vector_storage) = if let Some(ref ws_id_str) =
            workspace_id_str
        {
            // Get workspace embedding provider
            let embed_provider = match get_workspace_embedding_provider(&state_clone, ws_id_str)
                .await
            {
                Ok(Some(p)) => Some(p),
                Ok(None) => {
                    debug!(workspace_id = %ws_id_str, "Workspace using default embedding provider for streaming");
                    None
                }
                Err(e) => {
                    // OODA-228/OODA-229: Send error event with clear message
                    error!(workspace_id = %ws_id_str, error = ?e, "Cannot create workspace embedding provider for streaming");
                    let err_msg = e.to_string();
                    let _ = tx
                        .send(ChatStreamEvent::Error {
                            message: err_msg,
                            code: "EMBEDDING_PROVIDER_CONFIG_ERROR".to_string(),
                        })
                        .await;
                    return; // Exit task early with error sent
                }
            };

            // Get workspace vector storage
            let vector_storage = match get_workspace_vector_storage(&state_clone, ws_id_str).await {
                Ok(Some(s)) => Some(s),
                Ok(None) => {
                    debug!(workspace_id = %ws_id_str, "Workspace using default vector storage for streaming");
                    None
                }
                Err(e) => {
                    // OODA-228: Send error event for vector storage failures too
                    error!(workspace_id = %ws_id_str, error = ?e, "Cannot get workspace vector storage for streaming");
                    let err_msg = format!(
                        "Cannot stream query for workspace: {}. Vector storage error: {:?}",
                        ws_id_str, e
                    );
                    let _ = tx
                        .send(ChatStreamEvent::Error {
                            message: err_msg,
                            code: "VECTOR_STORAGE_ERROR".to_string(),
                        })
                        .await;
                    return; // Exit task early with error sent
                }
            };

            (embed_provider, vector_storage)
        } else {
            (None, None)
        };

        // WHY: Five dispatch paths exist because the SOTA engine needs different
        // combinations of providers. The paths form a priority cascade:
        //
        //   (embed + vector + llm_override)  → full workspace isolation
        //   (embed only + llm_override)      → uses DEFAULT vector storage (potential dimension bug)
        //   (embed only, no llm)             → uses DEFAULT vector storage + DEFAULT LLM
        //   (llm_override only)              → uses DEFAULT embedding + DEFAULT vector storage
        //   (nothing)                        → all-default (server startup providers)
        //
        // The happy path for workspace queries is ALWAYS the first branch
        // (embed + vector + llm_override). If you land in other branches, check
        // whether get_workspace_embedding_provider or get_workspace_vector_storage
        // returned None/Err — that usually means a missing API key or dimension mismatch.
        let stream_result = match (&ws_embedding_provider, &ws_vector_storage) {
            (Some(embed), Some(vector)) => {
                // OODA-228: Use workspace embedding + storage + optional LLM override
                debug!("Using full config for streaming (workspace embedding + vector storage + LLM override)");
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
                // WHY: We have workspace embedding but no workspace-specific vector storage.
                // This is unusual but can happen during workspace migration or misconfiguration.
                // Use workspace embedding + server default vector storage + optional LLM override.
                //
                // Previously this dropped the embedding provider entirely and fell through to
                // query_stream_with_context_and_llm, which used the DEFAULT embedding provider.
                // That caused dimension mismatches when workspace embedding dimension != default.
                //
                // FIX: Use query_stream_with_full_config with the server's default vector storage.
                // This preserves the workspace embedding while using the default vector table.
                warn!("[QUERY] Workspace embedding available but no workspace-specific vector storage - using workspace embedding with server default vector storage");
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
                // No workspace config - use LLM override only
                if let Some(ref llm) = llm_override {
                    debug!("Using LLM provider override for streaming (no workspace config)");
                    state_clone
                        .sota_engine
                        .query_stream_with_context_and_llm(engine_request, llm.clone())
                        .await
                } else {
                    debug!(
                        "Using default configuration for streaming (no workspace or LLM override)"
                    );
                    state_clone
                        .sota_engine
                        .query_stream_with_context(engine_request)
                        .await
                }
            }
        };

        match stream_result {
            Ok((context, _mode, mut stream)) => {
                // Send context event BEFORE streaming tokens (for source citations)
                let mut sources = build_sources(&context);

                // Resolve document names for chunk sources
                resolve_chunk_file_paths(state_clone.kv_storage.as_ref(), &mut sources).await;

                // Save message context for later persistence
                saved_message_context = Some(sources_to_message_context(&sources));

                if !sources.is_empty() {
                    let context_event = ChatStreamEvent::Context {
                        sources: sources.clone(),
                    };
                    if tx.send(context_event).await.is_err() {
                        warn!("Client disconnected before receiving context event");
                        return;
                    }
                    info!(
                        "Sent context event with {} sources ({} entities, {} relationships, {} chunks)",
                        sources.len(),
                        context.entities.len(),
                        context.relationships.len(),
                        context.chunks.len()
                    );
                }

                // Stream tokens
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(text) => {
                            // Accumulate content with proper tracking
                            accumulator.append_content(&text);

                            let event = ChatStreamEvent::Token {
                                content: text.clone(),
                            };
                            if tx.send(event).await.is_err() {
                                warn!("Client disconnected during streaming");
                                // Still save the partial message
                                break;
                            }
                        }
                        Err(e) => {
                            error!("Streaming error: {}", e);
                            let _ = tx
                                .send(ChatStreamEvent::Error {
                                    message: e.to_string(),
                                    code: "STREAM_ERROR".to_string(),
                                })
                                .await;
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to start streaming query: {}", e);
                let _ = tx
                    .send(ChatStreamEvent::Error {
                        message: e.to_string(),
                        code: "QUERY_FAILED".to_string(),
                    })
                    .await;
                return;
            }
        }

        // Get metrics from accumulator (proper token estimation instead of chunk count)
        let duration_ms = accumulator.duration_ms();
        let tokens_used = accumulator.estimated_tokens();
        let full_content = accumulator.content().to_string();

        // 7. Save assistant message (AFTER streaming completes)
        match state_clone
            .conversation_service
            .create_message(
                conversation_id,
                CreateMessageRequest {
                    content: full_content.clone(),
                    role: MessageRole::Assistant,
                    parent_id: Some(user_message_id),
                    stream: true,
                },
            )
            .await
        {
            Ok(assistant_message) => {
                // Update with metadata AND context for source citations
                let _ = state_clone
                    .conversation_service
                    .update_message(
                        assistant_message.message_id,
                        UpdateMessageRequest {
                            content: None,
                            tokens_used: Some(tokens_used as i32),
                            duration_ms: Some(duration_ms as i32),
                            thinking_time_ms: None,
                            context: saved_message_context, // Save context for source citations!
                            is_error: None,
                        },
                    )
                    .await;

                info!(
                    conversation_id = %conversation_id,
                    assistant_message_id = %assistant_message.message_id,
                    tokens_used = tokens_used,
                    duration_ms = duration_ms,
                    chunk_count = accumulator.chunk_count(),
                    llm_provider = ?used_provider,
                    llm_model = ?used_model,
                    "Streaming chat completion successful"
                );

                let _ = tx
                    .send(ChatStreamEvent::Done {
                        assistant_message_id: assistant_message.message_id,
                        tokens_used,
                        duration_ms,
                        // SPEC-032: Provider lineage tracking
                        llm_provider: used_provider.clone(),
                        llm_model: used_model.clone(),
                    })
                    .await;

                // FEAT0505: Auto-generate conversation title for new conversations
                if is_new_conversation {
                    let title_llm =
                        llm_override.unwrap_or_else(|| state_clone.llm_provider.clone());
                    let title_conv_service = state_clone.conversation_service.clone();
                    let title_conv_id = conversation_id;
                    let title_first_msg = first_message_for_title.clone();
                    let title_tx = tx.clone();
                    let title_tenant_id = tenant_id;
                    let title_user_id = user_id;

                    tokio::spawn(async move {
                        let title = crate::handlers::title_generator::generate_title(
                            title_llm,
                            &title_first_msg,
                        )
                        .await;

                        match title_conv_service
                            .update_conversation(
                                title_tenant_id,
                                title_user_id,
                                title_conv_id,
                                edgequake_core::types::UpdateConversationRequest {
                                    title: Some(title.clone()),
                                    ..Default::default()
                                },
                            )
                            .await
                        {
                            Ok(_) => {
                                info!(
                                    conversation_id = %title_conv_id,
                                    title = %title,
                                    "Auto-generated conversation title"
                                );
                                // Send title update event to frontend via SSE
                                let _ = title_tx
                                    .send(ChatStreamEvent::TitleUpdate {
                                        conversation_id: title_conv_id,
                                        title,
                                    })
                                    .await;
                            }
                            Err(e) => {
                                warn!(
                                    conversation_id = %title_conv_id,
                                    error = %e,
                                    "Failed to update conversation title"
                                );
                            }
                        }
                    });
                }
            }
            Err(e) => {
                error!("Failed to save assistant message: {}", e);
                let _ = tx
                    .send(ChatStreamEvent::Error {
                        message: format!("Failed to save response: {}", e),
                        code: "SAVE_FAILED".to_string(),
                    })
                    .await;
            }
        }
    });

    // 7. Convert channel to SSE stream
    let sse_stream = ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok(Event::default().data(json))
    });

    Ok(Sse::new(sse_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
