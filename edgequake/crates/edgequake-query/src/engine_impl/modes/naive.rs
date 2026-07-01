//! Naive query mode — direct chunk vector search.

use std::sync::Arc;

use crate::context::QueryContext;
use crate::error::Result;
use crate::helpers::build_chunk_from_result;

use edgequake_storage::traits::VectorStorage;

use super::super::{QueryEmbeddings, QueryEngine};
use super::make_scope_metadata_filter;

impl QueryEngine {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine_impl) async fn query_naive_with_vector_storage(
        &self,
        query_text: &str,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        // Document IDs to restrict vector search to (SPEC-031 Tier 1 pre-filter).
        allowed_document_ids: Option<&[String]>,
        vector_storage: &Arc<dyn VectorStorage>,
        max_chunks: usize,
    ) -> Result<QueryContext> {
        let retrieval_config = self.config_with_max_chunks(max_chunks);
        let mut context = QueryContext::new();

        // SPEC-031: push document scope filter to SQL layer — type=chunk AND
        // optionally document_ids (Tier 1 pre-filter)
        let mf = make_scope_metadata_filter(
            tenant_id,
            workspace_id,
            allowed_document_ids,
            Some("chunk"),
        );

        let candidate_k = retrieval_config
            .max_chunks
            .saturating_mul(retrieval_config.bm25_candidate_multiplier);

        let results = vector_storage
            .query_filtered(&embeddings.query, candidate_k, None, mf.as_ref())
            .await?;

        if crate::sparse_retrieval::bm25_retrieval_enabled(&retrieval_config) {
            let mut chunks = crate::sparse_retrieval::fuse_vector_and_bm25_chunks(
                query_text,
                &results,
                vector_storage,
                mf.as_ref(),
                self.reranker.as_deref(),
                self.kv_storage.as_deref(),
                &retrieval_config,
            )
            .await;
            crate::chunk_hydration::hydrate_retrieved_chunks(
                self.kv_storage.as_deref(),
                &mut chunks,
            )
            .await;
            for chunk in chunks {
                context.add_chunk(chunk);
            }
            return Ok(context);
        }

        let mut raw_chunks: Vec<_> = results
            .iter()
            .filter(|r| r.score >= retrieval_config.min_score)
            .take(retrieval_config.max_chunks)
            .map(build_chunk_from_result)
            .collect();
        crate::chunk_hydration::hydrate_retrieved_chunks(
            self.kv_storage.as_deref(),
            &mut raw_chunks,
        )
        .await;
        for chunk in raw_chunks {
            context.add_chunk(chunk);
        }

        Ok(context)
    }
}
