//! Non-streaming chat completion handler.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use tracing::{debug, error, info, warn};

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::OptionalAuth;
use crate::handlers::query::{resolve_chunk_file_paths, resolve_query_workspace};
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::services::{
    build_message_context_from_engine, ensure_debug_granularity_allowed,
    execute_sota_query_with_auth_fallback, resolve_workspace_query_resources,
};
use crate::state::AppState;
use edgequake_core::types::{
    CreateConversationRequest, CreateMessageRequest, MessageRole, UpdateMessageRequest,
};
use edgequake_query::QueryRequest as EngineQueryRequest;

use super::{
    build_sources, enrich_query_with_language, parse_mode, parse_query_mode, ChatCompletionRequest,
    ChatCompletionResponse,
};

/// Execute a non-streaming chat completion.
///
/// Creates conversation if needed, saves user message, generates response,
/// and saves assistant message - all in one atomic operation.
#[utoipa::path(
    post,
    path = "/api/v1/chat/completions",
    tag = "Chat",
    request_body = ChatCompletionRequest,
    responses(
        (status = 200, description = "Chat completion successful", body = ChatCompletionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn chat_completion(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    OptionalAuth(auth_user): OptionalAuth,
    Json(request): Json<ChatCompletionRequest>,
) -> ApiResult<Json<ChatCompletionResponse>> {
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
        "Processing chat completion"
    );

    // Fail closed when an explicit workspace header is invalid (same as /query).
    let workspace = resolve_query_workspace(&state, tenant_ctx.workspace_id.as_deref()).await?;
    let workspace_id = workspace.as_ref().map(|ws| ws.workspace_id);

    let mode = parse_mode(&request.mode);
    let query_mode = parse_query_mode(&request.mode);

    // FEAT0505: Track whether this is a new conversation for auto-title generation
    let is_new_conversation = request.conversation_id.is_none();

    // 1. Get or create conversation
    let conversation_id = if let Some(id) = request.conversation_id {
        // Verify conversation exists and belongs to user
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
        // Create new conversation
        let conv = state
            .conversation_service
            .create_conversation(
                tenant_id,
                user_id,
                workspace_id,
                CreateConversationRequest {
                    title: None, // Will be auto-generated from first message
                    mode: Some(mode),
                    folder_id: None,
                },
            )
            .await?;

        info!(conversation_id = %conv.conversation_id, "Created new conversation");
        conv.conversation_id
    };

    // SPEC-124: durable conversation_id → Langfuse session / GenAI conversation.
    let langfuse_id = crate::handlers::query::langfuse_query_identity(
        &state,
        Some(&conversation_id.to_string()),
        Some(&user_id.to_string()),
        Some(&tenant_id.to_string()),
        workspace.as_ref(),
    )
    .await;
    let _langfuse_identity = edgequake_observability::stamp_query_langfuse_identity(langfuse_id);

    // 2. Save user message
    let user_message = state
        .conversation_service
        .create_message(
            conversation_id,
            CreateMessageRequest {
                content: request.message.clone(),
                role: MessageRole::User,
                parent_id: request.parent_id,
                stream: false,
            },
        )
        .await?;

    debug!(message_id = %user_message.message_id, "Saved user message");

    // Multi-turn chatbot memory: load prior turns, apply shared history cut.
    let conversation_history = super::history::load_recent_conversation_history(
        state.conversation_service.as_ref(),
        conversation_id,
        user_message.message_id,
    )
    .await?;

    // 3. Build and execute query using SOTA engine (LightRAG-style)
    // OODA-231: Use workspace's tenant_id for graph queries, not header tenant_id.
    // WHY: Header tenant_id is for authentication (random UUID from frontend).
    // But the graph data was ingested with the workspace's actual tenant_id.
    // Using header tenant_id causes 0 results because of tenant_id mismatch.
    let enriched_query = enrich_query_with_language(&request.message, &request.language);
    let mut engine_request = EngineQueryRequest::new(&enriched_query)
        .with_mode(query_mode)
        .with_conversation_history(conversation_history);

    // SPEC-004: Thread system prompt extension if provided
    if let Some(ref system_prompt) = request.system_prompt {
        engine_request = engine_request.with_system_prompt(system_prompt);
    }

    let data_tenant_id = workspace
        .as_ref()
        .map(|ws| ws.tenant_id.to_string())
        .unwrap_or_else(|| tenant_id.to_string());
    engine_request = engine_request.with_tenant_id(data_tenant_id.clone());
    if let Some(ref ws_id) = workspace_id {
        engine_request = engine_request.with_workspace_id(ws_id.to_string());
    }

    // SPEC-005 + SPEC-146: Resolve document filter ∩ allow-set for RAG scope
    let mut client_filter_ids = None;
    if let Some(ref filter) = request.document_filter {
        let ws_id_str = workspace_id.as_ref().map(|id| id.to_string());
        let tenant_filter = Some(data_tenant_id.clone());
        if let Some(allowed_ids) =
            crate::handlers::query::document_filter_resolver::resolve_document_filter(
                state.storage.kv_storage.as_ref(),
                filter,
                &tenant_filter,
                &ws_id_str,
            )
            .await?
        {
            client_filter_ids = Some(allowed_ids);
        }
    }
    let (allowed_ids, authz_ctx, allow_set) =
        crate::services::spec146_authz::resolve_query_allowed_document_ids(
            &state,
            &tenant_ctx,
            Some(&user_id.to_string()),
            client_filter_ids,
        )
        .await?;

    // G-146-7 / LAW-146-7: empty allow-set → SSOT answer (no LLM), same as /query.
    if crate::services::spec146_authz::is_empty_allow_set(&allow_set) {
        let answer = edgequake_authz::ZERO_AUTHZ_ANSWER.to_string();
        if !super::conversation_guard::conversation_exists(&state, conversation_id).await? {
            return Err(ApiError::NotFound(format!(
                "Conversation {} no longer exists",
                conversation_id
            )));
        }
        let assistant_message = state
            .conversation_service
            .create_message(
                conversation_id,
                CreateMessageRequest {
                    content: answer.clone(),
                    role: MessageRole::Assistant,
                    parent_id: Some(user_message.message_id),
                    stream: false,
                },
            )
            .await?;
        return Ok(Json(ChatCompletionResponse {
            conversation_id,
            user_message_id: user_message.message_id,
            assistant_message_id: assistant_message.message_id,
            content: answer,
            mode: request.mode.clone().unwrap_or_else(|| "hybrid".into()),
            sources: vec![],
            stats: crate::handlers::query_types::QueryStats::default(),
            tokens_used: 0,
            duration_ms: 0,
            llm_provider: None,
            llm_model: None,
        }));
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

    // FEAT0203: Forward image attachments to the query engine for vision queries.
    // SPEC-083 C-25: materialize data: URL images before Anthropic (crates.io llm bug).
    if let Some(ref images) = request.images {
        let mut image_data: Vec<edgequake_llm::traits::ImageData> = images
            .iter()
            .map(|i| {
                if i.mime_type.eq_ignore_ascii_case("url") {
                    edgequake_llm::traits::ImageData::from_url(&i.data)
                } else {
                    edgequake_llm::traits::ImageData::new(&i.data, &i.mime_type)
                }
            })
            .collect();
        // Prefer data: → base64 for Anthropic compat; leave https URLs for OpenAI.
        image_data = image_data
            .into_iter()
            .map(|img| edgequake_pipeline::materialize_image_for_anthropic(&img).unwrap_or(img))
            .collect();
        if !image_data.is_empty() {
            engine_request = engine_request.with_images(image_data);
        }
    }

    // SPEC-032 + OADA-227: Unified provider resolution with safety limits
    // Priority order:
    //   1. Request-specified provider/model (explicit user selection)
    //   2. Workspace-configured provider/model (workspace settings)
    //   3. Server default (engine_impl's default provider)
    // Supports both formats:
    //   - Legacy format: provider="provider/model" (e.g., "ollama/gemma3:12b")
    //   - New format: provider="provider", model="model_name"
    let resolver = WorkspaceProviderResolver::from_app_state(&state);
    let llm_request =
        LlmResolutionRequest::from_provider_string(request.provider.clone(), request.model.clone());

    let (llm_override, used_provider, used_model) = match resolver
        .resolve_llm_provider_for_workspace(workspace.as_ref(), &llm_request)
        .await
    {
        Ok(Some(resolved)) => {
            debug!(
                provider = %resolved.provider_name,
                model = %resolved.model_name,
                source = ?resolved.source,
                "Resolved LLM provider (non-streaming) [QUERY]"
            );
            (
                Some(resolved.provider),
                Some(resolved.provider_name),
                Some(resolved.model_name),
            )
        }
        Ok(None) => {
            // No provider resolved - will use server default
            debug!("Using server default LLM provider (non-streaming)");
            (None, None, None)
        }
        Err(e) => {
            // Explicit provider request failed - return error to user
            // OODA-234: Unified error conversion via From<ProviderResolutionError>
            error!(error = %e, "Failed to resolve LLM provider (non-streaming)");
            return Err(ApiError::from(e));
        }
    };

    // FEAT0203: When images are attached, prefer the vision-capable LLM provider.
    // WHY: Some models (e.g. mistral-small-latest) silently drop image content.
    // The vision provider (e.g. pixtral-large-latest) is used instead when available.
    // A request-level provider override takes precedence over the server-default vision provider.
    let (llm_override, used_provider, used_model) = if llm_override.is_none()
        && engine_request
            .images
            .as_ref()
            .is_some_and(|imgs| !imgs.is_empty())
    {
        if let Some(ref vision_provider) = state.query.vision_llm_provider {
            debug!("Using vision LLM provider for image query (FEAT0203)");
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
        .unwrap_or_else(|| state.query.llm_provider.as_ref());
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

    // OADA-228: Resolve workspace-specific embedding/vector for query execution.
    let workspace_id_str = workspace_id.as_ref().map(|id| id.to_string());
    let resources = resolve_workspace_query_resources(&state, workspace_id_str.as_deref()).await?;

    let result = execute_sota_query_with_auth_fallback(
        &state,
        engine_request,
        resources,
        llm_override.clone(),
    )
    .await?;

    // SPEC-124: root observation I/O (query turn).
    edgequake_observability::record_query_root_io(&request.message, &result.answer);

    // 4. Build sources and resolve document names for chunk sources
    let mut sources = build_sources(&result.context, request.content_granularity);
    resolve_chunk_file_paths(state.storage.kv_storage.as_ref(), &mut sources).await;
    // SPEC-142: persist verified links (document name + page), not raw [N].
    let verified = crate::services::verified_citations::verified_answer(&result.answer, &sources);
    let context = build_message_context_from_engine(&result.context, &sources);

    if !super::conversation_guard::conversation_exists(&state, conversation_id).await? {
        return Err(ApiError::NotFound(format!(
            "Conversation {} no longer exists",
            conversation_id
        )));
    }

    // 5. Save assistant message
    let assistant_message = state
        .conversation_service
        .create_message(
            conversation_id,
            CreateMessageRequest {
                content: verified.clone(),
                role: MessageRole::Assistant,
                parent_id: Some(user_message.message_id),
                stream: false,
            },
        )
        .await?;

    // 6. Update assistant message with metadata
    if let Err(e) = state
        .conversation_service
        .update_message(
            assistant_message.message_id,
            UpdateMessageRequest {
                content: None,
                mode: Some(mode),
                tokens_used: Some(result.stats.generated_tokens as i32),
                duration_ms: Some(result.stats.total_time_ms as i32),
                thinking_time_ms: None,
                context: Some(context),
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
        user_message_id = %user_message.message_id,
        assistant_message_id = %assistant_message.message_id,
        "Chat completion successful"
    );

    // FEAT0505: Auto-generate conversation title for new conversations (fire-and-forget)
    if is_new_conversation {
        let title_llm = llm_override.unwrap_or_else(|| state.query.llm_provider.clone());
        let title_conv_service = state.conversation_service.clone();
        let title_conv_id = conversation_id;
        let title_first_msg = request.message.clone();
        let title_tenant_id = tenant_id;
        let title_user_id = user_id;

        tokio::spawn(async move {
            let title =
                crate::handlers::title_generator::generate_title(title_llm, &title_first_msg).await;

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
                        "Auto-generated conversation title (non-streaming)"
                    );
                }
                Err(e) => {
                    warn!(
                        conversation_id = %title_conv_id,
                        error = %e,
                        "Failed to update conversation title (non-streaming)"
                    );
                }
            }
        });
    }

    Ok(Json(ChatCompletionResponse {
        conversation_id,
        user_message_id: user_message.message_id,
        assistant_message_id: assistant_message.message_id,
        content: verified,
        mode: result.mode.to_string(),
        sources,
        stats: crate::services::query_stats_mapper::from_engine_stats(
            &result.stats,
            &result.context,
            used_provider.clone(),
            used_model.clone(),
        ),
        tokens_used: result.stats.generated_tokens as u32,
        duration_ms: result.stats.total_time_ms,
        // SPEC-032: Provider lineage tracking
        llm_provider: used_provider,
        llm_model: used_model,
    }))
}
