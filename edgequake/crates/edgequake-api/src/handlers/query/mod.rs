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

pub mod context;
pub(crate) mod document_filter_resolver;
pub mod json_or_image;
mod query_execute;
mod query_stream;
pub(crate) mod workspace_resolve;

pub use context::{
    fetch_query_context, get_context_artifact, retrieve_query_context, search_query_context,
};
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

/// Resolve `file_path` for chunk sources that are missing it.
///
/// Collects unique document IDs from chunk sources, performs a batched KV lookup
/// for document metadata, and patches `file_path` with the resolved document title.
/// Non-chunk sources and sources that already have `file_path` are left unchanged.
/// Resolve document facts for chunk sources: title, kind, MIME and chunk modality.
///
/// WHY one lookup for four facts: this already walked the metadata for every cited
/// document to fill `file_path`. Fetching the same row again for `doc_kind` would double
/// the reads on the query hot path. `file_path` keeps its old meaning (the title) because
/// citation headers depend on it; the kind and MIME are added beside it so a client can
/// tell a picture citation from a text one without parsing a filename.
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

    let facts = resolve_document_facts(kv_storage, &chunk_doc_ids).await;

    for source in sources.iter_mut() {
        if source.source_type != "chunk" {
            continue;
        }
        let Some(doc_id) = source.document_id.as_ref() else {
            continue;
        };
        let Some(fact) = facts.get(doc_id) else {
            continue;
        };
        if source.file_path.is_none() {
            source.file_path = Some(fact.title.clone());
        }
        source.doc_kind = fact.doc_kind.clone();
        source.mime_type = fact.mime_type.clone();
        source.source_modality = fact.source_modality.clone();
    }
}

/// The citation facts one document contributes.
#[derive(Debug, Clone, Default)]
struct DocumentFacts {
    title: String,
    doc_kind: Option<String>,
    mime_type: Option<String>,
    source_modality: Option<String>,
}

