//! Query entry points using workspace-specific vector storage.
//!
//! Contains: `query_with_workspace_config`, `query_with_full_config`,
//!           `query_with_vector_storage`.

use std::sync::Arc;

use crate::error::Result;
use edgequake_storage::traits::VectorStorage;

use super::super::QueryEngine;
use super::query_pipeline::QueryProviders;

impl QueryEngine {
    /// Execute a query with workspace-specific vector storage (default embedding).
    ///
    /// @implements SPEC-033 (Workspace vector isolation)
    pub async fn query_with_vector_storage(
        &self,
        request: crate::types::QueryRequest,
        vector_storage: Arc<dyn VectorStorage>,
    ) -> Result<crate::types::QueryResponse> {
        self.query_with_workspace_config(request, self.embedding_provider.clone(), vector_storage)
            .await
    }

    /// Execute a query with workspace-specific embedding and vector storage.
    ///
    /// @implements SPEC-033: Full workspace isolation for vector storage.
    pub async fn query_with_workspace_config(
        &self,
        request: crate::types::QueryRequest,
        embedding_provider: Arc<dyn crate::EmbeddingProvider>,
        vector_storage: Arc<dyn VectorStorage>,
    ) -> Result<crate::types::QueryResponse> {
        self.run_query_pipeline(
            request,
            QueryProviders {
                embedding: embedding_provider.as_ref(),
                vector_storage: Some(&vector_storage),
                keyword_llm: None,
                answer_llm: None,
            },
        )
        .await
    }

    /// Execute a query with full workspace configuration AND optional LLM override.
    ///
    /// @implements SPEC-032, SPEC-033, OODA-228
    pub async fn query_with_full_config(
        &self,
        request: crate::types::QueryRequest,
        embedding_provider: Arc<dyn crate::EmbeddingProvider>,
        vector_storage: Arc<dyn VectorStorage>,
        llm_provider: Option<Arc<dyn crate::LLMProvider>>,
    ) -> Result<crate::types::QueryResponse> {
        self.run_query_pipeline(
            request,
            QueryProviders {
                embedding: embedding_provider.as_ref(),
                vector_storage: Some(&vector_storage),
                keyword_llm: llm_provider.clone(),
                answer_llm: llm_provider,
            },
        )
        .await
    }
}
