//! Non-streaming chat completion handler.

use axum::extract::State;
use axum::Json;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::query::{
    get_workspace_embedding_provider, get_workspace_vector_storage, resolve_chunk_file_paths,
    QueryStats,
};
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::state::AppState;
use edgequake_core::types::{
    CreateConversationRequest, CreateMessageRequest, MessageRole, UpdateMessageRequest,
};
use edgequake_query::QueryRequest as EngineQueryRequest;

use super::{
    build_sources, enrich_query_with_language, parse_mode, parse_query_mode,
    sources_to_message_context, ChatCompletionRequest, ChatCompletionResponse,
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
    Json(request): Json<ChatCompletionRequest>,
) -> ApiResult<Json<ChatCompletionResponse>> {
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
        "Processing chat completion"
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
                warn!(workspace_id = %ws_id, "Workspace not found, ignoring stale workspace_id");
                (None, None)
            }
            Err(e) => {
                warn!(workspace_id = %ws_id, error = %e, "Failed to validate workspace, ignoring");
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

    // 1. Get or create conversation
    let conversation_id = if let Some(id) = request.conversation_id {
        // Verify conversation exists and belongs to user
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
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create conversation: {}", e)))?;

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
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to save user message: {}", e)))?;

    debug!(message_id = %user_message.message_id, "Saved user message");

    // 3. Build and execute query using SOTA engine (LightRAG-style)
    // OODA-231: Use workspace's tenant_id for graph queries, not header tenant_id.
    // WHY: Header tenant_id is for authentication (random UUID from frontend).
    // But the graph data was ingested with the workspace's actual tenant_id.
    // Using header tenant_id causes 0 results because of tenant_id mismatch.
    let enriched_query = enrich_query_with_language(&request.message, &request.language);
    let mut engine_request = EngineQueryRequest::new(&enriched_query).with_mode(query_mode);

    let data_tenant_id = workspace
        .as_ref()
        .map(|ws| ws.tenant_id.to_string())
        .unwrap_or_else(|| tenant_id.to_string());
    engine_request = engine_request.with_tenant_id(data_tenant_id);
    if let Some(ref ws_id) = workspace_id {
        engine_request = engine_request.with_workspace_id(ws_id.to_string());
    }

    // SPEC-032 + OODA-227: Unified provider resolution with safety limits
    // Priority order:
    //   1. Request-specified provider/model (explicit user selection)
    //   2. Workspace-configured provider/model (workspace settings)
    //   3. Server default (sota_engine's default provider)
    // Supports both formats:
    //   - Legacy format: provider="provider/model" (e.g., "ollama/gemma3:12b")
    //   - New format: provider="provider", model="model_name"
    let resolver = WorkspaceProviderResolver::new(state.workspace_service.clone());
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

    // OODA-228: Get workspace-specific embedding provider and vector storage
    // This ensures query embeddings match the dimension of stored vectors
    let workspace_id_str = workspace_id.as_ref().map(|id| id.to_string());
    let (ws_embedding_provider, ws_vector_storage) = if let Some(ref ws_id_str) = workspace_id_str {
        let embedding_result = get_workspace_embedding_provider(&state, ws_id_str).await;
        let vector_result = get_workspace_vector_storage(&state, ws_id_str).await;

        match (embedding_result, vector_result) {
            (Ok(Some(embed)), Ok(Some(vector))) => {
                debug!(
                    workspace_id = %ws_id_str,
                    "Using workspace-specific embedding provider AND vector storage for chat query"
                );
                (Some(embed), Some(vector))
            }
            (Ok(Some(embed)), Ok(None)) => {
                // Embedding provider exists but no vector storage (shouldn't happen in normal use)
                debug!(
                    workspace_id = %ws_id_str,
                    "Using workspace-specific embedding provider only for chat query"
                );
                (Some(embed), None)
            }
            (Ok(Some(_embed)), Err(e)) => {
                // OODA-228: Vector storage failed - return error, don't silently ignore
                error!(
                    workspace_id = %ws_id_str,
                    error = %e,
                    "Cannot get workspace vector storage - storage error"
                );
                return Err(ApiError::Internal(format!(
                    "Cannot query workspace: {}. Vector storage error: {}",
                    ws_id_str, e
                )));
            }
            (Ok(None), _) => {
                debug!(
                    workspace_id = %ws_id_str,
                    "No workspace-specific embedding config, using defaults"
                );
                (None, None)
            }
            (Err(e), _) => {
                // OODA-228/OODA-229: Return clear error for configuration issues
                // WHY: Silent fallback to default causes dimension mismatch because:
                // 1. Workspace was configured with provider X (e.g., OpenAI 3072 dims)
                // 2. Documents were embedded with dimension X
                // 3. Now provider X fails (e.g., missing OPENAI_API_KEY)
                // 4. If we fall back to provider Y (e.g., Ollama 768 dims), query will fail
                //    with "different vector dimensions" error from PostgreSQL
                error!(
                    workspace_id = %ws_id_str,
                    error = %e,
                    "Cannot create workspace embedding provider - configuration error"
                );

                // Return the error directly (it already has a good message from query.rs)
                return Err(e);
            }
        }
    } else {
        (None, None)
    };

    // Execute query with workspace-specific providers if available
    let result = match (&ws_embedding_provider, &ws_vector_storage) {
        (Some(embed), Some(vector)) => {
            // Full workspace isolation with optional LLM override
            state
                .sota_engine
                .query_with_full_config(
                    engine_request,
                    embed.clone(),
                    vector.clone(),
                    llm_override.clone(),
                )
                .await
                .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
        }
        (Some(embed), None) => {
            // WHY: Same fix as streaming path — use workspace embedding with server
            // default vector storage instead of dropping to query_with_embedding_provider
            // which may use a different vector storage dimension.
            warn!("[QUERY] Workspace embedding available but no vector storage - using workspace embedding with server default vector storage");
            state
                .sota_engine
                .query_with_full_config(
                    engine_request,
                    embed.clone(),
                    state.vector_storage.clone(),
                    llm_override.clone(),
                )
                .await
                .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
        }
        _ => {
            // No workspace-specific config, use default or LLM override only
            if let Some(ref llm) = llm_override {
                state
                    .sota_engine
                    .query_with_llm_provider(engine_request, llm.clone())
                    .await
                    .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
            } else {
                state
                    .sota_engine
                    .query(engine_request)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
            }
        }
    };

    // 4. Build sources and resolve document names for chunk sources
    let mut sources = build_sources(&result.context);
    resolve_chunk_file_paths(state.kv_storage.as_ref(), &mut sources).await;
    let context = sources_to_message_context(&sources);

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
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to save assistant message: {}", e)))?;

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
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to update message metadata: {}", e)))?;

    info!(
        conversation_id = %conversation_id,
        user_message_id = %user_message.message_id,
        assistant_message_id = %assistant_message.message_id,
        "Chat completion successful"
    );

    // SPEC-032 Item 18: Calculate tokens per second
    let tokens_per_second =
        if result.stats.generation_time_ms > 0 && result.stats.generated_tokens > 0 {
            Some(
                (result.stats.generated_tokens as f32) / (result.stats.generation_time_ms as f32)
                    * 1000.0,
            )
        } else {
            None
        };

    // FEAT0505: Auto-generate conversation title for new conversations (fire-and-forget)
    if is_new_conversation {
        let title_llm = llm_override.unwrap_or_else(|| state.llm_provider.clone());
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
        stats: QueryStats {
            embedding_time_ms: result.stats.embedding_time_ms,
            retrieval_time_ms: result.stats.retrieval_time_ms,
            generation_time_ms: result.stats.generation_time_ms,
            total_time_ms: result.stats.total_time_ms,
            sources_retrieved: result.context.chunks.len()
                + result.context.entities.len()
                + result.context.relationships.len(),
            rerank_time_ms: None,
            // SPEC-032 Item 18, 22: Token metrics and model lineage
            tokens_used: Some(result.stats.generated_tokens),
            tokens_per_second,
            // Clone the already-Option values (don't double-wrap)
            llm_provider: used_provider.clone(),
            llm_model: used_model.clone(),
        },
        tokens_used: result.stats.generated_tokens as u32,
        duration_ms: result.stats.total_time_ms,
        // SPEC-032: Provider lineage tracking
        llm_provider: used_provider,
        llm_model: used_model,
    }))
}
