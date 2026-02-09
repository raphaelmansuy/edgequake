//! Unified chat completions handler.
//!
//! This module provides a unified endpoint for chat interactions that handles
//! conversation creation, message persistence, and LLM streaming in a single
//! atomic operation. This is the preferred API for client applications.
//!
//! # WHY: Query Provider Resolution vs Pipeline Provider Resolution
//!
//! The chat handler resolves providers PER-REQUEST. This is SEPARATE from the
//! pipeline's document-extraction providers (see processor.rs). Users often see
//! Ollama logs interleaved with their OpenAI chat query logs and assume their
//! query used Ollama. In reality, Ollama logs come from background pipeline
//! tasks running concurrently.
//!
//! ```text
//!  ┌──────────────────────────────────────────────────────────────────────┐
//!  │  QUERY PROVIDER RESOLUTION (this module)                            │
//!  │                                                                      │
//!  │  UI sends: { provider: "openai", model: "gpt-5-nano" }             │
//!  │       │                                                              │
//!  │       ▼                                                              │
//!  │  WorkspaceProviderResolver::resolve_llm_provider_with_workspace      │
//!  │       │                                                              │
//!  │       ├── Has request.provider + request.model?                      │
//!  │       │   └── YES ──► create_safe_llm_provider() → source=Request   │
//!  │       │                                                              │
//!  │       ├── Has workspace.llm_provider?                                │
//!  │       │   └── YES ──► create_safe_llm_provider() → source=Workspace │
//!  │       │                                                              │
//!  │       └── Neither? ──► None → use sota_engine's default              │
//!  │                                                                      │
//!  │  Result: llm_override = Arc<dyn LLMProvider>                        │
//!  │  Used for: answer generation + keyword extraction (query-time only)  │
//!  └──────────────────────────────────────────────────────────────────────┘
//!
//!  ┌──────────────────────────────────────────────────────────────────────┐
//!  │  PIPELINE PROVIDER (processor.rs - background task, NOT this module) │
//!  │                                                                      │
//!  │  Worker picks up document task with workspace_id                     │
//!  │       │                                                              │
//!  │       ▼                                                              │
//!  │  get_workspace_pipeline_strict(workspace_id)                        │
//!  │       │                                                              │
//!  │       ├── Creates llm + embedding from workspace DB config           │
//!  │       │   └── SUCCESS ──► workspace-specific Pipeline               │
//!  │       │                                                              │
//!  │       └── FAILURE ──► Task fails (strict mode) or falls back to     │
//!  │                       server default pipeline (Ollama from env)      │
//!  │                                                                      │
//!  │  Result: Pipeline with LLMExtractor + EmbeddingProvider             │
//!  │  Used for: entity extraction from documents (background ingestion)   │
//!  └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Implements
//!
//! - **FEAT0501**: Unified chat endpoint with streaming SSE responses
//! - **FEAT0502**: Server-initiated message persistence
//! - **FEAT0503**: Automatic conversation creation and management
//! - **FEAT0504**: Multi-mode query support (local/global/hybrid/naive)
//!
//! ## Use Cases
//!
//! - **UC2101**: User sends a chat message and receives streamed response
//! - **UC2102**: System creates conversation automatically on first message
//! - **UC2103**: User views source citations in chat response
//! - **UC2104**: System persists assistant response after streaming completes
//!
//! ## Enforces
//!
//! - **BR0501**: All messages must be persisted with proper roles
//! - **BR0502**: Streaming must accumulate tokens before persistence
//! - **BR0503**: Source tracking must include document IDs for citations
//! - **BR0504**: Query mode defaults to hybrid when not specified
//!
//! Key benefits:
//! - Server-initiated persistence (no client-side message saving)
//! - Transactional integrity for message storage
//! - Single API call instead of multiple round-trips
//! - Automatic conversation management

