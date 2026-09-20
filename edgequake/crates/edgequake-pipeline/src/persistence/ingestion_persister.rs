//! SPEC-021 P-G2 — single persistence path for chunk vectors + graph merge (RC-7).
//!
//! Both `edgequake-core::orchestrator::ingestion` and
//! `edgequake-api::processor::text_insert` delegate here so the cross-store
//! sequence cannot diverge (P-G2b config SSOT).

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use edgequake_llm::LLMProvider;
use edgequake_observability::record_ingest_stage_duration;
use edgequake_storage::{
    chunk_text_authority_from_env, chunk_text_authority_writes_kv,
    chunk_text_authority_writes_relational, compensation, traits::ChunkRepository,
    traits::KVStorage, GraphStorage, TextEmbedder, VectorStorage,
};
use edgequake_storage_contracts::IngestionCommitter;

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

    /// Attach embedder for optional community_report vector indexing (SPEC-046).
    pub fn with_text_embedder(mut self, embedder: Arc<dyn TextEmbedder>) -> Self {
        self.config.text_embedder = Some(embedder);
        self
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

    /// SPEC-091 W1: attach relational chunk repository (gated by authority env flag).
    pub fn with_relational_chunks(
        mut self,
        relational_chunks: Option<Arc<dyn ChunkRepository>>,
    ) -> Self {
        self.config = self.config.with_relational_chunks(relational_chunks);
        self
    }

    /// SPEC-149: attach the atomic authority committer.
    pub fn with_ingestion_committer(
        mut self,
        committer: Option<Arc<dyn IngestionCommitter>>,
    ) -> Self {
        self.config = self.config.with_ingestion_committer(committer);
        self
    }

    /// SPEC-149: explicit durable authority (preferred over Option-pair builders).
    pub fn with_ingestion_authority(mut self, authority: IngestionAuthority) -> Self {
        self.config = self.config.with_ingestion_authority(authority);
        self
    }

    /// SPEC-091 W3: attach typed embedding index.
    pub fn with_typed_embedding_index(
        mut self,
        index: Option<Arc<dyn edgequake_storage::traits::domain::EmbeddingIndex>>,
    ) -> Self {
        self.config = self.config.with_typed_embedding_index(index);
        self
    }

    /// SPEC-091 IW2: attach typed fleet embedding index.
    pub fn with_fleet_embedding_index(
        mut self,
        index: Option<Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>>,
    ) -> Self {
        self.config = self.config.with_fleet_embedding_index(index);
        self
    }

    /// SPEC-091 IP2: attach transactional outbox sink.
    pub fn with_outbox(mut self, outbox: Option<Arc<dyn edgequake_storage::OutboxSink>>) -> Self {
        self.config = self.config.with_outbox(outbox);
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
        edgequake_observability::with_pipeline_stage_span(
            "ingest.persist",
            persist_processing_result_impl(
                self.graph_storage.clone(),
                self.vector_storage.clone(),
                &self.config,
                ctx,
                result,
                chunk_options,
            ),
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
    /// Durable ingest generation. Legacy/memory callers default to generation 1.
    pub ingest_generation: Option<u64>,
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
            ingest_generation: None,
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

    pub fn with_ingest_generation(mut self, generation: u64) -> Self {
        self.ingest_generation = Some(generation);
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

/// How relational chunk/authority persistence is selected for this persist call.
///
/// P0 postgres serving must use [`Self::DurableCommitter`]. Memory/unit tests may
/// use [`Self::LegacyRepository`]. Mixing "optional committer" with silent
/// fallback is forbidden: an incomplete durable wiring fails closed.
#[derive(Clone)]
pub enum IngestionAuthority {
    /// Direct `ChunkRepository::insert_batch` (memory / non-authority tests).
    LegacyRepository(Arc<dyn ChunkRepository>),
    /// Atomic SPEC-149 ingestion committer (required for postgres product serving).
    DurableCommitter {
        committer: Arc<dyn IngestionCommitter>,
        /// Spine repository still required for typed embedding FK resolution.
        relational_chunks: Arc<dyn ChunkRepository>,
        /// Resolved embedding model identity — never read from env in the hot path.
        embedding_model_id: String,
    },
}

impl IngestionAuthority {
    pub fn relational_chunks(&self) -> Arc<dyn ChunkRepository> {
        match self {
            Self::LegacyRepository(repo) => Arc::clone(repo),
            Self::DurableCommitter {
                relational_chunks, ..
            } => Arc::clone(relational_chunks),
        }
    }

    pub fn as_durable_committer(&self) -> Option<&Arc<dyn IngestionCommitter>> {
        match self {
            Self::DurableCommitter { committer, .. } => Some(committer),
            Self::LegacyRepository(_) => None,
        }
    }

    pub fn embedding_model_id(&self) -> Option<&str> {
        match self {
            Self::DurableCommitter {
                embedding_model_id, ..
            } => Some(embedding_model_id.as_str()),
            Self::LegacyRepository(_) => None,
        }
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
    /// SPEC-091 / SPEC-149: explicit authority strategy (no silent Option fallback).
    pub ingestion_authority: Option<IngestionAuthority>,
    /// SPEC-091 W1: relational chunk authority writer (`chunks` table).
    /// Prefer [`Self::ingestion_authority`]; retained for gradual call-site migration.
    pub relational_chunks: Option<Arc<dyn ChunkRepository>>,
    /// SPEC-149: atomic authority for chunks, immutable facts, embeddings,
    /// contribution provenance, and projection event append.
    /// Prefer [`Self::ingestion_authority`]; retained for gradual call-site migration.
    pub ingestion_committer: Option<Arc<dyn IngestionCommitter>>,
    /// SPEC-091 W3: typed chunk embedding index (`chunk_embeddings` table).
    /// Wired only on postgres contexts; dual-write runs during rollout
    /// (warn-only) and becomes authoritative under `chunk_embeddings` backend.
    /// The spine `relational_chunks` repo doubles as the chunk-id resolver.
    pub typed_embedding_index: Option<Arc<dyn edgequake_storage::traits::domain::EmbeddingIndex>>,
    /// SPEC-091 IW2: typed fleet embedding index (entity/relationship/report).
    pub fleet_embedding_index: Option<Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>>,
    /// Optional per-phase merge progress callback (SPEC-032 W-04).
    pub merge_progress: Option<std::sync::Arc<MergeProgressCallback>>,
    /// Optional lineage sink (SPEC-032 W-08).
    pub lineage_sink: Option<Arc<dyn crate::merger::LineageSink>>,
    /// Optional text embedder for community_report vectors (SPEC-046 EQ-046-11).
    pub text_embedder: Option<Arc<dyn TextEmbedder>>,
    /// SPEC-091 IP2: transactional outbox for multi-store milestones (LAW-D3).
    pub outbox: Option<Arc<dyn edgequake_storage::OutboxSink>>,
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
            ingestion_authority: None,
            relational_chunks: None,
            ingestion_committer: None,
            typed_embedding_index: None,
            fleet_embedding_index: None,
            merge_progress: None,
            lineage_sink: None,
            text_embedder: None,
            outbox: None,
        }
    }

    pub fn with_kv_storage(mut self, kv_storage: Option<Arc<dyn KVStorage>>) -> Self {
        self.kv_storage = kv_storage;
        self
    }

    /// SPEC-091 IP2: wire transactional outbox sink.
    pub fn with_outbox(mut self, outbox: Option<Arc<dyn edgequake_storage::OutboxSink>>) -> Self {
        self.outbox = outbox;
        self
    }

    pub fn with_relational_chunks(
        mut self,
        relational_chunks: Option<Arc<dyn ChunkRepository>>,
    ) -> Self {
        self.relational_chunks = relational_chunks.clone();
        if let Some(repo) = relational_chunks {
            if self.ingestion_authority.is_none() && self.ingestion_committer.is_none() {
                self.ingestion_authority = Some(IngestionAuthority::LegacyRepository(repo));
            }
        }
        self
    }

    pub fn with_ingestion_committer(
        mut self,
        committer: Option<Arc<dyn IngestionCommitter>>,
    ) -> Self {
        self.ingestion_committer = committer.clone();
        if let (Some(committer), Some(repo)) = (committer, self.relational_chunks.clone()) {
            let embedding_model_id = std::env::var("EDGEQUAKE_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".into());
            self.ingestion_authority = Some(IngestionAuthority::DurableCommitter {
                committer,
                relational_chunks: repo,
                embedding_model_id,
            });
        }
        self
    }

    /// Explicit authority wiring for P0 postgres (preferred over Option pairs).
    pub fn with_ingestion_authority(mut self, authority: IngestionAuthority) -> Self {
        match &authority {
            IngestionAuthority::LegacyRepository(repo) => {
                self.relational_chunks = Some(Arc::clone(repo));
                self.ingestion_committer = None;
            }
            IngestionAuthority::DurableCommitter {
                committer,
                relational_chunks,
                ..
            } => {
                self.relational_chunks = Some(Arc::clone(relational_chunks));
                self.ingestion_committer = Some(Arc::clone(committer));
            }
        }
        self.ingestion_authority = Some(authority);
        self
    }

    /// SPEC-091 W3: wire the typed embedding index.
    pub fn with_typed_embedding_index(
        mut self,
        index: Option<Arc<dyn edgequake_storage::traits::domain::EmbeddingIndex>>,
    ) -> Self {
        self.typed_embedding_index = index;
        self
    }

    /// SPEC-091 IW2: wire the typed fleet embedding index.
    pub fn with_fleet_embedding_index(
        mut self,
        index: Option<Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>>,
    ) -> Self {
        self.fleet_embedding_index = index;
        self
    }

    /// Wire a merge progress callback. The callback is cloned for each persist call.
    pub fn with_merge_progress(mut self, cb: MergeProgressCallback) -> Self {
        self.merge_progress = Some(std::sync::Arc::new(cb));
        self
    }

    pub fn with_text_embedder(mut self, embedder: Arc<dyn TextEmbedder>) -> Self {
        self.text_embedder = Some(embedder);
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
    edgequake_observability::with_pipeline_stage_span(
        "ingest.persist",
        persist_processing_result_impl(
            graph_storage,
            vector_storage,
            config,
            ctx,
            result,
            chunk_options,
        ),
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
    // SPEC-091: under typed authority, incomplete wiring is fail-closed —
    // never silently skip typed chunk / fleet writes while legacy is stopped.
    let typed_authority = edgequake_storage::vector_backend::vector_backend_reads_typed(
        edgequake_storage::vector_backend_from_env(),
    );
    if typed_authority {
        let has_chunk_embeddings = result.chunks.iter().any(|c| c.embedding.is_some());
        if has_chunk_embeddings
            && (config.typed_embedding_index.is_none() || config.relational_chunks.is_none())
        {
            return Err(crate::error::PipelineError::StorageError(
                edgequake_storage::error::StorageError::Database(
                    "SPEC-091: typed vector backend requires typed_embedding_index and \
                     relational_chunks when chunk embeddings are present"
                        .into(),
                ),
            ));
        }
        let has_entity_or_rel_embeddings = result.extractions.iter().any(|ex| {
            ex.entities.iter().any(|e| e.embedding.is_some())
                || ex.relationships.iter().any(|r| r.embedding.is_some())
        });
        if has_entity_or_rel_embeddings && config.fleet_embedding_index.is_none() {
            return Err(crate::error::PipelineError::StorageError(
                edgequake_storage::error::StorageError::Database(
                    "SPEC-091: typed vector backend requires fleet_embedding_index when \
                     entity/relationship embeddings are present"
                        .into(),
                ),
            ));
        }
    }

    let authority = chunk_text_authority_from_env();
    let committed_relational = if chunk_text_authority_writes_relational(authority) {
        match config.ingestion_authority.as_ref() {
            Some(IngestionAuthority::DurableCommitter {
                committer,
                embedding_model_id,
                ..
            }) => {
                let chunks = super::relational_chunk_writer::build_relational_chunks(ctx, result)
                    .map_err(crate::error::PipelineError::StorageError)?;
                let generation = ctx.ingest_generation.ok_or_else(|| {
                    crate::error::PipelineError::StorageError(
                        edgequake_storage::error::StorageError::InvalidData(
                            "durable ingest requires IngestionPersistContext.ingest_generation"
                                .into(),
                        ),
                    )
                })?;
                let command = super::relational_chunk_writer::build_prepared_ingestion_batch(
                    ctx,
                    result,
                    &chunks,
                    embedding_model_id,
                    generation,
                )
                .map_err(crate::error::PipelineError::StorageError)?;
                let stage_start = Instant::now();
                committer
                    .commit_batch(&command)
                    .await
                    .map_err(edgequake_storage::StorageError::from)
                    .map_err(crate::error::PipelineError::StorageError)?;
                record_ingest_stage_duration(
                    "relational_commit_batch",
                    stage_start.elapsed().as_secs_f64(),
                );
                true
            }
            Some(IngestionAuthority::LegacyRepository(_)) => false,
            None => {
                // Leftover public Option fields must not silently commit.
                if config.ingestion_committer.is_some() {
                    return Err(crate::error::PipelineError::StorageError(
                        edgequake_storage::error::StorageError::InvalidData(
                            "ingestion_committer set without IngestionAuthority::DurableCommitter; \
                             use with_ingestion_authority"
                                .into(),
                        ),
                    ));
                }
                false
            }
        }
    } else {
        false
    };

    let mut chunk_kv_ids: Vec<String> = Vec::new();
    if chunk_text_authority_writes_kv(authority) {
        if let Some(kv) = &config.kv_storage {
            let records = crate::chunk_storage::build_chunk_kv_records(
                &ctx.document_id,
                ctx.source_file_path.as_deref(),
                result,
            );
            if !records.is_empty() {
                chunk_kv_ids = records.iter().map(|(id, _)| id.clone()).collect();
                let stage_start = Instant::now();
                kv.upsert(&records)
                    .await
                    .map_err(crate::error::PipelineError::StorageError)?;
                record_ingest_stage_duration("kv_upsert", stage_start.elapsed().as_secs_f64());
            }
        }
    }

    if chunk_text_authority_writes_relational(authority) && !committed_relational {
        if let Some(repo) = &config.relational_chunks {
            let stage_start = Instant::now();
            super::relational_chunk_writer::persist_relational_chunks(repo.as_ref(), ctx, result)
                .await
                .map_err(crate::error::PipelineError::StorageError)?;
            record_ingest_stage_duration(
                "relational_chunk_upsert",
                stage_start.elapsed().as_secs_f64(),
            );
        }
    }

    let chunk_vectors = build_chunk_vector_batch(result, ctx, chunk_options);
    let chunk_vector_ids: Vec<String> = chunk_vectors.iter().map(|(id, _, _)| id.clone()).collect();

    // SPEC-091 IP0: under typed authority legacy eq_*_vectors is write-stopped —
    // skip the adapter call entirely (no dead RTT / stage noise).
    if !chunk_vectors.is_empty() {
        if edgequake_storage::legacy_vector_writes_stopped() {
            record_ingest_stage_duration("chunk_vector_upsert_skipped", 0.0);
            tracing::debug!(
                document_id = %ctx.document_id,
                count = chunk_vectors.len(),
                "SPEC-091 IP0: skipped legacy chunk vector upsert (typed authority)"
            );
        } else {
            let stage_start = Instant::now();
            vector_storage
                .upsert(&chunk_vectors)
                .await
                .map_err(crate::error::PipelineError::StorageError)?;
            // SPEC-060: ingest chunk vector stage histogram
            record_ingest_stage_duration(
                "chunk_vector_upsert",
                stage_start.elapsed().as_secs_f64(),
            );
        }
    }

    // SPEC-091 W3: dual-write typed chunk embeddings. Always runs when wired
    // (rollout shadow) so the engine backfill only has to cover pre-existing
    // legacy rows; warn-only under `legacy_tables` (rollback escape hatch),
    // fail-closed under `chunk_embeddings` (typed is the authority there). The
    // relational spine repo resolves `chunks.id` for the typed FK (LD-02).
    if let (Some(index), Some(repo)) = (&config.typed_embedding_index, &config.relational_chunks) {
        let stage_start = Instant::now();
        let typed_result = super::typed_embedding_writer::persist_typed_chunk_embeddings(
            index.as_ref(),
            repo.as_ref(),
            ctx,
            result,
        )
        .await;
        record_ingest_stage_duration(
            "typed_chunk_embedding_upsert",
            stage_start.elapsed().as_secs_f64(),
        );
        if let Err(e) = typed_result {
            let backend = edgequake_storage::vector_backend_from_env();
            if edgequake_storage::vector_backend::vector_backend_reads_typed(backend) {
                return Err(crate::error::PipelineError::StorageError(e));
            }
            tracing::warn!(
                error = %e,
                document_id = %ctx.document_id,
                "SPEC-091 W3: typed chunk embedding dual-write failed (legacy backend continues)"
            );
        }
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

    // 050 B7: placeholders need the same embedder as community reports.
    if let Some(ref embedder) = config.text_embedder {
        merger = merger.with_text_embedder(embedder.clone());
    }
    if let Some(ref fleet_index) = config.fleet_embedding_index {
        merger = merger.with_fleet_embedding_index(fleet_index.clone());
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
            // SPEC-046 P0.2: emit graph quality from merge delta (cheap, sync).
            let delta_metrics = edgequake_storage::metrics_from_merge_delta(
                &stats.artifacts.graph_nodes_created,
                &stats.artifacts.graph_edges_created,
            );
            edgequake_storage::log_graph_quality(&delta_metrics, Some(ctx.document_id.as_str()));

            // SPEC-034 IMP-06: Fire community index refresh as a background task.
            //
            // WHY: Community index refresh is a read-model rebuild that does not
            // affect the correctness of the current persist operation. Blocking
            // the persist hot-path on it adds latency spikes with no caller
            // benefit. Errors are logged but cannot surface to the caller (the
            // community index is regenerable from graph data at any time).
            //
            // SPEC-046: also collect full-graph quality metrics in the same
            // background task (non-blocking).
            {
                let gs = graph_storage.clone();
                let vs = vector_storage.clone();
                let ws = ctx.workspace_id.clone();
                let tenant = ctx.tenant_id.clone();
                let doc_id = ctx.document_id.clone();
                let embedder = config.text_embedder.clone();
                tokio::spawn(async move {
                    let ws_uuid = ws.as_deref().and_then(|s| uuid::Uuid::parse_str(s).ok());
                    match edgequake_storage::collect_graph_quality_metrics(
                        gs.as_ref(),
                        ws_uuid.as_ref(),
                        500,
                    )
                    .await
                    {
                        Ok(mut m) => {
                            m.workspace_id = ws.clone();
                            edgequake_storage::log_graph_quality(&m, Some(doc_id.as_str()));
                        }
                        Err(e) => {
                            tracing::debug!(
                                error = %e,
                                "SPEC-046: full graph quality collect skipped"
                            );
                        }
                    }
                    let extras = match embedder {
                        Some(e) => {
                            edgequake_storage::CommunityRefreshExtras::with_report_indexing(vs, e)
                                .with_tenant_id(tenant)
                        }
                        None => edgequake_storage::CommunityRefreshExtras::default()
                            .with_tenant_id(tenant),
                    };
                    edgequake_storage::schedule_community_index_refresh_with_extras(gs, ws, extras)
                        .await;
                });
            }
            // SPEC-091 W4/IP2: chunks are query-visible only after vectors + graph
            // merge succeeded → mark serving state `ready` for this document.
            // Warn-only on mark failure; fence default is on (IP2) — backfill
            // reconciles any missed rows.
            let doc_uuid = uuid::Uuid::parse_str(&ctx.document_id).ok();
            let ws = ctx
                .workspace_id
                .as_deref()
                .and_then(|s| uuid::Uuid::parse_str(s).ok());
            if chunk_text_authority_writes_relational(authority) {
                if let (Some(repo), Some(doc_uuid)) = (&config.relational_chunks, doc_uuid) {
                    if let Err(e) = repo
                        .set_serving_state(
                            edgequake_storage::traits::domain::DocumentId::new(doc_uuid),
                            edgequake_storage::serving_fence::SERVING_STATE_READY,
                        )
                        .await
                    {
                        tracing::warn!(
                            error = %e,
                            document_id = %ctx.document_id,
                            "SPEC-091: serving-state ready mark failed (backfill reconciles)"
                        );
                    }
                    edgequake_storage::enqueue_outbox_best_effort(
                        config.outbox.as_deref(),
                        edgequake_storage::OUTBOX_AGGREGATE_DOCUMENT,
                        doc_uuid,
                        edgequake_storage::OUTBOX_EVENT_CHUNK_READY,
                        serde_json::json!({ "document_id": ctx.document_id }),
                        ws,
                    )
                    .await;
                }
            }
            if let Some(doc_uuid) = doc_uuid {
                edgequake_storage::enqueue_outbox_best_effort(
                    config.outbox.as_deref(),
                    edgequake_storage::OUTBOX_AGGREGATE_DOCUMENT,
                    doc_uuid,
                    edgequake_storage::OUTBOX_EVENT_MERGE_DONE,
                    serde_json::json!({
                        "document_id": ctx.document_id,
                        "entities_created": stats.entities_created,
                        "relationships_created": stats.relationships_created,
                    }),
                    ws,
                )
                .await;
            }

            Ok(IngestionPersistOutput {
                chunk_vector_ids,
                merge_stats: stats,
            })
        }
        Ok(stats) => {
            let cause = match stats.first_error.as_deref() {
                Some(detail) => format!(
                    "{} knowledge-graph merge error(s) during persist: {detail}",
                    stats.errors
                ),
                None => format!(
                    "{} knowledge-graph merge error(s) during persist",
                    stats.errors
                ),
            };
            compensate_merge_failure(
                graph_storage.as_ref(),
                vector_storage.as_ref(),
                config.kv_storage.as_deref(),
                (!committed_relational)
                    .then_some(config.relational_chunks.as_deref())
                    .flatten(),
                config.outbox.as_deref(),
                authority,
                ctx,
                &chunk_vector_ids,
                &chunk_kv_ids,
                &stats.artifacts,
                &cause,
            )
            .await;
            Err(crate::error::PipelineError::GraphError(cause))
        }
        Err(merge_err) => {
            // Hard Err is rare after SPEC-057 P3 (vector upserts return Ok+errors
            // with partial artifacts). Keep defensive compensate; prefer Ok(errors)
            // path which carries real MergeArtifacts.
            let cause = merge_err.to_string();
            compensate_merge_failure(
                graph_storage.as_ref(),
                vector_storage.as_ref(),
                config.kv_storage.as_deref(),
                (!committed_relational)
                    .then_some(config.relational_chunks.as_deref())
                    .flatten(),
                config.outbox.as_deref(),
                authority,
                ctx,
                &chunk_vector_ids,
                &chunk_kv_ids,
                &crate::merger::MergeArtifacts::default(),
                &cause,
            )
            .await;
            Err(merge_err)
        }
    }
}

#[allow(clippy::too_many_arguments)] // thin saga wrapper; mirrors compensation::compensate_merge_failure_with_kv
async fn compensate_merge_failure(
    graph_storage: &dyn GraphStorage,
    vector_storage: &dyn VectorStorage,
    kv_storage: Option<&dyn KVStorage>,
    relational_chunks: Option<&dyn edgequake_storage::traits::domain::ChunkRepository>,
    outbox: Option<&dyn edgequake_storage::OutboxSink>,
    authority: edgequake_storage::chunk_text_authority::ChunkTextAuthority,
    ctx: &IngestionPersistContext,
    chunk_vector_ids: &[String],
    chunk_kv_ids: &[String],
    artifacts: &crate::merger::MergeArtifacts,
    cause: &str,
) {
    let stage_start = Instant::now();
    compensation::compensate_merge_failure_with_kv(
        graph_storage,
        vector_storage,
        kv_storage,
        &ctx.document_id,
        chunk_vector_ids,
        chunk_kv_ids,
        &artifacts.entity_vector_ids,
        &artifacts.relationship_vector_ids,
        &artifacts.graph_nodes_created,
        &artifacts.graph_edges_created,
        cause,
    )
    .await;
    // SPEC-091 W1: compensate relational chunk rows written in dual/relational
    // mode (warn-only — a retry conflict-skips, and the backfill reconciles).
    let doc_uuid = uuid::Uuid::parse_str(&ctx.document_id).ok();
    let ws = ctx
        .workspace_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    if edgequake_storage::chunk_text_authority::chunk_text_authority_writes_relational(authority) {
        if let (Some(repo), Some(doc_uuid)) = (relational_chunks, doc_uuid) {
            if let Err(e) = repo
                .delete_for_document(
                    &mut edgequake_storage::traits::domain::UnitOfWork::default(),
                    edgequake_storage::traits::domain::DocumentId::new(doc_uuid),
                )
                .await
            {
                tracing::warn!(
                    error = %e,
                    document_id = %ctx.document_id,
                    "SPEC-091: relational chunk compensation failed (retry conflict-skips)"
                );
            }
        }
    }
    if let Some(doc_uuid) = doc_uuid {
        edgequake_storage::enqueue_outbox_best_effort(
            outbox,
            edgequake_storage::OUTBOX_AGGREGATE_DOCUMENT,
            doc_uuid,
            edgequake_storage::OUTBOX_EVENT_COMPENSATE,
            serde_json::json!({
                "document_id": ctx.document_id,
                "cause": cause,
            }),
            ws,
        )
        .await;
    }
    // SPEC-060: compensate stage histogram
    record_ingest_stage_duration("compensate", stage_start.elapsed().as_secs_f64());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::TextChunk;
    use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};
    use edgequake_storage::{
        GraphStorageReadOps, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
    };
    use serial_test::serial;

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
            modality: None,
        };
        ProcessingResult {
            document_id: "doc1".to_string(),
            chunks: vec![chunk],
            extractions: vec![ExtractionResult {
                entities: vec![ExtractedEntity::new("Sarah Chen", "PERSON", "Engineer")
                    .with_source_chunk_id("doc1-chunk-0")],
                relationships: vec![
                    ExtractedRelationship::new("Sarah Chen", "EdgeQuake", "LEADS")
                        .with_source_chunk_id("doc1-chunk-0"),
                ],
                source_chunk_id: "doc1-chunk-0".to_string(),
                ..Default::default()
            }],
            stats: Default::default(),
            lineage: None,
        }
    }

    #[tokio::test]
    #[serial]
    async fn persist_writes_chunk_kv_when_configured() {
        // SPEC-091 Wave A/D: authority default is `relational` (no KV chunk
        // write) and vector default is typed (fail-closed without typed ports).
        // This test exercises the legacy dual-KV path explicitly.
        std::env::set_var("EDGEQUAKE_CHUNK_TEXT_AUTHORITY", "dual");
        std::env::set_var("EDGEQUAKE_VECTOR_BACKEND", "legacy_tables");
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
        std::env::remove_var("EDGEQUAKE_CHUNK_TEXT_AUTHORITY");
        std::env::remove_var("EDGEQUAKE_VECTOR_BACKEND");

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
    #[serial]
    async fn persist_writes_chunk_vectors_and_graph_nodes() {
        // Legacy path: typed default skips eq_* upsert (IP0). Opt into legacy.
        std::env::set_var("EDGEQUAKE_VECTOR_BACKEND", "legacy_tables");
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
        std::env::remove_var("EDGEQUAKE_VECTOR_BACKEND");
    }

    /// SPEC-091 IP0 / IP-AC-01: typed authority must not call legacy vector upsert.
    #[tokio::test]
    #[serial]
    async fn typed_authority_skips_legacy_chunk_vector_upsert() {
        use async_trait::async_trait;
        use edgequake_storage::traits::domain::{
            EmbeddingCapabilities, EmbeddingIndex, EmbeddingRow, ModelId, ScoredChunk,
            UpsertReport, VectorQuery, WorkspaceId,
        };
        use edgequake_storage::MemoryChunkRepository;

        std::env::set_var("EDGEQUAKE_VECTOR_BACKEND", "typed_embeddings");

        struct NoopEmbeddingIndex;
        #[async_trait]
        impl EmbeddingIndex for NoopEmbeddingIndex {
            fn capabilities(&self) -> EmbeddingCapabilities {
                EmbeddingCapabilities {
                    metric: "cosine",
                    supports_filters: true,
                    supports_rerank: false,
                }
            }
            async fn upsert_batch(
                &self,
                _model: ModelId,
                rows: &[EmbeddingRow],
            ) -> std::result::Result<UpsertReport, edgequake_storage::StorageError> {
                Ok(UpsertReport {
                    upserted: rows.len() as u64,
                    ..Default::default()
                })
            }
            async fn search(
                &self,
                _req: &VectorQuery,
            ) -> std::result::Result<Vec<ScoredChunk>, edgequake_storage::StorageError>
            {
                Ok(vec![])
            }
            async fn delete_for_workspace(
                &self,
                _workspace: WorkspaceId,
            ) -> std::result::Result<u64, edgequake_storage::StorageError> {
                Ok(0)
            }
        }

        let graph = Arc::new(MemoryGraphStorage::new("ip0-skip"));
        let vector = Arc::new(MemoryVectorStorage::new("ip0-skip", 4));
        vector.initialize().await.unwrap();

        let doc_id = uuid::Uuid::new_v4().to_string();
        let ws_id = uuid::Uuid::new_v4().to_string();
        let config = IngestionPersistConfig::from_settings(
            IngestionPersistSettings::default(),
            Arc::new(crate::merger::NoopEntitySink),
            None,
        )
        .with_typed_embedding_index(Some(Arc::new(NoopEmbeddingIndex)))
        .with_relational_chunks(Some(Arc::new(MemoryChunkRepository::new())));

        let mut result = sample_result();
        result.document_id = doc_id.clone();
        result.chunks[0].id = format!("{doc_id}-chunk-0");

        persist_processing_result(
            graph,
            vector.clone(),
            &config,
            &IngestionPersistContext::new(&doc_id, None, Some(ws_id)),
            &result,
            ChunkVectorBuildOptions::STANDARD,
        )
        .await
        .expect("typed persist");

        // Legacy MemoryVectorStorage would have received the upsert if called.
        let chunk_key = format!("{doc_id}-chunk-0");
        assert!(
            vector.get_by_id(&chunk_key).await.unwrap().is_none(),
            "IP-AC-01: legacy VectorStorage must stay empty under typed authority"
        );
        assert!(vector.is_empty().await.unwrap());
        std::env::remove_var("EDGEQUAKE_VECTOR_BACKEND");
    }

    #[tokio::test]
    #[serial]
    async fn typed_persist_fails_closed_when_typed_index_missing() {
        std::env::set_var("EDGEQUAKE_VECTOR_BACKEND", "typed_embeddings");
        let graph = Arc::new(MemoryGraphStorage::new("typed-fc"));
        let vector = Arc::new(MemoryVectorStorage::new("typed-fc", 4));
        vector.initialize().await.unwrap();

        let config = IngestionPersistConfig::from_settings(
            IngestionPersistSettings::default(),
            Arc::new(crate::merger::NoopEntitySink),
            None,
        );
        // No typed_embedding_index / relational_chunks — must Err under typed.
        let err = persist_processing_result(
            graph,
            vector,
            &config,
            &IngestionPersistContext::new("doc1", None, None),
            &sample_result(),
            ChunkVectorBuildOptions::STANDARD,
        )
        .await
        .expect_err("typed + missing index must fail closed");
        let msg = err.to_string();
        assert!(
            msg.contains("typed_embedding_index") || msg.contains("relational_chunks"),
            "unexpected error: {msg}"
        );
        std::env::remove_var("EDGEQUAKE_VECTOR_BACKEND");
    }

    /// SPEC-383: typed persist + merge failure must compensate relational chunks
    /// (the real SSOT rollback). Memory VectorStorage delete is a no-op against
    /// never-written legacy ids.
    #[tokio::test]
    #[serial]
    async fn typed_merge_failure_compensates_relational_chunks() {
        use async_trait::async_trait;
        use edgequake_storage::embedding_family::EmbeddingFamily;
        use edgequake_storage::traits::domain::{
            ChunkRepository, DocumentId, EmbeddingCapabilities, EmbeddingIndex, EmbeddingRow,
            FleetEmbeddingRow, ModelId, ScoredChunk, ScoredFleet, UpsertReport, VectorQuery,
            WorkspaceId,
        };
        use edgequake_storage::traits::FleetEmbeddingIndex;
        use edgequake_storage::MemoryChunkRepository;

        std::env::set_var("EDGEQUAKE_VECTOR_BACKEND", "typed_embeddings");
        std::env::set_var("EDGEQUAKE_CHUNK_TEXT_AUTHORITY", "relational");

        struct NoopEmbeddingIndex;
        #[async_trait]
        impl EmbeddingIndex for NoopEmbeddingIndex {
            fn capabilities(&self) -> EmbeddingCapabilities {
                EmbeddingCapabilities {
                    metric: "cosine",
                    supports_filters: true,
                    supports_rerank: false,
                }
            }
            async fn upsert_batch(
                &self,
                _model: ModelId,
                rows: &[EmbeddingRow],
            ) -> std::result::Result<UpsertReport, edgequake_storage::StorageError> {
                Ok(UpsertReport {
                    upserted: rows.len() as u64,
                    ..Default::default()
                })
            }
            async fn search(
                &self,
                _req: &VectorQuery,
            ) -> std::result::Result<Vec<ScoredChunk>, edgequake_storage::StorageError>
            {
                Ok(vec![])
            }
            async fn delete_for_workspace(
                &self,
                _workspace: WorkspaceId,
            ) -> std::result::Result<u64, edgequake_storage::StorageError> {
                Ok(0)
            }
        }

        struct FailSink;
        #[async_trait]
        impl RelationalEntitySink for FailSink {
            async fn upsert_entity(
                &self,
                _name: &str,
                _entity_type: &str,
                _description: &str,
                _tenant_id: Option<&str>,
                _workspace_id: Option<&str>,
                _source_chunk_ids: &[String],
            ) -> crate::Result<()> {
                Err(crate::error::PipelineError::StorageError(
                    edgequake_storage::error::StorageError::Database(
                        "spec383 injected sink failure".into(),
                    ),
                ))
            }

            async fn remove_entity_sources(
                &self,
                _name: &str,
                _workspace_id: Option<&str>,
                _sources_to_remove: &[String],
                _remaining_sources: &[String],
            ) -> crate::Result<()> {
                Ok(())
            }
        }

        struct NoopFleet;
        #[async_trait]
        impl FleetEmbeddingIndex for NoopFleet {
            fn capabilities(&self, _family: EmbeddingFamily) -> EmbeddingCapabilities {
                EmbeddingCapabilities {
                    metric: "cosine",
                    supports_filters: true,
                    supports_rerank: false,
                }
            }
            async fn upsert_batch(
                &self,
                _family: EmbeddingFamily,
                _model: ModelId,
                _rows: &[FleetEmbeddingRow],
            ) -> std::result::Result<UpsertReport, edgequake_storage::StorageError> {
                Ok(UpsertReport::default())
            }
            async fn search(
                &self,
                _family: EmbeddingFamily,
                _req: &VectorQuery,
            ) -> std::result::Result<Vec<ScoredFleet>, edgequake_storage::StorageError>
            {
                Ok(vec![])
            }
            async fn delete_for_workspace(
                &self,
                _family: EmbeddingFamily,
                _workspace: WorkspaceId,
            ) -> std::result::Result<u64, edgequake_storage::StorageError> {
                Ok(0)
            }
        }

        let graph = Arc::new(MemoryGraphStorage::new("spec383"));
        let vector = Arc::new(MemoryVectorStorage::new("spec383", 4));
        vector.initialize().await.unwrap();
        let chunks = Arc::new(MemoryChunkRepository::new());

        let doc_id = uuid::Uuid::new_v4();
        let ws_id = uuid::Uuid::new_v4();
        let config = IngestionPersistConfig::from_settings(
            IngestionPersistSettings::default(),
            Arc::new(FailSink),
            None,
        )
        .with_typed_embedding_index(Some(Arc::new(NoopEmbeddingIndex)))
        .with_relational_chunks(Some(chunks.clone()))
        .with_fleet_embedding_index(Some(Arc::new(NoopFleet)));

        let mut result = sample_result();
        result.document_id = doc_id.to_string();
        result.chunks[0].id = format!("{doc_id}-chunk-0");
        result.extractions[0].entities[0].embedding = Some(vec![0.1, 0.2, 0.3, 0.4]);

        let err = persist_processing_result(
            graph,
            vector,
            &config,
            &IngestionPersistContext::new(doc_id.to_string(), None, Some(ws_id.to_string())),
            &result,
            ChunkVectorBuildOptions::STANDARD,
        )
        .await
        .expect_err("injected sink failure must fail merge");
        let msg = err.to_string();
        assert!(
            msg.contains("spec383") || msg.contains("merge"),
            "unexpected error: {msg}"
        );

        let left = chunks
            .load_for_document(DocumentId::new(doc_id))
            .await
            .expect("load compensated chunks");
        assert!(
            left.is_empty(),
            "SPEC-383: relational chunk compensation must delete chunks after merge failure"
        );

        std::env::remove_var("EDGEQUAKE_VECTOR_BACKEND");
        std::env::remove_var("EDGEQUAKE_CHUNK_TEXT_AUTHORITY");
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
