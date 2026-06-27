//! Streaming query entry points (SPEC-017 P1-01).
//!
//! All streaming paths delegate context retrieval to `query_pipeline`, then stream
//! the LLM answer from the enriched context.

use std::sync::Arc;

use futures::StreamExt;

use crate::context::QueryContext;
use crate::error::{QueryError, Result};
use crate::modes::QueryMode;
use edgequake_storage::traits::VectorStorage;

use super::super::QueryEngine;
use super::query_pipeline::QueryProviders;

pub(super) use super::super::TokenStream;

impl QueryEngine {
    /// Legacy v1 streaming: tokens only (uses default providers + shared pipeline).
    pub async fn query_stream(&self, request: crate::types::QueryRequest) -> Result<TokenStream> {
        let (_, _, stream) = self.query_stream_with_context(request).await?;
        Ok(stream)
    }

    /// Streaming query returning context + token stream (default providers).
    pub async fn query_stream_with_context(
        &self,
        request: crate::types::QueryRequest,
    ) -> Result<(QueryContext, QueryMode, TokenStream)> {
        self.stream_with_providers(
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

    /// Streaming query with LLM override for keyword extraction and answer generation.
    pub async fn query_stream_with_context_and_llm(
        &self,
        request: crate::types::QueryRequest,
        llm_provider: Arc<dyn crate::LLMProvider>,
    ) -> Result<(QueryContext, QueryMode, TokenStream)> {
        let llm = llm_provider.clone();
        self.stream_with_providers(
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

    /// Streaming query with full workspace embedding/vector config + optional LLM override.
    pub async fn query_stream_with_full_config(
        &self,
        request: crate::types::QueryRequest,
        embedding_provider: Arc<dyn crate::EmbeddingProvider>,
        vector_storage: Arc<dyn VectorStorage>,
        llm_provider: Option<Arc<dyn crate::LLMProvider>>,
    ) -> Result<(QueryContext, QueryMode, TokenStream)> {
        self.stream_with_providers(
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

    async fn stream_with_providers(
        &self,
        request: crate::types::QueryRequest,
        providers: QueryProviders<'_>,
    ) -> Result<(QueryContext, QueryMode, TokenStream)> {
        let answer_llm = providers.answer_llm.clone();
        let (context, mode) = self.run_context_pipeline(&request, providers).await?;
        let context = self.enrich_retrieved_context(&request, context).await;
        self.stream_answer_from_context(&request, context, mode, answer_llm)
            .await
    }

    async fn stream_answer_from_context(
        &self,
        request: &crate::types::QueryRequest,
        context: QueryContext,
        mode: QueryMode,
        llm_override: Option<Arc<dyn crate::LLMProvider>>,
    ) -> Result<(QueryContext, QueryMode, TokenStream)> {
        // P-G11 (RC-16): vision parity. When the request carries images, use
        // the vision-capable `chat` path (the `stream` trait method cannot carry
        // images). E30: vision-LLm failure falls back to text-only stream inside
        // `stream_vision_answer`.
        if let Some(images) = request.images.as_ref().filter(|i| !i.is_empty()) {
            let stream = self
                .stream_vision_answer(
                    &request.query,
                    &context,
                    llm_override,
                    request.system_prompt.as_deref(),
                    images,
                )
                .await?;
            return Ok((context, mode, stream));
        }

        if context.is_empty() {
            return Ok((
                context,
                mode,
                futures::stream::once(async {
                    Ok("I'm sorry, but I couldn't find any relevant information in my knowledge base to answer your question.".to_string())
                })
                .boxed(),
            ));
        }

        let prompt = self.build_prompt(
            &request.query,
            &context,
            request.system_prompt.as_deref(),
            &request.conversation_history,
        );

        let llm = llm_override.unwrap_or_else(|| self.llm_provider.clone());

        let stream = if llm.supports_streaming() {
            llm.stream(&prompt)
                .await
                .map(|stream| stream.map(|res| res.map_err(QueryError::from)).boxed())
                .map_err(QueryError::from)?
        } else {
            tracing::warn!(
                provider = llm.name(),
                "Provider doesn't support streaming, falling back to non-streaming mode"
            );
            let response = llm.complete(&prompt).await.map_err(QueryError::from)?;
            futures::stream::once(async move { Ok(response.content) }).boxed()
        };

        Ok((context, mode, stream))
    }
}