use axum::{
    extract::State,
    response::sse::{Event, Sse},
    Json,
};
use futures::stream::StreamExt;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::query::{
    get_workspace_embedding_provider, get_workspace_vector_storage, QueryStats, SourceReference,
};
use crate::middleware::TenantContext;
use crate::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use crate::state::AppState;
use crate::streaming::StreamAccumulator;
use edgequake_core::types::{
    ConversationMode, CreateConversationRequest, CreateMessageRequest, MessageContext,
    MessageContextEntity, MessageContextRelationship, MessageRole, MessageSource,
    UpdateMessageRequest,
};
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};

// Re-export DTOs from chat_types module
pub use crate::handlers::chat_types::*;

// ============================================================================
// Helper Functions
// ============================================================================

fn parse_mode(mode: &Option<String>) -> ConversationMode {
    mode.as_ref()
        .and_then(|m| match m.to_lowercase().as_str() {
            "local" => Some(ConversationMode::Local),
            "global" => Some(ConversationMode::Global),
            "hybrid" => Some(ConversationMode::Hybrid),
            "naive" | "simple" => Some(ConversationMode::Naive),
            _ => None,
        })
        .unwrap_or(ConversationMode::Hybrid)
}

fn parse_query_mode(mode: &Option<String>) -> QueryMode {
    mode.as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Hybrid)
}

fn build_sources(context: &edgequake_query::QueryContext) -> Vec<SourceReference> {
    let mut sources = Vec::new();
    let mut ref_counter = 1usize;

    for chunk in &context.chunks {
        sources.push(SourceReference {
            source_type: "chunk".to_string(),
            id: chunk.id.clone(),
            score: chunk.score,
            rerank_score: None,
            snippet: Some(chunk.content.chars().take(200).collect()),
            reference_id: Some(ref_counter),
            document_id: chunk.document_id.clone(),
            file_path: None,
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            chunk_index: chunk.chunk_index,
        });
        ref_counter += 1;
    }

    for entity in &context.entities {
        sources.push(SourceReference {
            source_type: "entity".to_string(),
            id: entity.name.clone(),
            score: entity.score,
            rerank_score: None,
            snippet: Some(entity.description.chars().take(200).collect()),
            reference_id: Some(ref_counter),
            // Source tracking for citations (LightRAG parity)
            document_id: entity.source_document_id.clone(),
            file_path: entity.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
        });
        ref_counter += 1;
    }

    for rel in &context.relationships {
        sources.push(SourceReference {
            source_type: "relationship".to_string(),
            id: format!("{}->{}", rel.source, rel.target),
            score: rel.score,
            rerank_score: None,
            snippet: Some(format!(
                "{} {} {}",
                rel.source, rel.relation_type, rel.target
            )),
            reference_id: Some(ref_counter),
            // Source tracking for citations (LightRAG parity)
            document_id: rel.source_document_id.clone(),
            file_path: rel.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
        });
        ref_counter += 1;
    }

    sources
}

fn sources_to_message_context(sources: &[SourceReference]) -> MessageContext {
    MessageContext {
        sources: sources
            .iter()
            .filter(|s| s.source_type == "chunk")
            .map(|s| MessageSource {
                id: s.id.clone(),
                title: Some(s.source_type.clone()),
                content: Some(s.snippet.clone().unwrap_or_default()),
                score: s.score,
                document_id: s.document_id.clone(),
            })
            .collect(),
        entities: sources
            .iter()
            .filter(|s| s.source_type == "entity")
            .map(|s| MessageContextEntity {
                name: s.id.clone(),
                entity_type: "UNKNOWN".to_string(), // Not available in SourceReference
                description: s.snippet.clone(),
                score: s.score,
                source_document_id: s.document_id.clone(),
                source_file_path: s.file_path.clone(),
                source_chunk_ids: Vec::new(), // Not available in SourceReference
            })
            .collect(),
        relationships: sources
            .iter()
            .filter(|s| s.source_type == "relationship")
            .map(|s| {
                // Parse the relationship ID which is in "SOURCE->TARGET" format
                let parts: Vec<&str> = s.id.split("->").collect();
                let (source, target) = if parts.len() >= 2 {
                    (parts[0].trim().to_string(), parts[1].trim().to_string())
                } else {
                    (s.id.clone(), "UNKNOWN".to_string())
                };
                // Try to extract relation type from snippet ("SOURCE RELATION_TYPE TARGET")
                let relation_type = s
                    .snippet
                    .as_ref()
                    .map(|snippet| {
                        let words: Vec<&str> = snippet.split_whitespace().collect();
                        if words.len() >= 3 {
                            words[1..words.len() - 1].join("_").to_uppercase()
                        } else {
                            "RELATED_TO".to_string()
                        }
                    })
                    .unwrap_or_else(|| "RELATED_TO".to_string());

                MessageContextRelationship {
                    source,
                    target,
                    relation_type,
                    description: s.snippet.clone(),
                    score: s.score,
                    source_document_id: s.document_id.clone(),
                    source_file_path: s.file_path.clone(),
                }
            })
            .collect(),
    }
}

