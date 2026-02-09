//! Query execution handlers.
//!
//! @implements FEAT0403
//! @implements FEAT0404
//! @implements SPEC-032: Workspace-specific embedding in query process
//!
//! # Implements
//!
//! - **UC0201**: Execute Query
//! - **UC0202**: Query with Conversation History
//! - **UC0203**: Stream Query Response
//! - **FEAT0403**: Query Execution Endpoint
//! - **FEAT0404**: Query Streaming Endpoint
//! - **FEAT0007**: Multi-Mode Query Execution
//! - **FEAT0101-0106**: Query modes (naive/local/global/hybrid/mix/bypass)
//!
//! # Enforces
//!
//! - **BR0101**: Token budget must not exceed LLM context window
//! - **BR0103**: Query mode must be valid enum value
//! - **BR0105**: Empty queries are rejected
//! - **BR0201**: Tenant isolation (queries scoped to workspace)
//!
//! # Workspace-Specific Embedding (SPEC-032)
//!
//! Queries use the embedding model configured for the workspace. This allows:
//! - Different workspaces to use different embedding providers (OpenAI, Ollama, LM Studio)
//! - Dimension-specific vector search per workspace
//!
//! # Endpoints
//!
//! | Method | Path | Handler | Description |
//! |--------|------|---------|-------------|
//! | POST | `/api/v1/query` | [`execute_query`] | Execute RAG query |
//! | POST | `/api/v1/query/stream` | [`execute_query_stream`] | Stream query response |
//!
//! # Query Flow
//!
//! ```text
//! POST /api/v1/query
//!        ↓
//!   Validate query length
//!        ↓
//!   Parse mode (default: hybrid)
//!        ↓
//!   Add tenant context (BR0201)
//!        ↓
//!   Load workspace embedding config (SPEC-032)
//!        ↓
//!   Execute via SOTA engine with workspace embedding
//!        ↓
//!   Format response + sources
//! ```

use axum::{extract::State, Json};
use tracing::{debug, error, warn};

use crate::error::{ApiError, ApiResult};
use crate::middleware::TenantContext;
use crate::providers::WorkspaceProviderResolver;
use crate::state::AppState;
use crate::validation::validate_query;
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};

// Re-export DTOs for backward compatibility
pub use crate::handlers::query_types::{
    ConversationMessage, QueryRequest, QueryResponse, QueryStats, SourceReference,
    StreamQueryRequest,
};

