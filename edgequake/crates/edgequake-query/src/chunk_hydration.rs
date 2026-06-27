//! Hydrate retrieved chunks from KV when vector metadata omits inline content (SPEC-024 2.5).

use edgequake_storage::chunk_content::{batch_fetch_chunk_contents, content_from_metadata_or_kv};
use edgequake_storage::traits::KVStorage;

use crate::context::RetrievedChunk;

/// Fill empty chunk bodies from KV (no-op when content already present).
pub async fn hydrate_retrieved_chunks(kv: Option<&dyn KVStorage>, chunks: &mut [RetrievedChunk]) {
    let Some(kv) = kv else {
        return;
    };

    let missing: Vec<String> = chunks
        .iter()
        .filter(|c| c.content.is_empty())
        .map(|c| c.id.clone())
        .collect();

    if missing.is_empty() {
        return;
    }

    let Ok(contents) = batch_fetch_chunk_contents(kv, &missing).await else {
        return;
    };

    for chunk in chunks.iter_mut() {
        if chunk.content.is_empty() {
            if let Some(text) = contents.get(&chunk.id) {
                chunk.content = text.clone();
            }
        }
    }
}

/// Resolve document strings for BM25 reranking (metadata legacy + KV fallback).
pub async fn chunk_documents_for_rerank(
    kv: Option<&dyn KVStorage>,
    vector_results: &[edgequake_storage::traits::VectorSearchResult],
) -> Vec<String> {
    let ids: Vec<String> = vector_results.iter().map(|r| r.id.clone()).collect();
    let kv_map = if let Some(kv) = kv {
        batch_fetch_chunk_contents(kv, &ids)
            .await
            .unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };

    vector_results
        .iter()
        .map(|r| content_from_metadata_or_kv(&r.metadata, kv_map.get(&r.id).map(String::as_str)))
        .collect()
}
