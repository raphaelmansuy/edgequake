//! Query handlers.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use edgequake_query::{QueryMode, QueryRequest as EngineQueryRequest};

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

    /// Maximum number of results.
    #[serde(default)]
    pub max_results: Option<usize>,
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

    /// Content snippet.
    pub snippet: Option<String>,
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
        return Err(ApiError::ValidationError("Query cannot be empty".to_string()));
    }

    if request.query.len() > state.config.max_query_length {
        return Err(ApiError::BadRequest(format!(
            "Query exceeds maximum length of {} characters",
            state.config.max_query_length
        )));
    }

    // Parse query mode
    let mode = request.mode
        .as_ref()
        .and_then(|m| QueryMode::from_str(m))
        .unwrap_or(QueryMode::Hybrid);

    // Build engine query request
    let mut engine_request = EngineQueryRequest::new(&request.query)
        .with_mode(mode);
    
    if request.context_only {
        engine_request = engine_request.context_only();
    }

    // Execute query using the query engine
    let result = state.query_engine.query(engine_request).await
        .map_err(|e| ApiError::Internal(format!("Query failed: {}", e)))?;

    // Convert sources from context
    let mut sources = Vec::new();
    
    for chunk in &result.context.chunks {
        sources.push(SourceReference {
            source_type: "chunk".to_string(),
            id: chunk.id.clone(),
            score: chunk.score,
            snippet: Some(chunk.content.chars().take(200).collect()),
        });
    }

    for entity in &result.context.entities {
        sources.push(SourceReference {
            source_type: "entity".to_string(),
            id: entity.name.clone(),
            score: entity.score,
            snippet: Some(entity.description.chars().take(200).collect()),
        });
    }

    for rel in &result.context.relationships {
        sources.push(SourceReference {
            source_type: "relationship".to_string(),
            id: format!("{}->{}", rel.source, rel.target),
            score: rel.score,
            snippet: Some(format!("{} {} {}", rel.source, rel.relation_type, rel.target)),
        });
    }

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
        },
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
    State(_state): State<AppState>,
    Json(request): Json<StreamQueryRequest>,
) -> ApiResult<&'static str> {
    if request.query.trim().is_empty() {
        return Err(ApiError::ValidationError("Query cannot be empty".to_string()));
    }

    // Streaming implementation would use SSE
    Ok("Streaming not yet implemented")
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
            max_results: None,
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
            max_results: Some(5),
        };

        let result = execute_query(State(state), Json(request)).await;
        assert!(result.is_ok());
    }
}
