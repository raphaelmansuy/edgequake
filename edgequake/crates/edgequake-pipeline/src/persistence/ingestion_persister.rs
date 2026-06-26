//! SPEC-021 P-G2 — single persistence path for chunk vectors + graph merge (RC-7).
//!
//! Both `edgequake-core::orchestrator::ingestion` and
//! `edgequake-api::processor::text_insert` delegate here so the 8-step
//! cross-store sequence cannot diverge.

use std::sync::Arc;

use edgequake_llm::LLMProvider;
use edgequake_storage::{compensation, GraphStorage, VectorStorage};
use serde_json::json;

use crate::merger::{
    KnowledgeGraphMerger, MergeStats, MergerConfig, RelationalEntitySink,
};
use crate::pipeline::ProcessingResult;
use crate::summarizer::{LLMSummarizer, SummarizerConfig};
use crate::Result;

/// Tenant/workspace scope for vector metadata and merger.
#[derive(Debug, Clone)]
pub struct IngestionPersistContext {
    pub document_id: String,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
}

/// Whether to include chunk position fields in vector metadata (processor path).
#[derive(Debug, Clone, Copy, Default)]
pub struct ChunkVectorBuildOptions {
    pub include_lineage_metadata: bool,
}

/// Merger + relational sink configuration (shared by all callers).
#[derive(Clone)]
pub struct IngestionPersistConfig {
    pub merger_config: MergerConfig,
    pub relational_sink: Arc<dyn RelationalEntitySink>,
    pub llm_provider: Option<Arc<dyn LLMProvider>>,
}

/// Output of a successful persist (chunk IDs for optional external bookkeeping).
#[derive(Debug, Clone)]
pub struct IngestionPersistOutput {
    pub chunk_vector_ids: Vec<String>,
    pub merge_stats: MergeStats,
}

/// Build batched chunk vector upsert entries from a processing result.
pub fn build_chunk_vector_batch(
    result: &ProcessingResult,
    ctx: &IngestionPersistContext,
    options: ChunkVectorBuildOptions,
) -> Vec<(String, Vec<f32>, serde_json::Value)> {
    result
        .chunks
        .iter()
        .filter_map(|chunk| {
            let embedding = chunk.embedding.as_ref()?;
            let mut metadata = json!({
                "type": "chunk",
                "document_id": ctx.document_id,
                "index": chunk.index,
                "content": chunk.content,
            });

            if options.include_lineage_metadata {
                metadata["start_line"] = json!(chunk.start_line);
                metadata["end_line"] = json!(chunk.end_line);
                metadata["start_offset"] = json!(chunk.start_offset);
                metadata["end_offset"] = json!(chunk.end_offset);
                metadata["token_count"] = json!(chunk.token_count);
            }

            if let Some(tenant_id) = &ctx.tenant_id {
                metadata["tenant_id"] = json!(tenant_id);
            }
            if let Some(workspace_id) = &ctx.workspace_id {
                metadata["workspace_id"] = json!(workspace_id);
            }

            Some((chunk.id.clone(), embedding.clone(), metadata))
        })
        .collect()
}