/// Execute a RAG query with multi-mode retrieval.
///
/// # Implements
///
/// - **UC0201**: Execute Query
/// - **FEAT0007**: Multi-Mode Query Execution
/// - **FEAT0101**: Naive mode (vector search only)
/// - **FEAT0102**: Local mode (entity-centric)
/// - **FEAT0103**: Global mode (community summaries)
/// - **FEAT0104**: Hybrid mode (local + global)
/// - **FEAT0105**: Mix mode (adaptive blend)
/// - **FEAT0106**: Bypass mode (direct LLM, no RAG)
///
/// # Enforces
///
/// - **BR0101**: Token budget enforcement
/// - **BR0103**: Mode validation
/// - **BR0201**: Tenant/workspace scoping
///
/// # Returns
///
/// - `response`: LLM-generated answer
/// - `sources`: Source references with document lineage
/// - `stats`: Retrieval statistics (chunks, entities, latency)
#[utoipa::path(
    post,
    path = "/api/v1/query",
    tag = "Query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = QueryResponse),
        (status = 400, description = "Invalid query")
    )
)]
pub async fn execute_query(
    State(state): State<AppState>,
    tenant_ctx: TenantContext,
    Json(request): Json<QueryRequest>,
) -> ApiResult<Json<QueryResponse>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        query = %request.query,
        "Executing query with tenant context"
    );

    validate_query(&request.query, state.config.max_query_length)?;

    // Parse query mode
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Hybrid);

    // Build engine query request with conversation history and tenant context
    let mut engine_request = EngineQueryRequest::new(&request.query).with_mode(mode);

    // OODA-231.1: Fetch workspace to get correct tenant_id for data queries
    // WHY: Header tenant_id is for authentication (random UUID from frontend).
    // But the graph data was ingested with the workspace's actual tenant_id.
    // Using header tenant_id causes 0 results because of tenant_id mismatch.
    let workspace = if let Some(ref workspace_id) = tenant_ctx.workspace_id {
        get_workspace(&state, workspace_id).await.ok().flatten()
    } else {
        None
    };

    // Use workspace's tenant_id for data queries, fall back to header tenant_id
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

    if request.context_only {
        engine_request = engine_request.context_only();
    }

    if request.prompt_only {
        engine_request = engine_request.prompt_only();
    }

    // Add rerank settings to engine request
    engine_request = engine_request.with_rerank(request.enable_rerank);
    if let Some(top_k) = request.rerank_top_k {
        engine_request = engine_request.with_rerank_top_k(top_k);
    }

    // SPEC-032: Add LLM provider/model overrides if provided in request
    // This allows query-time override of the LLM provider and model
    if let Some(ref provider) = request.llm_provider {
        debug!(provider = %provider, "Using LLM provider override from request");
        engine_request = engine_request.with_llm_provider(provider);
    }
    if let Some(ref model) = request.llm_model {
        debug!(model = %model, "Using LLM model override from request");
        engine_request = engine_request.with_llm_model(model);
    }

    // Add conversation history if provided
    if let Some(history) = &request.conversation_history {
        let engine_history: Vec<edgequake_query::ConversationMessage> = history
            .iter()
            .map(|m| edgequake_query::ConversationMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        engine_request = engine_request.with_conversation_history(engine_history);
    }

    // SPEC-032 & SPEC-033: Get workspace-specific embedding provider AND vector storage
    // If workspace has custom embedding config, use workspace-specific resources
    let result = if let Some(ref workspace_id) = tenant_ctx.workspace_id {
        // Try to get workspace embedding and vector storage configuration
        let embedding_result = get_workspace_embedding_provider(&state, workspace_id).await;
        let vector_result = get_workspace_vector_storage(&state, workspace_id).await;

        // Check if LLM provider override is requested
        let llm_override = if let (Some(ref provider), Some(ref model)) =
            (&request.llm_provider, &request.llm_model)
        {
            debug!(provider = %provider, model = %model, "Creating LLM provider override from request");
            Some(
                edgequake_llm::ProviderFactory::create_llm_provider(provider, model).map_err(
                    |e| ApiError::Internal(format!("Failed to create LLM provider: {}", e)),
                )?,
            )
        } else {
            None
        };

        match (embedding_result, vector_result) {
            (Ok(Some(embedding_provider)), Ok(Some(vector_storage))) => {
                // Full workspace isolation: use both workspace-specific embedding and vector storage
                debug!(
                    workspace_id = %workspace_id,
                    has_llm_override = llm_override.is_some(),
                    "Using workspace-specific embedding provider AND vector storage for query"
                );
                state
                    .sota_engine
                    .query_with_full_config(
                        engine_request,
                        embedding_provider,
                        vector_storage,
                        llm_override,
                    )
                    .await
                    .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
            }
            (Ok(Some(embedding_provider)), _) => {
                // Workspace-specific embedding only
                debug!(
                    workspace_id = %workspace_id,
                    "Using workspace-specific embedding provider for query"
                );
                state
                    .sota_engine
                    .query_with_embedding_provider(engine_request, embedding_provider)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
            }
            (Ok(None), _) => {
                // No workspace-specific config, use default engine embedding
                debug!(
                    workspace_id = %workspace_id,
                    "Using default embedding provider for query"
                );
                state
                    .sota_engine
                    .query(engine_request)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
            }
            (Err(e), _) => {
                // OODA-229: Return configuration errors to the user instead of silent fallback
                // WHY: If workspace is configured for OpenAI but API key is missing, using
                // the default provider would return wrong results (different embeddings).
                // Better to fail fast with a clear error message.
                if matches!(&e, ApiError::ConfigError(_)) {
                    error!(
                        workspace_id = %workspace_id,
                        error = %e,
                        "Workspace embedding configuration error - returning to user"
                    );
                    return Err(e);
                }

                // For other errors, fallback to default with warning
                warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "Failed to get workspace embedding config, using default"
                );
                state
                    .sota_engine
                    .query(engine_request)
                    .await
                    .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
            }
        }
    } else {
        // No workspace context, use default engine embedding
        state
            .sota_engine
            .query(engine_request)
            .await
            .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?
    };

    // Convert sources from context
    let mut sources = Vec::new();

    // Apply simple relevance-based reranking if enabled
    // In a production environment, this would call an external reranker service (e.g., Cohere)
    let reranked = request.enable_rerank;
    let rerank_time_ms = if reranked {
        // Simulate rerank time for now - actual implementation would call rerank API
        Some(5u64)
    } else {
        None
    };

    // Get rerank_top_k or default to all results
    let rerank_top_k = request.rerank_top_k.unwrap_or(usize::MAX);

    // Build chunk sources with rerank scores
    let mut ref_counter = 1usize;
    let mut chunk_sources: Vec<SourceReference> = result
        .context
        .chunks
        .iter()
        .map(|chunk| {
            // Calculate simulated rerank score based on original score
            let rerank_score = if reranked {
                // Normalize score to 0-1 range and apply slight boost
                Some((chunk.score.min(1.0) * 0.95 + 0.05).min(1.0))
            } else {
                None
            };

            let ref_id = ref_counter;
            ref_counter += 1;

            SourceReference {
                source_type: "chunk".to_string(),
                id: chunk.id.clone(),
                score: chunk.score,
                rerank_score,
                snippet: Some(chunk.content.chars().take(200).collect()),
                reference_id: Some(ref_id),
                document_id: chunk.document_id.clone(),
                file_path: None, // TODO: Resolve document_id to file_path
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                chunk_index: chunk.chunk_index,
            }
        })
        .collect();

    // Sort by rerank score if reranking is enabled
    if reranked {
        chunk_sources.sort_by(|a, b| {
            b.rerank_score
                .unwrap_or(0.0)
                .partial_cmp(&a.rerank_score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        chunk_sources.truncate(rerank_top_k);
    }

    sources.extend(chunk_sources);

    for entity in &result.context.entities {
        let ref_id = ref_counter;
        ref_counter += 1;

        sources.push(SourceReference {
            source_type: "entity".to_string(),
            id: entity.name.clone(),
            score: entity.score,
            rerank_score: None,
            snippet: Some(entity.description.chars().take(200).collect()),
            reference_id: Some(ref_id),
            document_id: entity.source_document_id.clone(),
            file_path: entity.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
        });
    }

    for rel in &result.context.relationships {
        let ref_id = ref_counter;
        ref_counter += 1;

        sources.push(SourceReference {
            source_type: "relationship".to_string(),
            id: format!("{}->{}", rel.source, rel.target),
            score: rel.score,
            rerank_score: None,
            snippet: Some(format!(
                "{} {} {}",
                rel.source, rel.relation_type, rel.target
            )),
            reference_id: Some(ref_id),
            document_id: rel.source_document_id.clone(),
            file_path: rel.source_file_path.clone(),
            start_line: None,
            end_line: None,
            chunk_index: None,
        });
    }

    // Generate conversation ID if conversation history was provided
    let conversation_id = if request.conversation_history.is_some() {
        Some(uuid::Uuid::new_v4().to_string())
    } else {
        None
    };

    // SPEC-032 Item 18, 22: Get LLM provider/model info for lineage tracking
    let (llm_provider, llm_model) =
        get_workspace_llm_info(&state, tenant_ctx.workspace_id.as_deref()).await;

    // SPEC-032 Item 18: Calculate tokens per second
    let tokens_used = if result.stats.generated_tokens > 0 {
        Some(result.stats.generated_tokens)
    } else {
        None
    };

    let tokens_per_second =
        if result.stats.generation_time_ms > 0 && result.stats.generated_tokens > 0 {
            Some(
                (result.stats.generated_tokens as f32) / (result.stats.generation_time_ms as f32)
                    * 1000.0,
            )
        } else {
            None
        };

    let response = QueryResponse {
        answer: result.answer,
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
            rerank_time_ms,
            // SPEC-032 Item 18, 22: Token metrics and model lineage
            tokens_used,
            tokens_per_second,
            llm_provider,
            llm_model,
        },
        conversation_id,
        reranked,
    };

    Ok(Json(response))
}

use axum::response::sse::{Event, Sse};
use futures::StreamExt;

/// Execute a streaming query.
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
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>>> {
    debug!(
        tenant_id = ?tenant_ctx.tenant_id,
        workspace_id = ?tenant_ctx.workspace_id,
        query = %request.query,
        "Executing streaming query with tenant context"
    );

    validate_query(&request.query, state.config.max_query_length)?;

    // Parse query mode
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| QueryMode::parse(m))
        .unwrap_or(QueryMode::Hybrid);

    // Build engine query request with tenant context
    let mut engine_request = EngineQueryRequest::new(&request.query).with_mode(mode);

    // OODA-231.1: Fetch workspace to get correct tenant_id for data queries
    // WHY: Header tenant_id is for authentication (random UUID from frontend).
    // But the graph data was ingested with the workspace's actual tenant_id.
    let workspace = if let Some(ref workspace_id) = tenant_ctx.workspace_id {
        get_workspace(&state, workspace_id).await.ok().flatten()
    } else {
        None
    };

    // Use workspace's tenant_id for data queries, fall back to header tenant_id
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

    // Execute streaming query using SOTA engine (LightRAG-style)
    let stream = state
        .sota_engine
        .query_stream(engine_request)
        .await
        .map_err(|e| ApiError::Internal(format!("Streaming query failed: {}", e)))?;

    let sse_stream = stream.map(|res| match res {
        Ok(text) => Ok(Event::default().data(text)),
        Err(e) => Ok(Event::default().data(format!("Error: {}", e))),
    });

    Ok(Sse::new(sse_stream))
}

