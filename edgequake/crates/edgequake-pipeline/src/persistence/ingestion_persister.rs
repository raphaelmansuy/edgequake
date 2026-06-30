//! SPEC-021 P-G2 — single persistence path for chunk vectors + graph merge (RC-7).
//!
//! Both `edgequake-core::orchestrator::ingestion` and
//! `edgequake-api::processor::text_insert` delegate here so the cross-store
//! sequence cannot diverge (P-G2b config SSOT).

use std::sync::Arc;

use async_trait::async_trait;
use edgequake_llm::LLMProvider;
use edgequake_storage::{compensation, traits::KVStorage, GraphStorage, VectorStorage};

use crate::merger::{
    KnowledgeGraphMerger, MergeProgressCallback, MergeStats, MergerConfig, RelationalEntitySink,
};
use crate::pipeline::ProcessingResult;
use crate::summarizer::{LLMSummarizer, SummarizerConfig};
use crate::Result;

/// Ingestion persistence port (P-G2d / DIP). Callers depend on this trait, not
/// storage details.
#[async_trait]
pub trait IngestionPersister: Send + Sync {
    async fn persist(
        &self,
        ctx: &IngestionPersistContext,
        result: &ProcessingResult,
        chunk_options: ChunkVectorBuildOptions,
    ) -> Result<IngestionPersistOutput>;
}

/// Default production persister — wraps graph + vector stores + merger config.
pub struct DefaultIngestionPersister {
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    config: IngestionPersistConfig,
}

impl DefaultIngestionPersister {
    pub fn new(
        graph_storage: Arc<dyn GraphStorage>,
        vector_storage: Arc<dyn VectorStorage>,
        config: IngestionPersistConfig,
    ) -> Self {
        Self {
            graph_storage,
            vector_storage,
            config,
        }
    }

    /// DRY factory — orchestrator and processor must use this (P-G2b SSOT).
    pub fn from_settings(
        graph_storage: Arc<dyn GraphStorage>,
        vector_storage: Arc<dyn VectorStorage>,
        settings: IngestionPersistSettings,
        relational_sink: Arc<dyn RelationalEntitySink>,
        llm_provider: Option<Arc<dyn LLMProvider>>,
        kv_storage: Option<Arc<dyn KVStorage>>,
    ) -> Self {
        Self::new(
            graph_storage,
            vector_storage,
            IngestionPersistConfig::from_settings(settings, relational_sink, llm_provider)
                .with_kv_storage(kv_storage),
        )
    }

    /// Attach a merge progress callback (SPEC-032 W-04).
    pub fn with_merge_progress(mut self, cb: MergeProgressCallback) -> Self {
        self.config = self.config.with_merge_progress(cb);
        self
    }

    /// Attach a lineage sink (SPEC-032 W-08).
    pub fn with_lineage_sink(mut self, sink: Arc<dyn crate::merger::LineageSink>) -> Self {
        self.config.lineage_sink = Some(sink);
        self
    }
}

#[async_trait]
impl IngestionPersister for DefaultIngestionPersister {
    async fn persist(
        &self,
        ctx: &IngestionPersistContext,
        result: &ProcessingResult,
        chunk_options: ChunkVectorBuildOptions,
    ) -> Result<IngestionPersistOutput> {
        persist_processing_result_impl(
            self.graph_storage.clone(),
            self.vector_storage.clone(),
            &self.config,
            ctx,
            result,
            chunk_options,
        )
        .await
    }
}

/// Tenant/workspace scope for vector metadata and merger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestionPersistContext {
    pub document_id: String,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    /// Optional vector metadata (e.g. `"injection"` for SPEC-0002 citation filtering).
    pub source_type: Option<String>,
    /// Optional vector metadata path label (e.g. `"injection"`).
    pub source_file_path: Option<String>,
}

impl IngestionPersistContext {
    pub fn new(
        document_id: impl Into<String>,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Self {
        Self {
            document_id: document_id.into(),
            tenant_id,
            workspace_id,
            source_type: None,
            source_file_path: None,
        }
    }

    /// Attach optional source metadata written into chunk vector rows.
    pub fn with_source_metadata(
        mut self,
        source_type: Option<String>,
        source_file_path: Option<String>,
    ) -> Self {
        self.source_type = source_type;
        self.source_file_path = source_file_path;
        self
    }
}

/// Shared knobs every ingestion caller must agree on (P-G2b SSOT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestionPersistSettings {
    pub use_llm_summarization: bool,
}

impl Default for IngestionPersistSettings {
    fn default() -> Self {
        Self {
            use_llm_summarization: MergerConfig::default().use_llm_summarization,
        }
    }
}

/// Chunk vector metadata options — all production paths use the same shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkVectorBuildOptions {
    pub include_lineage_metadata: bool,
}