/// Persist chunk embeddings + merge extractions into graph/entity vectors (P-G2 SSOT).
pub async fn persist_processing_result(
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    config: &IngestionPersistConfig,
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
    chunk_options: ChunkVectorBuildOptions,
) -> Result<IngestionPersistOutput> {
    let chunk_vectors = build_chunk_vector_batch(result, ctx, chunk_options);
    let chunk_vector_ids: Vec<String> = chunk_vectors.iter().map(|(id, _, _)| id.clone()).collect();

    if !chunk_vectors.is_empty() {
        vector_storage
            .upsert(&chunk_vectors)
            .await
            .map_err(|e| crate::error::PipelineError::StorageError(e))?;
    }

    let mut merger =
        KnowledgeGraphMerger::new(
            config.merger_config.clone(),
            graph_storage,
            vector_storage.clone(),
        )
        .with_tenant_context(ctx.tenant_id.clone(), ctx.workspace_id.clone())
        .with_relational_sink(config.relational_sink.clone());

    if config.merger_config.use_llm_summarization {
        if let Some(llm) = config.llm_provider.clone() {
            let summarizer =
                Arc::new(LLMSummarizer::new(llm, SummarizerConfig::default()));
            merger = merger.with_summarizer(summarizer);
        }
    }

    let merge_stats = match merger.merge(result.extractions.clone()).await {
        Ok(stats) if stats.errors == 0 => stats,
        Ok(stats) => {
            let cause = format!(
                "{} knowledge-graph merge error(s) during persist",
                stats.errors
            );
            compensate_orphan_chunk_vectors(
                vector_storage.as_ref(),
                &ctx.document_id,
                &chunk_vector_ids,
                &cause,
            )
            .await;
            return Err(crate::error::PipelineError::GraphError(cause));
        }
        Err(merge_err) => {
            let cause = merge_err.to_string();
            compensate_orphan_chunk_vectors(
                vector_storage.as_ref(),
                &ctx.document_id,
                &chunk_vector_ids,
                &cause,
            )
            .await;
            return Err(merge_err);
        }
    };

    Ok(IngestionPersistOutput {
        chunk_vector_ids,
        merge_stats,
    })
}

async fn compensate_orphan_chunk_vectors(
    vector_storage: &dyn VectorStorage,
    document_id: &str,
    chunk_vector_ids: &[String],
    cause: &str,
) {
    compensation::compensate_orphan_vectors(
        vector_storage,
        document_id,
        chunk_vector_ids,
        &[],
        cause,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::TextChunk;
    use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};
    use edgequake_storage::{
        GraphStorageReadOps, MemoryGraphStorage, MemoryVectorStorage,
    };

    fn sample_result() -> ProcessingResult {
        let chunk = TextChunk {
            id: "doc1-chunk-0".to_string(),
            content: "Sarah Chen leads EdgeQuake.".to_string(),
            index: 0,
            embedding: Some(vec![0.1, 0.2, 0.3, 0.4]),
            start_line: 1,
            end_line: 1,
            start_offset: 0,
            end_offset: 28,
            token_count: 5,
        };
        ProcessingResult {
            document_id: "doc1".to_string(),
            chunks: vec![chunk],
            extractions: vec![ExtractionResult {
                entities: vec![ExtractedEntity::new("Sarah Chen", "PERSON", "Engineer")
                    .with_source_chunk_id("doc1-chunk-0")],
                relationships: vec![ExtractedRelationship::new(
                    "Sarah Chen",
                    "EdgeQuake",
                    "LEADS",
                )],
                source_chunk_id: "doc1-chunk-0".to_string(),
                ..Default::default()
            }],
            stats: Default::default(),
            lineage: None,
        }
    }

    #[tokio::test]
    async fn persist_writes_chunk_vectors_and_graph_nodes() {
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        let vector = Arc::new(MemoryVectorStorage::new("test", 4));
        vector.initialize().await.unwrap();

        let ctx = IngestionPersistContext {
            document_id: "doc1".to_string(),
            tenant_id: None,
            workspace_id: None,
        };
        let config = IngestionPersistConfig {
            merger_config: MergerConfig::default(),
            relational_sink: Arc::new(crate::merger::NoopEntitySink),
            llm_provider: None,
        };

        let out = persist_processing_result(
            graph.clone(),
            vector.clone(),
            &config,
            &ctx,
            &sample_result(),
            ChunkVectorBuildOptions {
                include_lineage_metadata: true,
            },
        )
        .await
        .expect("persist");

        assert_eq!(out.chunk_vector_ids.len(), 1);
        assert!(out.merge_stats.entities_created + out.merge_stats.entities_updated > 0);
        assert!(vector.get_by_id("doc1-chunk-0").await.unwrap().is_some());
        assert!(
            graph
                .get_node("SARAH_CHEN")
                .await
                .unwrap()
                .is_some()
        );
    }
}
