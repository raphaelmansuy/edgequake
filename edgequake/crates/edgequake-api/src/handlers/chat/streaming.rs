//! Streaming chat completion handler (SSE).

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::response::sse::{Event, Sse};
use axum::Json;
use edgequake_observability::ErrorEvent;
use futures::stream::StreamExt;
use serde_json::json;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::query::{resolve_chunk_file_paths, resolve_query_workspace};
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::services::{
    execute_sota_query_stream_with_auth_fallback, resolve_workspace_query_resources,
};
use crate::state::AppState;
use crate::streaming::StreamAccumulator;
use edgequake_core::types::{
    CreateConversationRequest, CreateMessageRequest, MessageContext, MessageRole,
    UpdateMessageRequest,
};
use edgequake_observability::RequestContext;
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
#[tracing::instrument(
    name = "chat_stream",
    skip(state, tenant_ctx, request),
    fields(
        request_id = %req_ctx.request_id,
        query.mode = tracing::field::Empty,
        otel.name = "chat_stream",
    )
)]
pub async fn chat_completion_stream(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Extension(req_ctx): Extension<RequestContext>,
    Json(request): Json<ChatCompletionRequest>,
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>> {
    // Validate request
    if request.message.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Message cannot be empty".to_string(),
        ));
    }

    // Validate image attachments (Issue #203) — delegated to shared helper (DRY).
    if let Some(ref images) = request.images {
        super::validation::validate_image_attachments(images)?;
    }

    let tenant_id = tenant_ctx
        .tenant_id
        .ok_or(ApiError::unauthorized())?
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("Invalid tenant ID".to_string()))?;
    let user_id = tenant_ctx
        .user_id
        .ok_or(ApiError::unauthorized())?
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    debug!(
        tenant_id = %tenant_id,
        user_id = %user_id,
        conversation_id = ?request.conversation_id,
        "Processing streaming chat completion"
    );

    super::super::postgres_user_bootstrap::ensure_postgres_user_exists(&state, tenant_id, user_id)
        .await?;

    // Fail closed when an explicit workspace header is invalid (same as /query).
    let workspace = resolve_query_workspace(&state, tenant_ctx.workspace_id.as_deref()).await?;
    let workspace_id = workspace.as_ref().map(|ws| ws.workspace_id);

    let mode = parse_mode(&request.mode);
    let query_mode = parse_query_mode(&request.mode);
    tracing::Span::current().record("query.mode", format!("{mode:?}").as_str());

    // FEAT0505: Track whether this is a new conversation for auto-title generation
    let is_new_conversation = request.conversation_id.is_none();

    // 1. Get or create conversation (BEFORE streaming)
    let conversation_id = if let Some(id) = request.conversation_id {
        let conv = state
            .conversation_service
            .get_conversation(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Conversation {} not found", id)))?;

        if conv.tenant_id != tenant_id {
            return Err(ApiError::forbidden());
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
            .await?;

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
        .await?;

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
    // SPEC-004: Clone system prompt for async task
    let request_system_prompt = request.system_prompt.clone();
    // SPEC-005: Clone document filter for async task
    let request_document_filter = request.document_filter.clone();
    // FEAT0505: Clone for auto-title generation
    let first_message_for_title = request.message.clone();
    let stream_request_id = req_ctx.request_id.clone();

    // 5. Send initial conversation event
    let initial_event = ChatStreamEvent::Conversation {
        conversation_id,
        user_message_id,
    };

    // 6. Spawn background task for LLM streaming
    let stream_request_id_spawn = stream_request_id.clone();
    tokio::spawn(async move {
        // Send initial event
        if tx.send(initial_event).await.is_err() {
            ErrorEvent::log_stream_disconnect(
                &stream_request_id_spawn,
                "chat_stream",
                "initial_event",
            );
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

        // SPEC-004: Thread system prompt extension if provided
        if let Some(ref system_prompt) = request_system_prompt {
            engine_request = engine_request.with_system_prompt(system_prompt);
        }
        let data_tenant_id = workspace_clone
            .as_ref()
            .map(|ws| ws.tenant_id.to_string())
            .unwrap_or_else(|| tenant_id.to_string());
        engine_request = engine_request.with_tenant_id(data_tenant_id.clone());
        if let Some(ref ws_id) = workspace_id {
            engine_request = engine_request.with_workspace_id(ws_id.to_string());
        }

        // SPEC-005: Resolve document filter → allowed_document_ids for RAG context scoping
        if let Some(ref filter) = request_document_filter {
            let ws_id_str = workspace_id.as_ref().map(|id| id.to_string());
            let tenant_filter = Some(data_tenant_id.clone());
            match crate::handlers::query::document_filter_resolver::resolve_document_filter(
                state_clone.storage.kv_storage.as_ref(),
                filter,
                &tenant_filter,
                &ws_id_str,
            )
            .await
            {
                Ok(Some(allowed_ids)) => {
                    engine_request = engine_request.with_allowed_document_ids(allowed_ids);
                }
                Ok(None) => {} // No filter constraints
                Err(e) => {
                    let msg = format!("Document filter resolution failed: {e}");
                    ErrorEvent::log_stream_error(
                        &stream_request_id_spawn,
                        "chat_stream",
                        "DOCUMENT_FILTER_ERROR",
                        &msg,
                        json!({ "phase": "document_filter_resolve" }),
                    );
                    let _ = tx
                        .send(ChatStreamEvent::Error {
                            message: msg,
                            code: "DOCUMENT_FILTER_ERROR".to_string(),
                        })
                        .await;
                    return;
                }
            }
        }

        // FEAT0203: Forward image attachments to the query engine for vision queries.
        if let Some(ref images) = request.images {
            let image_data: Vec<edgequake_llm::traits::ImageData> = images
                .iter()
                .map(|i| edgequake_llm::traits::ImageData::new(&i.data, &i.mime_type))
                .collect();
            if !image_data.is_empty() {
                engine_request = engine_request.with_images(image_data);
            }
        }

        // SPEC-032 + OODA-227: Unified provider resolution with safety limits (streaming)
        // Priority order:
        //   1. Request-specified provider/model (explicit user selection)
        //   2. Workspace-configured provider/model (workspace settings)
        //   3. Server default (sota_engine's default provider)
        // Supports both formats:
        //   - Legacy format: provider="provider/model" (e.g., "ollama/gemma3:12b")
        //   - New format: provider="provider", model="model_name"
        let resolver = WorkspaceProviderResolver::from_app_state(&state_clone);
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
                let error_msg = e.to_string();
                ErrorEvent::log_stream_error(
                    &stream_request_id_spawn,
                    "chat_stream",
                    "PROVIDER_CONFIG_ERROR",
                    &error_msg,
                    json!({ "phase": "provider_resolve" }),
                );
                let _ = tx
                    .send(ChatStreamEvent::Error {
                        message: error_msg,
                        code: "PROVIDER_CONFIG_ERROR".to_string(),
                    })
                    .await;
                return;
            }
        };

        // FEAT0203: When images are attached, prefer the vision-capable LLM provider.
        // WHY: Some models (e.g. mistral-small-latest) silently drop image content.
        // A request-level provider override takes precedence over the server-default vision provider.
        let (llm_override, used_provider, used_model) = if llm_override.is_none()
            && engine_request
                .images
                .as_ref()
                .is_some_and(|imgs| !imgs.is_empty())
        {
            if let Some(ref vision_provider) = state_clone.query.vision_llm_provider {
                debug!("Using vision LLM provider for image query (FEAT0203 streaming)");
                (
                    Some(Arc::clone(vision_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>),
                    Some("vision".to_string()),
                    Some("vision-model".to_string()),
                )
            } else {
                (llm_override, used_provider, used_model)
            }
        } else {
            (llm_override, used_provider, used_model)
        };

        let workspace_id_str = workspace_id.as_ref().map(|id| id.to_string());
        let resources = match resolve_workspace_query_resources(
            &state_clone,
            workspace_id_str.as_deref(),
        )
        .await
        {
            Ok(resources) => resources,
            Err(e) => {
                let msg = e.to_string();
                ErrorEvent::log_stream_error(
                    &stream_request_id_spawn,
                    "chat_stream",
                    "WORKSPACE_QUERY_CONFIG_ERROR",
                    &msg,
                    json!({
                        "phase": "workspace_resolve",
                        "workspace_id": workspace_id_str,
                    }),
                );
                let _ = tx
                    .send(ChatStreamEvent::Error {
                        message: msg,
                        code: "WORKSPACE_QUERY_CONFIG_ERROR".to_string(),
                    })
                    .await;
                return;
            }
        };

        let retrieval_start = std::time::Instant::now();

        let stream_result = execute_sota_query_stream_with_auth_fallback(
            &state_clone,
            engine_request,
            resources,
            llm_override.clone(),
        )
        .await;

        match stream_result {
            Ok((context, used_mode, mut stream)) => {
                // Send context event BEFORE streaming tokens (for source citations)
                let mut sources = build_sources(&context);

                // Resolve document names for chunk sources
                resolve_chunk_file_paths(state_clone.storage.kv_storage.as_ref(), &mut sources)
                    .await;

                // Save message context for later persistence
                saved_message_context = Some(sources_to_message_context(&sources));

                let retrieval_elapsed_ms = retrieval_start.elapsed().as_millis() as u64;

                if !sources.is_empty() {
                    let context_event = ChatStreamEvent::Context {
                        sources: sources.clone(),
                        query_mode: Some(used_mode.to_string()),
                        retrieval_time_ms: Some(retrieval_elapsed_ms),
                    };
                    if tx.send(context_event).await.is_err() {
                        ErrorEvent::log_stream_disconnect(
                            &stream_request_id_spawn,
                            "chat_stream",
                            "context_event",
                        );
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
                                ErrorEvent::log_stream_disconnect(
                                    &stream_request_id_spawn,
                                    "chat_stream",
                                    "token_stream",
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            ErrorEvent::log_stream_error(
                                &stream_request_id_spawn,
                                "chat_stream",
                                "STREAM_ERROR",
                                &msg,
                                json!({ "phase": "token_stream" }),
                            );
                            let _ = tx
                                .send(ChatStreamEvent::Error {
                                    message: msg,
                                    code: "STREAM_ERROR".to_string(),
                                })
                                .await;
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                let msg = e.to_string();
                ErrorEvent::log_stream_error(
                    &stream_request_id_spawn,
                    "chat_stream",
                    "QUERY_FAILED",
                    &msg,
                    json!({ "phase": "stream_start" }),
                );
                let _ = tx
                    .send(ChatStreamEvent::Error {
                        message: msg,
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
                        llm_override.unwrap_or_else(|| state_clone.query.llm_provider.clone());
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
                let msg = format!("Failed to save response: {e}");
                ErrorEvent::log_stream_error(
                    &stream_request_id_spawn,
                    "chat_stream",
                    "SAVE_FAILED",
                    &msg,
                    json!({
                        "phase": "save_assistant_message",
                        "conversation_id": conversation_id.to_string(),
                    }),
                );
                let _ = tx
                    .send(ChatStreamEvent::Error {
                        message: msg,
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
