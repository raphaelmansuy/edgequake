//! Hybrid query mode — LightRAG round-robin merge of local, global, and naive arms.

use std::sync::Arc;

use crate::context::QueryContext;
use crate::error::Result;
use crate::keywords::ExtractedKeywords;

use edgequake_storage::traits::VectorStorage;

use super::super::{QueryEmbeddings, QueryEngine};

impl QueryEngine {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::engine_impl) async fn query_hybrid_with_vector_storage(
        &self,
        query_text: &str,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
        // Document IDs to restrict vector search to (SPEC-031 Tier 1 pre-filter).
        allowed_document_ids: Option<&[String]>,
        vector_storage: &Arc<dyn VectorStorage>,
        max_chunks: usize,
    ) -> Result<QueryContext> {
        let (local_context, global_context, naive_context) = tokio::join!(
            self.query_local_with_vector_storage(
                query_text,
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                allowed_document_ids,
                vector_storage,
                max_chunks,
            ),
            self.query_global_with_vector_storage(
                query_text,
                keywords,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                allowed_document_ids,
                vector_storage,
                max_chunks,
            ),
            self.query_naive_with_vector_storage(
                query_text,
                embeddings,
                tenant_id.clone(),
                workspace_id.clone(),
                allowed_document_ids,
                vector_storage,
                max_chunks,
            ),
        );

        let local_context = local_context?;
        let global_context = global_context?;
        let naive_context = naive_context?;

        let fusion_mode = crate::hybrid_merge::hybrid_fusion_mode_from_env();
        tracing::debug!(
            naive_chunks = naive_context.chunks.len(),
            local_chunks = local_context.chunks.len(),
            local_entities = local_context.entities.len(),
            global_chunks = global_context.chunks.len(),
            global_entities = global_context.entities.len(),
            ?fusion_mode,
            max_chunks,
            "Hybrid merge: LightRAG-style (local, global, naive)"
        );

        let merged = crate::hybrid_merge::merge_hybrid_contexts(
            local_context,
            global_context,
            naive_context,
            max_chunks,
        );

        tracing::debug!(
            merged_chunks = merged.chunks.len(),
            merged_entities = merged.entities.len(),
            merged_relationships = merged.relationships.len(),
            "Hybrid merge complete"
        );

        Ok(merged)
    }
}
