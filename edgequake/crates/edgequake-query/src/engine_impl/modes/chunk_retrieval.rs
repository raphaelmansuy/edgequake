//! Shared chunk retrieval for local/global query modes.

use std::collections::HashSet;
use std::sync::Arc;

use edgequake_storage::traits::{MetadataFilter, VectorStorage};

use crate::context::{QueryContext, RetrievedChunk};
use crate::engine_impl::{QueryEngine, QueryEngineConfig};
use crate::error::Result;
use crate::helpers::build_chunk_from_result;

#[allow(clippy::too_many_arguments)] // retrieval pipeline mirrors QueryEngine workspace arity
pub(super) async fn append_score_ranked_chunks(
    engine: &QueryEngine,
    context: &QueryContext,
    query_text: &str,
    query_embedding: &[f32],
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    vector_storage: &Arc<dyn VectorStorage>,
    retrieval_config: &QueryEngineConfig,
    workspace_mf: Option<&MetadataFilter>,
    log_label: &str,
) -> Result<Vec<RetrievedChunk>> {
    let mut chunk_ids = HashSet::new();

    for entity in &context.entities {
        for chunk_id in &entity.source_chunk_ids {
            chunk_ids.insert(chunk_id.clone());
        }
    }

    for rel in &context.relationships {
        if let Some(chunk_id) = &rel.source_chunk_id {
            chunk_ids.insert(chunk_id.clone());
        }
    }

    tracing::info!(
        total_chunk_ids = chunk_ids.len(),
        entity_count = context.entities.len(),
        relationship_count = context.relationships.len(),
        log_label,
        "OODA-230: chunk collection (workspace)"
    );

    if chunk_ids.is_empty() {
        return Ok(Vec::new());
    }

    let chunk_ids_vec: Vec<String> = chunk_ids.into_iter().collect();

    tracing::debug!(
        chunk_ids_count = chunk_ids_vec.len(),
        max_chunks = retrieval_config.max_chunks,
        log_label,
        "OODA-231: Requesting chunks by ID from vector storage (score-ranked)"
    );

    let results = vector_storage
        .query_filtered(
            query_embedding,
            retrieval_config.max_chunks,
            Some(&chunk_ids_vec),
            workspace_mf,
        )
        .await?;

    tracing::debug!(
        candidates = chunk_ids_vec.len(),
        returned = results.len(),
        log_label,
        "OODA-231: Chunk retrieval result (top-k by cosine similarity)"
    );

    let mf_chunk = MetadataFilter::from_tenant_workspace_type(tenant_id, workspace_id, "chunk");

    let mut chunks = if crate::sparse_retrieval::bm25_retrieval_enabled(retrieval_config) {
        crate::sparse_retrieval::fuse_vector_and_bm25_chunks(
            query_text,
            &results,
            vector_storage,
            mf_chunk.as_ref(),
            engine.reranker.as_deref(),
            engine.kv_storage.as_deref(),
            retrieval_config,
        )
        .await
    } else {
        results
            .iter()
            .filter(|r| r.score >= retrieval_config.min_score)
            .take(retrieval_config.max_chunks)
            .map(build_chunk_from_result)
            .collect()
    };

    crate::chunk_hydration::hydrate_retrieved_chunks(engine.kv_storage.as_deref(), &mut chunks)
        .await;

    Ok(chunks)
}
