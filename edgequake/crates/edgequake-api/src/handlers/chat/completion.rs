//! Non-streaming chat completion handler.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

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

    let tenant_id = tenant_ctx
        .tenant_id
        .ok_or(ApiError::unauthorized())?
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("Invalid tenant ID".to_string()))?;
    let client_user_id = tenant_ctx
        .user_id
        .ok_or(ApiError::unauthorized())?
        .parse::<Uuid>()
        .map_err(|_| ApiError::BadRequest("Invalid user ID".to_string()))?;

    let user_id = super::super::postgres_user_bootstrap::ensure_postgres_user_exists(
        &state,
        tenant_id,
        client_user_id,
    )
    .await?;

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

    // 3. Build and execute query using SOTA engine (LightRAG-style)
    // OODA-231: Use workspace's tenant_id for graph queries, not header tenant_id.
    // WHY: Header tenant_id is for authentication (random UUID from frontend).
    // But the graph data was ingested with the workspace's actual tenant_id.
    // Using header tenant_id causes 0 results because of tenant_id mismatch.
    let enriched_query = enrich_query_with_language(&request.message, &request.language);
    let mut engine_request = EngineQueryRequest::new(&enriched_query).with_mode(query_mode);

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

    // SPEC-005: Resolve document filter → allowed_document_ids for RAG context scoping
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
            engine_request = engine_request.with_allowed_document_ids(allowed_ids);
        }
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

    let (llm_override, used_provider, used_model) =
        match resolver.resolve_llm_provider_with_workspace(workspace.as_ref(), &llm_request) {
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
                Some("vision".to_string()),
                Some("vision-model".to_string()),
            )
        } else {
            (llm_override, used_provider, used_model)
        }
    } else {
        (llm_override, used_provider, used_model)
    };

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

    // 4. Build sources and resolve document names for chunk sources
    let mut sources = build_sources(&result.context, request.content_granularity);
    resolve_chunk_file_paths(state.storage.kv_storage.as_ref(), &mut sources).await;
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
                content: result.answer.clone(),
                role: MessageRole::Assistant,
                parent_id: Some(user_message.message_id),
                stream: false,
            },
        )
        .await?;

    // 6. Update assistant message with metadata
    state
        .conversation_service
        .update_message(
            assistant_message.message_id,
            UpdateMessageRequest {
                content: None,
                tokens_used: Some(result.stats.generated_tokens as i32),
                duration_ms: Some(result.stats.total_time_ms as i32),
                thinking_time_ms: None,
                context: Some(context),
                is_error: None,
            },
        )
        .await?;

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
        content: result.answer,
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