/// Get workspace by ID for tenant isolation.
///
/// @implements OODA-231.1: Correct tenant_id for data queries
async fn get_workspace(
    state: &AppState,
    workspace_id: &str,
) -> Result<Option<edgequake_core::Workspace>, ApiError> {
    use uuid::Uuid;

    let workspace_uuid = Uuid::parse_str(workspace_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid workspace ID: {}", e)))?;

    state
        .workspace_service
        .get_workspace(workspace_uuid)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get workspace: {}", e)))
}

/// Get workspace-specific embedding provider for query execution.
///
/// @implements SPEC-032: Workspace-specific embedding in query process
/// @implements OODA-259: Delegates to WorkspaceProviderResolver to eliminate duplication
///
/// This function delegates to [`WorkspaceProviderResolver::resolve_embedding_provider_optional`]
/// which provides the canonical implementation for workspace-aware embedding provider creation.
///
/// # Arguments
///
/// * `state` - Application state containing workspace service
/// * `workspace_id` - ID of the workspace to get embedding config for
///
/// # Returns
///
/// - `Ok(Some(provider))` - Workspace-specific embedding provider
/// - `Ok(None)` - Workspace uses default embedding, no override needed
/// - `Err(_)` - Error looking up workspace or creating provider
pub async fn get_workspace_embedding_provider(
    state: &AppState,
    workspace_id: &str,
) -> Result<Option<std::sync::Arc<dyn edgequake_query::EmbeddingProvider>>, ApiError> {
    // OODA-259: Delegate to resolver to eliminate code duplication
    // The resolver now provides `resolve_embedding_provider_optional` which returns
    // Ok(None) for fallback semantics (workspace has no embedding config)
    let resolver = WorkspaceProviderResolver::new(state.workspace_service.clone());
    let result = resolver
        .resolve_embedding_provider_optional(workspace_id)
        .await
        .map_err(ApiError::from)?;

    // Extract just the Arc<dyn EmbeddingProvider> from ResolvedEmbeddingProvider
    Ok(result.map(|resolved| resolved.provider))
}

/// Get workspace-specific vector storage for query execution.
///
/// SPEC-033: Workspace vector isolation.
///
/// This function looks up the workspace's embedding dimension and gets or creates
/// a workspace-specific vector storage instance. If the workspace uses the default
/// dimension, returns None to indicate the default should be used.
///
/// @implements OODA-228: Fix dimension mismatch in chat handler
///
/// # Arguments
///
/// * `state` - Application state containing workspace service and vector registry
/// * `workspace_id` - ID of the workspace to get vector storage for
///
/// # Returns
///
/// - `Ok(Some(storage))` - Workspace-specific vector storage
/// - `Ok(None)` - Workspace uses default storage, no override needed
/// - `Err(_)` - Error looking up workspace or creating storage
pub async fn get_workspace_vector_storage(
    state: &AppState,
    workspace_id: &str,
) -> Result<Option<std::sync::Arc<dyn edgequake_storage::traits::VectorStorage>>, ApiError> {
    use edgequake_storage::traits::WorkspaceVectorConfig;
    use uuid::Uuid;

    // Parse workspace ID
    let workspace_uuid = Uuid::parse_str(workspace_id)
        .map_err(|e| ApiError::BadRequest(format!("Invalid workspace ID: {}", e)))?;

    // Get workspace from service
    let workspace = state
        .workspace_service
        .get_workspace(workspace_uuid)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get workspace: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Workspace not found: {}", workspace_id)))?;

    // Create workspace-specific vector storage config
    let config = WorkspaceVectorConfig {
        workspace_id: workspace_uuid,
        dimension: workspace.embedding_dimension,
        namespace: "default".to_string(),
    };

    debug!(
        workspace_id = %workspace_id,
        dimension = workspace.embedding_dimension,
        "Getting workspace-specific vector storage"
    );

    // Get or create workspace vector storage
    // OODA-225: Auto-evict and retry on dimension mismatch
    // WHY: When embedding provider changes (e.g., Ollama 768 → OpenAI 1536), the cached
    // vector storage instance may hold the old dimension. If get_or_create fails due to
    // dimension mismatch, we evict the cache and retry with the new dimension.
    let storage = match state.vector_registry.get_or_create(config.clone()).await {
        Ok(s) => s,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("Dimension mismatch") || error_msg.contains("cached=") {
                // Dimension mismatch detected - evict cache and retry
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %error_msg,
                    "Dimension mismatch detected, evicting cache and retrying"
                );
                state.vector_registry.evict(&workspace_uuid).await;

                // Retry after eviction
                state
                    .vector_registry
                    .get_or_create(config)
                    .await
                    .map_err(|e2| {
                        ApiError::Internal(format!(
                            "Failed to create vector storage for workspace {} after cache eviction: {}",
                            workspace_id, e2
                        ))
                    })?
            } else {
                return Err(ApiError::Internal(format!(
                    "Failed to create vector storage for workspace {}: {}",
                    workspace_id, e
                )));
            }
        }
    };

    Ok(Some(storage))
}