// ============================================================================
// Non-Streaming Chat Completion
// ============================================================================

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
    let mut engine_request = EngineQueryRequest::new(&request.message).with_mode(query_mode);

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

    // 4. Build sources and context
    let sources = build_sources(&result.context);
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

// ============================================================================
// Streaming Chat Completion
// ============================================================================

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
        let mut engine_request = EngineQueryRequest::new(&message_content).with_mode(query_mode);
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
                let sources = build_sources(&context);

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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mode() {
        assert_eq!(
            parse_mode(&Some("local".to_string())),
            ConversationMode::Local
        );
        assert_eq!(
            parse_mode(&Some("GLOBAL".to_string())),
            ConversationMode::Global
        );
        assert_eq!(
            parse_mode(&Some("hybrid".to_string())),
            ConversationMode::Hybrid
        );
        assert_eq!(
            parse_mode(&Some("naive".to_string())),
            ConversationMode::Naive
        );
        assert_eq!(
            parse_mode(&Some("simple".to_string())),
            ConversationMode::Naive
        );
        assert_eq!(parse_mode(&None), ConversationMode::Hybrid);
        assert_eq!(
            parse_mode(&Some("invalid".to_string())),
            ConversationMode::Hybrid
        );
    }

    #[test]
    fn test_chat_stream_event_serialization() {
        let event = ChatStreamEvent::Conversation {
            conversation_id: Uuid::nil(),
            user_message_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"conversation\""));

        let event = ChatStreamEvent::Token {
            content: "hello".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"token\""));
        assert!(json.contains("\"content\":\"hello\""));

        let event = ChatStreamEvent::Done {
            assistant_message_id: Uuid::nil(),
            tokens_used: 100,
            duration_ms: 500,
            llm_provider: None,
            llm_model: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"done\""));
        assert!(json.contains("\"tokens_used\":100"));
    }

    #[test]
    fn test_chat_completion_request_defaults() {
        let json = r#"{"message": "hello world"}"#;
        let request: Result<ChatCompletionRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert_eq!(req.message, "hello world");
        assert!(req.stream); // default_stream() returns true
        assert!(req.conversation_id.is_none());
    }

    #[test]
    fn test_chat_completion_request_with_conversation() {
        let json = r#"{
            "message": "test",
            "conversation_id": "00000000-0000-0000-0000-000000000001",
            "mode": "global",
            "stream": false
        }"#;
        let request: Result<ChatCompletionRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());
        let req = request.unwrap();
        assert!(!req.stream);
        assert_eq!(req.mode, Some("global".to_string()));
        assert!(req.conversation_id.is_some());
    }

    #[test]
    fn test_chat_stream_event_context() {
        let event = ChatStreamEvent::Context { sources: vec![] };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"context\""));
        assert!(json.contains("\"sources\":[]"));
    }

    #[test]
    fn test_chat_stream_event_error() {
        let event = ChatStreamEvent::Error {
            message: "Something went wrong".to_string(),
            code: "INTERNAL_ERROR".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("Something went wrong"));
        assert!(json.contains("INTERNAL_ERROR"));
    }
}