impl ChunkVectorBuildOptions {
    /// SSOT: include chunk position fields for citation / lineage parity.
    pub const STANDARD: Self = Self {
        include_lineage_metadata: true,
    };
}

impl Default for ChunkVectorBuildOptions {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// Merger + relational sink configuration (shared by all callers).
#[derive(Clone)]
pub struct IngestionPersistConfig {
    pub merger_config: MergerConfig,
    pub relational_sink: Arc<dyn RelationalEntitySink>,
    pub llm_provider: Option<Arc<dyn LLMProvider>>,
    /// When set, chunk text is written to KV before vector upsert (SPEC-024 2.5 SSOT).
    pub kv_storage: Option<Arc<dyn KVStorage>>,
    /// Optional per-phase merge progress callback (SPEC-032 W-04).
    pub merge_progress: Option<std::sync::Arc<MergeProgressCallback>>,
    /// Optional lineage sink (SPEC-032 W-08).
    pub lineage_sink: Option<Arc<dyn crate::merger::LineageSink>>,
}

impl IngestionPersistConfig {
    /// Build config from shared settings — orchestrator and processor must use this.
    pub fn from_settings(
        settings: IngestionPersistSettings,
        relational_sink: Arc<dyn RelationalEntitySink>,
        llm_provider: Option<Arc<dyn LLMProvider>>,
    ) -> Self {
        Self {
            merger_config: MergerConfig {
                use_llm_summarization: settings.use_llm_summarization,
                ..Default::default()
            },
            relational_sink,
            llm_provider,
            kv_storage: None,
            merge_progress: None,
            lineage_sink: None,
        }
    }

    pub fn with_kv_storage(mut self, kv_storage: Option<Arc<dyn KVStorage>>) -> Self {
        self.kv_storage = kv_storage;
        self
    }

