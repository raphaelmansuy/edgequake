//! Streaming chat completion handler (SSE).

use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::response::sse::Event;
use axum::response::Response;
use axum::Json;
use edgequake_observability::ErrorEvent;
use futures::stream::StreamExt;
use serde_json::json;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::OptionalAuth;
use crate::handlers::query::{resolve_chunk_file_paths, resolve_query_workspace};
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::services::{
    build_message_context_from_engine,
    context_bundle_mapper::{map_query_context_to_subgraph, MappingOptions},
    ensure_debug_granularity_allowed, execute_sota_query_stream_with_auth_fallback,
    resolve_workspace_query_resources,
};
use crate::state::AppState;
use crate::streaming::{live_sse, StreamAccumulator};
use edgequake_core::types::{
    CreateConversationRequest, CreateMessageRequest, MessageContext, MessageRole,
    UpdateMessageRequest,
};
use edgequake_observability::RequestContext;
use edgequake_query::QueryRequest as EngineQueryRequest;

use super::{
    build_sources, enrich_query_with_language, parse_mode, parse_query_mode, ChatCompletionRequest,
    ChatStreamEvent,
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
        (status = 200, description = "Streaming chat SSE (ChatStreamEvent payloads)",
            content(
                (ChatStreamEvent = "text/event-stream")
            )
        ),
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
    OptionalAuth(auth_user): OptionalAuth,
    Extension(req_ctx): Extension<RequestContext>,
    Json(request): Json<ChatCompletionRequest>,
) -> ApiResult<Response> {
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
    ensure_debug_granularity_allowed(
        request.content_granularity,
        auth_user.as_ref().map(|u| u.role.clone()),
    )?;

    let identity =
        super::super::postgres_user_bootstrap::resolve_conversation_identity(&state, &tenant_ctx)
            .await?;
    let tenant_id = identity.tenant_id;
    let user_id = identity.user_id;

    debug!(
        tenant_id = %tenant_id,
        user_id = %user_id,
        conversation_id = ?request.conversation_id,
        "Processing streaming chat completion"
    );

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

    // SPEC-124: stamp HTTP/chat_stream span; re-bind inside spawn for child spans.
    let langfuse_id = crate::handlers::query::langfuse_query_identity(
        &state,
        Some(&conversation_id.to_string()),
        Some(&user_id.to_string()),
        Some(&tenant_id.to_string()),
        workspace.as_ref(),
    )
    .await;
    let _langfuse_identity_outer =
        edgequake_observability::stamp_query_langfuse_identity(langfuse_id.clone());
    let langfuse_id_spawn = langfuse_id;

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

    // Multi-turn chatbot memory: load prior turns, apply shared history cut.
    // Done before spawn so both stream task and tests share one policy (DRY).
    let conversation_history = super::history::load_recent_conversation_history(
        state.conversation_service.as_ref(),
        conversation_id,
        user_message.message_id,
    )
    .await?;

    // 3. Create channel for SSE events
    let (tx, rx) = mpsc::channel::<ChatStreamEvent>(100);

    // 4. Clone state for async task
    let state_clone = state.clone();
    let tenant_ctx_clone = tenant_ctx.clone();
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
    let stream_content_granularity = request.content_granularity;
    let conversation_history_for_engine = conversation_history;

    // 5. Send initial conversation event
    let initial_event = ChatStreamEvent::Conversation {
        conversation_id,
        user_message_id,
    };

    // 6. Spawn background task for LLM streaming
    let stream_request_id_spawn = stream_request_id.clone();
    tokio::spawn(async move {
        edgequake_observability::bind_langfuse_trace_identity_async(
            langfuse_id_spawn,
            async move {
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
        // SPEC-142: keep sources for verified answer rewrite at Done / persist.
        // Initial empty is overwritten when Sources arrive; may stay empty if stream ends early.
        #[allow(unused_assignments)]
        let mut sources_for_verify: Vec<crate::handlers::query_types::SourceReference> = Vec::new();

        // Build query request
        // OODA-231: Use workspace's tenant_id for graph queries, not header tenant_id.
        // WHY: Header tenant_id is for authentication (random UUID from frontend).
        // But the graph data was ingested with the workspace's actual tenant_id.
        // Using header tenant_id causes 0 results because of tenant_id mismatch.
        let enriched_query = enrich_query_with_language(&message_content, &request_language);
        let mut engine_request = EngineQueryRequest::new(&enriched_query)
            .with_mode(query_mode)
            .with_conversation_history(conversation_history_for_engine);

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

        // SPEC-005 + SPEC-146: Resolve document filter ∩ allow-set for RAG scope
        let mut client_filter_ids = None;
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
                    client_filter_ids = Some(allowed_ids);
                }
                Ok(None) => {}
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
        match crate::services::spec146_authz::resolve_query_allowed_document_ids(
            &state_clone,
            &tenant_ctx_clone,
            Some(&user_id.to_string()),
            client_filter_ids,
        )
        .await
        {
            Ok((allowed_ids, authz_ctx, allow_set)) => {
                // G-146-7: empty allow → SSOT token stream, no LLM.
                if crate::services::spec146_authz::is_empty_allow_set(&allow_set) {
                    let answer = edgequake_authz::ZERO_AUTHZ_ANSWER.to_string();
                    let assistant_message = match state_clone
                        .conversation_service
                        .create_message(
                            conversation_id,
                            edgequake_core::types::CreateMessageRequest {
                                content: answer.clone(),
                                role: edgequake_core::types::MessageRole::Assistant,
                                parent_id: Some(user_message_id),
                                stream: true,
                            },
                        )
                        .await
                    {
                        Ok(m) => m,
                        Err(e) => {
                            let _ = tx
                                .send(ChatStreamEvent::Error {
                                    message: e.to_string(),
                                    code: "ASSISTANT_PERSIST_ERROR".to_string(),
                                })
                                .await;
                            return;
                        }
                    };
                    let _ = tx
                        .send(ChatStreamEvent::Token {
                            content: answer.clone(),
                        })
                        .await;
                    let _ = tx
                        .send(ChatStreamEvent::Done {
                            assistant_message_id: assistant_message.message_id,
                            tokens_used: 0,
                            duration_ms: 0,
                            llm_provider: None,
                            llm_model: None,
                            answer: Some(answer),
                        })
                        .await;
                    return;
                }
                if let Some(ids) = allowed_ids {
                    engine_request = engine_request.with_allowed_document_ids(ids);
                }
                if let (Some(ctx), Some(allow)) = (authz_ctx, allow_set) {
                    engine_request = engine_request.with_authz_cache_scope(
                        format!("{}:{}", ctx.principal.kind_str(), ctx.principal.id_str()),
                        ctx.policy_generation,
                        allow.fingerprint(),
                    );
                }
            }
            Err(e) => {
                let msg = e.to_string();
                let _ = tx
                    .send(ChatStreamEvent::Error {
                        message: msg,
                        code: "ABAC_ALLOW_SET_ERROR".to_string(),
                    })
                    .await;
                return;
            }
        }

        // FEAT0203: Forward image attachments to the query engine for vision queries.
        if let Some(ref images) = request.images {
            // SPEC-083 C-25: materialize data: URL images before Anthropic path.
            let image_data: Vec<edgequake_llm::traits::ImageData> = images
                .iter()
                .map(|i| {
                    let img = if i.mime_type.eq_ignore_ascii_case("url") {
                        edgequake_llm::traits::ImageData::from_url(&i.data)
                    } else {
                        edgequake_llm::traits::ImageData::new(&i.data, &i.mime_type)
                    };
                    edgequake_pipeline::materialize_image_for_anthropic(&img).unwrap_or(img)
                })
                .collect();
            if !image_data.is_empty() {
                engine_request = engine_request.with_images(image_data);
            }
        }

        // SPEC-032 + OODA-227: Unified provider resolution with safety limits (streaming)
        // Priority order:
        //   1. Request-specified provider/model (explicit user selection)
        //   2. Workspace-configured provider/model (workspace settings)
        //   3. Server default (engine_impl's default provider)
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
            .resolve_llm_provider_for_workspace(workspace_clone.as_ref(), &llm_request)
            .await
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
                    Some(vision_provider.name().to_string()),
                    Some(vision_provider.model().to_string()),
                )
            } else {
                (llm_override, used_provider, used_model)
            }
        } else {
            (llm_override, used_provider, used_model)
        };

        // Honesty: always record effective provider/model (incl. server-default path).
        let effective_llm = llm_override
            .as_ref()
            .map(|p| p.as_ref())
            .unwrap_or_else(|| state_clone.query.llm_provider.as_ref());
        let (used_provider, used_model) = {
            let (p, m) =
                super::coalesce_effective_llm_lineage(used_provider, used_model, effective_llm);
            (Some(p), Some(m))
        };

        let provider_for_effort = used_provider
            .as_deref()
            .or_else(|| workspace.as_ref().map(|w| w.llm_provider.as_str()))
            .unwrap_or("openai");
        let model_for_effort = used_model
            .as_deref()
            .or_else(|| workspace.as_ref().map(|w| w.llm_model.as_str()))
            .unwrap_or("");
        if let Some(effort) = crate::services::resolve_query_reasoning_effort(
            workspace.as_ref(),
            provider_for_effort,
            model_for_effort,
            request.reasoning_effort.as_deref(),
            None,
        ) {
            engine_request = engine_request.with_reasoning_effort(effort);
        }

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
                let mut sources = build_sources(&context, stream_content_granularity);

                // Resolve document names for chunk sources
                resolve_chunk_file_paths(state_clone.storage.kv_storage.as_ref(), &mut sources)
                    .await;

                // Save message context for later persistence
                saved_message_context = Some(build_message_context_from_engine(&context, &sources));
                sources_for_verify = sources.clone();

                let retrieval_elapsed_ms = retrieval_start.elapsed().as_millis() as u64;

                // SPEC-083 X-22: emit Thinking before Context (keep variant live).
                let thinking = ChatStreamEvent::Thinking {
                    content: format!(
                        "Retrieved context via {used_mode} ({} sources in {retrieval_elapsed_ms}ms)",
                        sources.len()
                    ),
                };
                if tx.send(thinking).await.is_err() {
                    ErrorEvent::log_stream_disconnect(
                        &stream_request_id_spawn,
                        "chat_stream",
                        "thinking_event",
                    );
                    return;
                }

                if !sources.is_empty() {
                    let subgraph = Some(map_query_context_to_subgraph(
                        &context,
                        &MappingOptions {
                            granularity: stream_content_granularity,
                            include_lineage: true,
                            include_documents: false,
                            include_agent_hints: false,
                            include_subgraph: true,
                            rerank_top_k: None,
                            reranked: false,
                        },
                    ));
                    let context_event = ChatStreamEvent::Context {
                        sources: sources.clone(),
                        query_mode: Some(used_mode.to_string()),
                        retrieval_time_ms: Some(retrieval_elapsed_ms),
                        subgraph,
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
        // SPEC-142: persist + Done carry verified document+page links.
        let verified = crate::services::verified_citations::verified_answer(
            &full_content,
            &sources_for_verify,
        );

        // SPEC-124: root observation I/O after stream completes.
        edgequake_observability::record_query_root_io(&message_content, &verified);

        // SPEC-040 #259: conversation may be deleted while the LLM stream runs.
        if !super::conversation_guard::conversation_exists(&state_clone, conversation_id)
            .await
            .unwrap_or(false)
        {
            let _ = tx
                .send(ChatStreamEvent::Error {
                    message: "Conversation no longer exists".to_string(),
                    code: "CONVERSATION_GONE".to_string(),
                })
                .await;
            return;
        }

        // 7. Save assistant message (AFTER streaming completes)
        match state_clone
            .conversation_service
            .create_message(
                conversation_id,
                CreateMessageRequest {
                    content: verified.clone(),
                    role: MessageRole::Assistant,
                    parent_id: Some(user_message_id),
                    stream: true,
                },
            )
            .await
        {
            Ok(assistant_message) => {
                // Update with metadata AND context for source citations
                if let Err(e) = state_clone
                    .conversation_service
                    .update_message(
                        assistant_message.message_id,
                        UpdateMessageRequest {
                            content: None,
                            mode: Some(mode),
                            tokens_used: Some(tokens_used as i32),
                            duration_ms: Some(duration_ms as i32),
                            thinking_time_ms: None,
                            context: saved_message_context, // Save context for source citations!
                            is_error: None,
                            llm_provider: used_provider.clone(),
                            llm_model: used_model.clone(),
                        },
                    )
                    .await
                {
                    error!(
                        conversation_id = %conversation_id,
                        assistant_message_id = %assistant_message.message_id,
                        error = %e,
                        "Failed to persist assistant message metadata (mode/lineage/tokens)"
                    );
                }

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
                        answer: Some(verified),
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
                let msg = if e.to_string().contains("messages_conversation_id_fkey") {
                    "Conversation no longer exists".to_string()
                } else {
                    format!("Failed to save response: {e}")
                };
                let code = if msg.contains("Conversation no longer exists") {
                    "CONVERSATION_GONE".to_string()
                } else {
                    "SAVE_FAILED".to_string()
                };
                ErrorEvent::log_stream_error(
                    &stream_request_id_spawn,
                    "chat_stream",
                    code.as_str(),
                    &msg,
                    json!({
                        "phase": "save_assistant_message",
                        "conversation_id": conversation_id.to_string(),
                    }),
                );
                let _ = tx.send(ChatStreamEvent::Error { message: msg, code }).await;
            }
        }
            },
        )
        .await;
    });

    // 7. Convert channel to SSE stream
    let sse_stream = ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok(Event::default().data(json))
    });

    Ok(live_sse(sse_stream))
}