/// Get workspace LLM provider and model info for lineage tracking.
///
/// @implements SPEC-032 Item 22: Display model used after tokens/second
///
/// # Returns
///
/// Tuple of (Option<provider>, Option<model>) from workspace config or defaults.
async fn get_workspace_llm_info(
    state: &AppState,
    workspace_id: Option<&str>,
) -> (Option<String>, Option<String>) {
    use edgequake_core::types::{DEFAULT_LLM_MODEL, DEFAULT_LLM_PROVIDER};
    use uuid::Uuid;

    // If no workspace, return defaults
    let workspace_id = match workspace_id {
        Some(id) => id,
        None => {
            return (
                Some(DEFAULT_LLM_PROVIDER.to_string()),
                Some(DEFAULT_LLM_MODEL.to_string()),
            );
        }
    };

    // Try to get workspace config
    let workspace_uuid = match Uuid::parse_str(workspace_id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return (
                Some(DEFAULT_LLM_PROVIDER.to_string()),
                Some(DEFAULT_LLM_MODEL.to_string()),
            );
        }
    };

    match state.workspace_service.get_workspace(workspace_uuid).await {
        Ok(Some(workspace)) => {
            let provider = if workspace.llm_provider.is_empty() {
                Some(DEFAULT_LLM_PROVIDER.to_string())
            } else {
                Some(workspace.llm_provider)
            };
            let model = if workspace.llm_model.is_empty() {
                Some(DEFAULT_LLM_MODEL.to_string())
            } else {
                Some(workspace.llm_model)
            };
            (provider, model)
        }
        _ => (
            Some(DEFAULT_LLM_PROVIDER.to_string()),
            Some(DEFAULT_LLM_MODEL.to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_validation() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();

        let request = QueryRequest {
            query: "".to_string(),
            mode: None,
            context_only: false,
            prompt_only: false,
            include_references: false,
            max_results: None,
            conversation_history: None,
            enable_rerank: true,
            rerank_model: None,
            rerank_top_k: None,
            llm_provider: None,
            llm_model: None,
        };

        let result = execute_query(State(state), tenant_ctx, Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_success() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();

        let request = QueryRequest {
            query: "What is Rust?".to_string(),
            mode: Some("naive".to_string()),
            context_only: false,
            prompt_only: false,
            include_references: true,
            max_results: Some(5),
            conversation_history: None,
            enable_rerank: true,
            rerank_model: None,
            rerank_top_k: None,
            llm_provider: None,
            llm_model: None,
        };

        let result = execute_query(State(state), tenant_ctx, Json(request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_query_success() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();

        let request = StreamQueryRequest {
            query: "What is Rust?".to_string(),
            mode: Some("naive".to_string()),
        };

        let result = stream_query(State(state), tenant_ctx, Json(request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_modes() {
        let state = AppState::test_state();
        let modes = vec!["naive", "local", "global", "hybrid", "mix"];

        for mode in modes {
            let tenant_ctx = TenantContext::default();
            let request = QueryRequest {
                query: "Test query".to_string(),
                mode: Some(mode.to_string()),
                context_only: false,
                prompt_only: false,
                include_references: false,
                max_results: None,
                conversation_history: None,
                enable_rerank: false,
                rerank_model: None,
                rerank_top_k: None,
                llm_provider: None,
                llm_model: None,
            };

            let result = execute_query(State(state.clone()), tenant_ctx, Json(request)).await;
            assert!(result.is_ok(), "Mode '{}' should succeed", mode);
        }
    }

    #[tokio::test]
    async fn test_query_with_context_only() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();

        let request = QueryRequest {
            query: "What is Rust?".to_string(),
            mode: Some("naive".to_string()),
            context_only: true,
            prompt_only: false,
            include_references: false,
            max_results: Some(3),
            conversation_history: None,
            enable_rerank: false,
            rerank_model: None,
            rerank_top_k: None,
            llm_provider: None,
            llm_model: None,
        };

        let result = execute_query(State(state), tenant_ctx, Json(request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_whitespace_only_fails() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();

        let request = QueryRequest {
            query: "   \t\n   ".to_string(),
            mode: None,
            context_only: false,
            prompt_only: false,
            include_references: false,
            max_results: None,
            conversation_history: None,
            enable_rerank: true,
            rerank_model: None,
            rerank_top_k: None,
            llm_provider: None,
            llm_model: None,
        };

        let result = execute_query(State(state), tenant_ctx, Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_query_empty_fails() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();

        let request = StreamQueryRequest {
            query: "".to_string(),
            mode: None,
        };

        let result = stream_query(State(state), tenant_ctx, Json(request)).await;
        assert!(result.is_err());
    }
}
