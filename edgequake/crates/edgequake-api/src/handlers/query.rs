//! Query handlers.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};

/// A single message in the conversation history.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ConversationMessage {
    /// Role of the message sender (user or assistant).
    pub role: String,

    /// Content of the message.
    pub content: String,
}

/// Query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct QueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode (naive, local, global, hybrid, mix).
    #[serde(default)]
    pub mode: Option<String>,

    /// Only return context, don't generate an answer.
    #[serde(default)]
    pub context_only: bool,

    /// Return the formatted prompt instead of calling the LLM.
    /// Useful for debugging or using your own LLM.
    #[serde(default)]
    pub prompt_only: bool,

    /// Include detailed reference metadata (document_id, file_path, reference_id) in sources.
    #[serde(default)]
    pub include_references: bool,

    /// Maximum number of results.
    #[serde(default)]
    pub max_results: Option<usize>,

    /// Conversation history for multi-turn context.
    #[serde(default)]
    pub conversation_history: Option<Vec<ConversationMessage>>,

    /// Enable reranking of retrieved chunks for better relevance.
    #[serde(default = "default_enable_rerank")]
    pub enable_rerank: bool,

    /// Rerank model to use (e.g., "cohere-rerank-v3").
    #[serde(default)]
    pub rerank_model: Option<String>,

    /// Top K chunks to keep after reranking.
    #[serde(default)]
    pub rerank_top_k: Option<usize>,
}

fn default_enable_rerank() -> bool {
    true
}

/// Query response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueryResponse {
    /// Generated answer.
    pub answer: String,

    /// Query mode used.
    pub mode: String,

    /// Retrieved context sources.
    pub sources: Vec<SourceReference>,

    /// Query statistics.
    pub stats: QueryStats,

    /// Conversation ID for multi-turn context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,

    /// Whether reranking was applied.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reranked: bool,
}

/// A source reference.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SourceReference {
    /// Source type (chunk, entity, relationship).
    pub source_type: String,

    /// Source ID.
    pub id: String,

    /// Relevance score.
    pub score: f32,

    /// Rerank score (if reranking was applied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_score: Option<f32>,

    /// Content snippet.
    pub snippet: Option<String>,

    /// Reference ID for citation (1, 2, 3, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<usize>,

    /// Document ID that this reference came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,

    /// Original file path of the source document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// Query statistics.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QueryStats {
    /// Embedding time in ms.
    pub embedding_time_ms: u64,

    /// Retrieval time in ms.
    pub retrieval_time_ms: u64,

    /// Generation time in ms.
    pub generation_time_ms: u64,

    /// Total time in ms.
    pub total_time_ms: u64,

    /// Number of sources retrieved.
    pub sources_retrieved: usize,

    /// Rerank time in ms (if reranking was applied).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_time_ms: Option<u64>,
}

/// Execute a query.
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
    Json(request): Json<QueryRequest>,
) -> ApiResult<Json<QueryResponse>> {
    if request.query.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Query cannot be empty".to_string(),
        ));
    }

    if request.query.len() > state.config.max_query_length {
        return Err(ApiError::BadRequest(format!(
            "Query exceeds maximum length of {} characters",
            state.config.max_query_length
        )));
    }

    // Parse query mode
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| QueryMode::from_str(m))
        .unwrap_or(QueryMode::Hybrid);

    // Build engine query request with conversation history
    let mut engine_request = EngineQueryRequest::new(&request.query).with_mode(mode);

    if request.context_only {
        engine_request = engine_request.context_only();
    }

    if request.prompt_only {
        engine_request = engine_request.prompt_only();
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

    // Execute query using the query engine
    let result = state
        .query_engine
        .query(engine_request)
        .await
        .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?;

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
            document_id: None,
            file_path: None,
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
            document_id: None,
            file_path: None,
        });
    }

    // Generate conversation ID if conversation history was provided
    let conversation_id = if request.conversation_history.is_some() {
        Some(uuid::Uuid::new_v4().to_string())
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
        },
        conversation_id,
        reranked,
    };

    Ok(Json(response))
}

/// Streaming query request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StreamQueryRequest {
    /// The query text.
    pub query: String,

    /// Query mode.
    #[serde(default)]
    pub mode: Option<String>,
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
    Json(request): Json<StreamQueryRequest>,
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>>> {
    if request.query.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "Query cannot be empty".to_string(),
        ));
    }

    // Parse query mode
    let mode = request
        .mode
        .as_ref()
        .and_then(|m| QueryMode::from_str(m))
        .unwrap_or(QueryMode::Hybrid);

    // Build engine query request
    let engine_request = EngineQueryRequest::new(&request.query).with_mode(mode);

    // Execute streaming query
    let stream = state
        .query_engine
        .query_stream(engine_request)
        .await
        .map_err(|e| ApiError::Internal(format!("Streaming query failed: {}", e)))?;

    let sse_stream = stream.map(|res| match res {
        Ok(text) => Ok(Event::default().data(text)),
        Err(e) => Ok(Event::default().data(format!("Error: {}", e))),
    });

    Ok(Sse::new(sse_stream))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_validation() {
        let state = AppState::test_state();

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
        };

        let result = execute_query(State(state), Json(request)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_success() {
        let state = AppState::test_state();

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
        };

        let result = execute_query(State(state), Json(request)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_query_success() {
        let state = AppState::test_state();

        let request = StreamQueryRequest {
            query: "What is Rust?".to_string(),
            mode: Some("naive".to_string()),
        };

        let result = stream_query(State(state), Json(request)).await;
        assert!(result.is_ok());
    }
}
