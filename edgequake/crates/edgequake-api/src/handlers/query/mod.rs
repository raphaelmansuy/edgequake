//! Query execution handlers.
//!
//! @implements FEAT0403
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

mod query_execute;
mod query_stream;
pub(crate) mod workspace_resolve;

pub use query_execute::*;
pub use query_stream::*;

// Re-export DTOs for backward compatibility
pub use crate::handlers::query_types::{
    ConversationMessage, QueryRequest, QueryResponse, QueryStats, SourceReference,
    StreamQueryRequest,
};

use std::collections::{HashMap, HashSet};

use tracing::{debug, warn};

use crate::handlers::query_types::SourceReference as SourceRef;

// ============================================================================
// Shared Helper Functions
// ============================================================================

/// Resolve document IDs to document titles from KV metadata.
///
/// Looks up `"{document_id}-metadata"` in KV storage for each unique document ID,
/// extracting the `"title"` field (falling back to `"file_name"`).
/// Returns a HashMap mapping document_id → document title.
async fn resolve_document_names(
    kv_storage: &dyn edgequake_storage::traits::KVStorage,
    document_ids: &[String],
) -> HashMap<String, String> {
    if document_ids.is_empty() {
        return HashMap::new();
    }

    // Deduplicate — multiple chunks often reference the same document
    let unique_ids: Vec<String> = document_ids
        .iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .cloned()
        .collect();

    let mut doc_names = HashMap::new();

    for doc_id in &unique_ids {
        let metadata_key = format!("{}-metadata", doc_id);
        match kv_storage.get_by_id(&metadata_key).await {
            Ok(Some(metadata)) => {
                if let Some(title) = metadata
                    .get("title")
                    .or_else(|| metadata.get("file_name"))
                    .and_then(|v| v.as_str())
                {
                    doc_names.insert(doc_id.clone(), title.to_string());
                }
            }
            Ok(None) => {
                debug!(document_id = %doc_id, "No metadata found for document");
            }
            Err(e) => {
                warn!(document_id = %doc_id, error = %e, "Failed to fetch document metadata");
            }
        }
    }

    doc_names
}

/// Resolve `file_path` for chunk sources that are missing it.
///
/// Collects unique document IDs from chunk sources, performs a batched KV lookup
/// for document metadata, and patches `file_path` with the resolved document title.
/// Non-chunk sources and sources that already have `file_path` are left unchanged.
pub(crate) async fn resolve_chunk_file_paths(
    kv_storage: &dyn edgequake_storage::traits::KVStorage,
    sources: &mut [SourceRef],
) {
    let chunk_doc_ids: Vec<String> = sources
        .iter()
        .filter(|s| s.source_type == "chunk" && s.file_path.is_none())
        .filter_map(|s| s.document_id.clone())
        .collect();

    if chunk_doc_ids.is_empty() {
        return;
    }

    let doc_names = resolve_document_names(kv_storage, &chunk_doc_ids).await;

    for source in sources.iter_mut() {
        if source.source_type == "chunk" && source.file_path.is_none() {
            if let Some(ref doc_id) = source.document_id {
                source.file_path = doc_names.get(doc_id).cloned();
            }
        }
    }
}

// Re-export workspace resolve functions for other modules
pub use workspace_resolve::{get_workspace_embedding_provider, get_workspace_vector_storage};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::TenantContext;
    use crate::state::AppState;
    use axum::extract::State;
    use axum::Json;

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