    /// Wire a merge progress callback. The callback is cloned for each persist call.
    pub fn with_merge_progress(mut self, cb: MergeProgressCallback) -> Self {
        self.merge_progress = Some(std::sync::Arc::new(cb));
        self
    }
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
            let metadata = crate::chunk_storage::build_chunk_vector_metadata(chunk, ctx, options);
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
    persist_processing_result_impl(
        graph_storage,
        vector_storage,
        config,
        ctx,
        result,
        chunk_options,
    )
    .await
}

async fn persist_processing_result_impl(
    graph_storage: Arc<dyn GraphStorage>,
    vector_storage: Arc<dyn VectorStorage>,
    config: &IngestionPersistConfig,
    ctx: &IngestionPersistContext,
    result: &ProcessingResult,
    chunk_options: ChunkVectorBuildOptions,
) -> Result<IngestionPersistOutput> {
    if let Some(kv) = &config.kv_storage {
        let records = crate::chunk_storage::build_chunk_kv_records(
            &ctx.document_id,
            ctx.source_file_path.as_deref(),
            result,
        );
        if !records.is_empty() {
            kv.upsert(&records)
                .await
                .map_err(crate::error::PipelineError::StorageError)?;
        }
    }

    let chunk_vectors = build_chunk_vector_batch(result, ctx, chunk_options);
    let chunk_vector_ids: Vec<String> = chunk_vectors.iter().map(|(id, _, _)| id.clone()).collect();

    if !chunk_vectors.is_empty() {
        vector_storage
            .upsert(&chunk_vectors)
            .await
            .map_err(crate::error::PipelineError::StorageError)?;
    }

    let mut merger = KnowledgeGraphMerger::new(
        config.merger_config.clone(),
        graph_storage.clone(),
        vector_storage.clone(),
    )
    .with_tenant_context(ctx.tenant_id.clone(), ctx.workspace_id.clone())
    .with_relational_sink(config.relational_sink.clone());

    // Wire lineage sink if provided (SPEC-032 W-08)
    if let Some(ref ls) = config.lineage_sink {
        merger = merger.with_lineage_sink(ls.clone());
    }

    if config.merger_config.use_llm_summarization {
        if let Some(llm) = config.llm_provider.clone() {
            let summarizer = Arc::new(LLMSummarizer::new(llm, SummarizerConfig::default()));
            merger = merger.with_summarizer(summarizer);
        }
    }

    let merge_result = merger
        .merge_with_progress(
            result.extractions.clone(),
            config.merge_progress.as_ref().map(|cb| cb.as_ref()),
        )
        .await;

    match merge_result {
        Ok(stats) if stats.errors == 0 => {
            // SPEC-034 IMP-06: Fire community index refresh as a background task.
            //
            // WHY: Community index refresh is a read-model rebuild that does not
            // affect the correctness of the current persist operation. Blocking
            // the persist hot-path on it adds latency spikes with no caller
            // benefit. Errors are logged but cannot surface to the caller (the
            // community index is regenerable from graph data at any time).
            {
                let gs = graph_storage.clone();
                let ws = ctx.workspace_id.clone();
                tokio::spawn(async move {
                    edgequake_storage::schedule_community_index_refresh(gs, ws).await;
                });
            }
            Ok(IngestionPersistOutput {
                chunk_vector_ids,
                merge_stats: stats,
            })
        }
        Ok(stats) => {
            let cause = format!(
                "{} knowledge-graph merge error(s) during persist",
                stats.errors
            );
            compensate_merge_failure(
                graph_storage.as_ref(),
                vector_storage.as_ref(),
                ctx,
                &chunk_vector_ids,
                &stats.artifacts,
                &cause,
            )
            .await;
            Err(crate::error::PipelineError::GraphError(cause))
        }
        Err(merge_err) => {
            let cause = merge_err.to_string();
            compensate_merge_failure(
                graph_storage.as_ref(),
                vector_storage.as_ref(),
                ctx,
                &chunk_vector_ids,
                &crate::merger::MergeArtifacts::default(),
                &cause,
            )
            .await;
            Err(merge_err)
        }
    }
}

async fn compensate_merge_failure(
    graph_storage: &dyn GraphStorage,
    vector_storage: &dyn VectorStorage,
    ctx: &IngestionPersistContext,
    chunk_vector_ids: &[String],
    artifacts: &crate::merger::MergeArtifacts,
    cause: &str,
) {
    compensation::compensate_merge_failure(
        graph_storage,
        vector_storage,
        &ctx.document_id,
        chunk_vector_ids,
        &artifacts.entity_vector_ids,
        &artifacts.relationship_vector_ids,
        &artifacts.graph_nodes_created,
        &artifacts.graph_edges_created,
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
        GraphStorageReadOps, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
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
            section: None,
            page_start: None,
            page_end: None,
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
    async fn persist_writes_chunk_kv_when_configured() {
        let graph = Arc::new(MemoryGraphStorage::new("kv-test"));
        let vector = Arc::new(MemoryVectorStorage::new("kv-test", 4));
        let kv = Arc::new(MemoryKVStorage::new("kv-test"));
        vector.initialize().await.unwrap();

        let config = IngestionPersistConfig::from_settings(
            IngestionPersistSettings::default(),
            Arc::new(crate::merger::NoopEntitySink),
            None,
        )
        .with_kv_storage(Some(kv.clone()));

        persist_processing_result(
            graph,
            vector,
            &config,
            &IngestionPersistContext::new("doc1", None, None),
            &sample_result(),
            ChunkVectorBuildOptions::STANDARD,
        )
        .await
        .expect("persist");

        let chunk_kv = kv.get_by_id("doc1-chunk-0").await.unwrap();
        assert!(
            chunk_kv.is_some(),
            "chunk text must live in KV when configured"
        );
        assert_eq!(
            chunk_kv
                .as_ref()
                .and_then(|v| v.get("content"))
                .and_then(|c| c.as_str()),
            Some("Sarah Chen leads EdgeQuake.")
        );
    }

    #[tokio::test]
    async fn persist_writes_chunk_vectors_and_graph_nodes() {
        let graph = Arc::new(MemoryGraphStorage::new("test"));
        let vector = Arc::new(MemoryVectorStorage::new("test", 4));
        vector.initialize().await.unwrap();

        let config = IngestionPersistConfig::from_settings(
            IngestionPersistSettings::default(),
            Arc::new(crate::merger::NoopEntitySink),
            None,
        );

        let out = persist_processing_result(
            graph.clone(),
            vector.clone(),
            &config,
            &IngestionPersistContext::new("doc1", None, None),
            &sample_result(),
            ChunkVectorBuildOptions::STANDARD,
        )
        .await
        .expect("persist");

        assert_eq!(out.chunk_vector_ids.len(), 1);
        assert!(out.merge_stats.entities_created + out.merge_stats.entities_updated > 0);
        assert!(vector.get_by_id("doc1-chunk-0").await.unwrap().is_some());
        assert!(graph.get_node("SARAH_CHEN").await.unwrap().is_some());
    }

    #[test]
    fn config_from_settings_is_deterministic() {
        let settings = IngestionPersistSettings {
            use_llm_summarization: false,
        };
        let sink: Arc<dyn RelationalEntitySink> = Arc::new(crate::merger::NoopEntitySink);
        let a = IngestionPersistConfig::from_settings(settings, sink.clone(), None);
        let b = IngestionPersistConfig::from_settings(settings, sink, None);
        assert_eq!(
            a.merger_config.use_llm_summarization,
            b.merger_config.use_llm_summarization
        );
        assert!(!a.merger_config.use_llm_summarization);
    }
}
