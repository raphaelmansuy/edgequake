//! Query handlers.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

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

    let start = std::time::Instant::now();

    // For now, return a placeholder response
    // Full implementation would use the QueryEngine
    let mode = request.mode.clone().unwrap_or_else(|| "hybrid".to_string());

    let response = QueryResponse {
        answer: format!(
            "This is a placeholder response for query: '{}'. \
             Full implementation would use the QueryEngine with {} mode.",
            request.query, mode
        ),
        mode,
        sources: vec![],
        stats: QueryStats {
            embedding_time_ms: 0,
            retrieval_time_ms: 0,
            generation_time_ms: 0,
            total_time_ms: start.elapsed().as_millis() as u64,
            sources_retrieved: 0,
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
