//! Core query entry points using the engine's default vector storage.
//!
//! Contains: `query`, `query_with_embedding_provider`, `query_with_llm_provider`,
//! and `get_context`.

use std::sync::Arc;

use crate::context::QueryContext;
use crate::error::Result;
use crate::modes::QueryMode;

use super::super::SOTAQueryEngine;
use super::query_pipeline::QueryProviders;

impl SOTAQueryEngine {
    /// Execute a query with full SOTA pipeline (default embedding + vector storage).
    ///
    /// @implements FEAT0109 (SOTA Query Engine)
    pub async fn query(
        &self,
        request: crate::engine::QueryRequest,
    ) -> Result<crate::engine::QueryResponse> {
        self.run_query_pipeline(
            request,
            QueryProviders {
                embedding: self.embedding_provider.as_ref(),
                vector_storage: None,
                keyword_llm: None,
                answer_llm: None,
            },
        )
        .await
    }

    /// Execute a query with a workspace-specific embedding provider (default vector storage).
    ///
    /// @implements SPEC-032: Workspace-specific embedding in query process
    pub async fn query_with_embedding_provider(
        &self,
        request: crate::engine::QueryRequest,
        embedding_provider: Arc<dyn crate::EmbeddingProvider>,
    ) -> Result<crate::engine::QueryResponse> {
        self.run_query_pipeline(
            request,
            QueryProviders {
                embedding: embedding_provider.as_ref(),
                vector_storage: None,
                keyword_llm: None,
                answer_llm: None,
            },
        )
        .await
    }

    /// Execute a query with an LLM provider override for keyword extraction and answer generation.
    ///
    /// @implements SPEC-032: Provider selection at query time
    pub async fn query_with_llm_provider(
        &self,
        request: crate::engine::QueryRequest,
        llm_provider: Arc<dyn crate::LLMProvider>,
    ) -> Result<crate::engine::QueryResponse> {
        let llm = llm_provider.clone();
        self.run_query_pipeline(
            request,
            QueryProviders {
                embedding: self.embedding_provider.as_ref(),
                vector_storage: None,
                keyword_llm: Some(llm_provider),
                answer_llm: Some(llm),
            },
        )
        .await
    }

    /// Get the retrieved context without generating an answer.
    ///
    /// Useful for streaming scenarios where context is sent first.
    pub async fn get_context(
        &self,
        request: &crate::engine::QueryRequest,
    ) -> Result<(QueryContext, QueryMode)> {
        let (context, mode) = self
            .run_context_pipeline(
                request,
                QueryProviders {
                    embedding: self.embedding_provider.as_ref(),
                    vector_storage: None,
                    keyword_llm: None,
                    answer_llm: None,
                },
            )
            .await?;
        Ok((self.enrich_retrieved_context(request, context).await, mode))
    }
}