/// Batched KV metadata read: one `get_by_id` per unique document.
async fn resolve_document_facts(
    kv_storage: &dyn edgequake_storage::traits::KVStorage,
    document_ids: &[String],
) -> HashMap<String, DocumentFacts> {
    if document_ids.is_empty() {
        return HashMap::new();
    }

    let mut facts = HashMap::new();
    for doc_id in document_ids.iter().collect::<HashSet<_>>() {
        let metadata_key =
            crate::services::document_metadata_scan::metadata_key_for_document(doc_id);
        match kv_storage.get_by_id(&metadata_key).await {
            Ok(Some(metadata)) => {
                let title = metadata
                    .get("title")
                    .or_else(|| metadata.get("file_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                // The upload path stores the MIME under `content_type`; `mime_type` is the
                // fallback for payloads written by an older build.
                let mime = string_field(&metadata, "content_type")
                    .or_else(|| string_field(&metadata, "mime_type"));
                // WHY derived rather than read: `doc_kind` is a generated column in Postgres
                // and is not copied into the KV document metadata. `DocKind::from_content_type`
                // is the single rule the `doc_kinds` filter and the SQL mapping both use, so
                // deriving here cannot drift from them.
                let kind = mime
                    .as_deref()
                    .map(edgequake_core::rag_profile::DocKind::from_content_type);
                facts.insert(
                    (*doc_id).clone(),
                    DocumentFacts {
                        title,
                        doc_kind: kind.map(|k| k.as_str().to_string()),
                        mime_type: mime,
                        // SPEC-180: an image document's single chunk is embedded from pixels.
                        // Older metadata does not carry the field, so the kind implies it.
                        source_modality: string_field(&metadata, "source_modality").or_else(|| {
                            matches!(kind, Some(edgequake_core::rag_profile::DocKind::Image))
                                .then(|| "image".to_string())
                                .or_else(|| kind.map(|_| "text".to_string()))
                        }),
                    },
                );
            }
            Ok(None) => debug!(document_id = %doc_id, "No metadata found for document"),
            Err(e) => warn!(document_id = %doc_id, error = %e, "Failed to fetch document metadata"),
        }
    }
    facts
}

fn string_field(metadata: &serde_json::Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

// Re-export workspace resolve functions for other modules
pub use workspace_resolve::{
    get_workspace_embedding_provider, get_workspace_vector_storage, langfuse_query_identity,
    resolve_query_workspace,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::auth::OptionalAuth;
    use crate::handlers::context_types::ContentGranularity;
    use crate::middleware::TenantContext;
    use crate::state::AppState;
    use axum::extract::State;
    use axum::Extension;
    use edgequake_observability::{PropagationHeaders, RequestContext};

    fn no_auth() -> OptionalAuth {
        OptionalAuth(None)
    }

    fn test_extensions() -> (Extension<RequestContext>, Extension<PropagationHeaders>) {
        (
            Extension(RequestContext::new(uuid::Uuid::new_v4().to_string())),
            Extension(PropagationHeaders::default()),
        )
    }

    /// The citation must say *what kind of document* it is, not only where it came from.
    ///
    /// WHY: the UI decided "this citation is a picture" by testing `file_path` for an image
    /// extension. `file_path` is filled with the document *title* (see
    /// `resolve_chunk_file_paths`), and an image document is uploaded with a caption title
    /// that has no extension, so the test never matched and the "Matching images" grid was
    /// invisible against a real backend. `doc_kind` / `mime_type` / `source_modality` are
    /// the facts a client needs, and they are already stored at ingest.
    #[tokio::test]
    async fn chunk_source_reports_kind_mime_and_modality() {
        use edgequake_storage::traits::KVStorage;
        let kv = edgequake_storage::adapters::memory::MemoryKVStorage::new("test");
        kv.initialize().await.unwrap();
        let doc_id = "doc-image-1";
        let key = crate::services::document_metadata_scan::metadata_key_for_document(doc_id);
        kv.upsert(&[(
            key,
            serde_json::json!({
                "title": "A beagle with its tongue out, outdoors",
                "doc_kind": "image",
                "content_type": "image/jpeg",
                "source_modality": "image",
            }),
        )])
        .await
        .unwrap();

        let mut sources = vec![SourceReference {
            source_type: "chunk".to_string(),
            id: "chunk-1".to_string(),
            score: 0.9,
            similarity: None,
            rerank_score: None,
            snippet: Some(String::new()),
            reference_id: Some(1),
            document_id: Some(doc_id.to_string()),
            file_path: None,
            start_line: None,
            end_line: None,
            chunk_index: None,
            page_start: None,
            page_end: None,
            entity_type: None,
            degree: None,
            source_chunk_ids: None,
            doc_kind: None,
            mime_type: None,
            source_modality: None,
        }];

        resolve_chunk_file_paths(&kv, &mut sources).await;

        assert_eq!(sources[0].doc_kind.as_deref(), Some("image"));
        assert_eq!(sources[0].mime_type.as_deref(), Some("image/jpeg"));
        assert_eq!(sources[0].source_modality.as_deref(), Some("image"));
        // `file_path` keeps its old meaning: the title, not a path with an extension.
        assert_eq!(
            sources[0].file_path.as_deref(),
            Some("A beagle with its tongue out, outdoors")
        );
    }

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
            reasoning_effort: None,
            system_prompt: None,
            question_type: None,
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            document_filter: None,
            mix_weights: None,
            extra_headers: None,
            include_subgraph: true,
            content_granularity: ContentGranularity::Citation,
            session_id: None,
        };

        let (ctx, propagation) = test_extensions();
        let result = execute_query(
            State(state),
            tenant_ctx,
            no_auth(),
            ctx,
            propagation,
            json_or_image::JsonOrImage::text(request),
        )
        .await;
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
            reasoning_effort: None,
            system_prompt: None,
            question_type: None,
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            document_filter: None,
            mix_weights: None,
            extra_headers: None,
            include_subgraph: true,
            content_granularity: ContentGranularity::Citation,
            session_id: None,
        };

        let (ctx, propagation) = test_extensions();
        let result = execute_query(
            State(state),
            tenant_ctx,
            no_auth(),
            ctx,
            propagation,
            json_or_image::JsonOrImage::text(request),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_query_success() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();

        let request = StreamQueryRequest {
            rerank_model: None,
            query: "What is Rust?".to_string(),
            mode: Some("naive".to_string()),
            system_prompt: None,
            question_type: None,
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            document_filter: None,
            llm_provider: None,
            llm_model: None,
            reasoning_effort: None,
            stream_format: None,
            extra_headers: None,
            include_subgraph: true,
            content_granularity: ContentGranularity::Citation,
            session_id: None,
        };

        let (ctx, propagation) = test_extensions();
        let result = stream_query(
            State(state),
            tenant_ctx,
            no_auth(),
            ctx,
            propagation,
            json_or_image::JsonOrImage::text(request),
        )
        .await;
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
                reasoning_effort: None,
                system_prompt: None,
                question_type: None,
                hl_keywords: None,
                ll_keywords: None,
                response_type: None,
                document_filter: None,
                mix_weights: None,
                extra_headers: None,
                include_subgraph: true,
                content_granularity: ContentGranularity::Citation,
                session_id: None,
            };

            let (ctx, propagation) = test_extensions();
            let result = execute_query(
                State(state.clone()),
                tenant_ctx,
                no_auth(),
                ctx,
                propagation,
                json_or_image::JsonOrImage::text(request),
            )
            .await;
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
            reasoning_effort: None,
            system_prompt: None,
            question_type: None,
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            document_filter: None,
            mix_weights: None,
            extra_headers: None,
            include_subgraph: true,
            content_granularity: ContentGranularity::Citation,
            session_id: None,
        };

        let (ctx, propagation) = test_extensions();
        let result = execute_query(
            State(state),
            tenant_ctx,
            no_auth(),
            ctx,
            propagation,
            json_or_image::JsonOrImage::text(request),
        )
        .await;
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
            reasoning_effort: None,
            system_prompt: None,
            question_type: None,
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            document_filter: None,
            mix_weights: None,
            extra_headers: None,
            include_subgraph: true,
            content_granularity: ContentGranularity::Citation,
            session_id: None,
        };

        let (ctx, propagation) = test_extensions();
        let result = execute_query(
            State(state),
            tenant_ctx,
            no_auth(),
            ctx,
            propagation,
            json_or_image::JsonOrImage::text(request),
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stream_query_empty_fails() {
        let state = AppState::test_state();
        let tenant_ctx = TenantContext::default();

        let request = StreamQueryRequest {
            rerank_model: None,
            query: "".to_string(),
            mode: None,
            system_prompt: None,
            question_type: None,
            hl_keywords: None,
            ll_keywords: None,
            response_type: None,
            document_filter: None,
            llm_provider: None,
            llm_model: None,
            reasoning_effort: None,
            stream_format: None,
            extra_headers: None,
            include_subgraph: true,
            content_granularity: ContentGranularity::Citation,
            session_id: None,
        };

        let (ctx, propagation) = test_extensions();
        let result = stream_query(
            State(state),
            tenant_ctx,
            no_auth(),
            ctx,
            propagation,
            json_or_image::JsonOrImage::text(request),
        )
        .await;
        assert!(result.is_err());
    }
}
