//! Entity and relationship merging into the knowledge graph.
//!
//! # Implements
//!
//! - **FEAT0006**: Entity Deduplication
//! - **FEAT0016**: Description Aggregation
//! - **FEAT0011**: Source Lineage Tracking
//!
//! # Enforces
//!
//! - **BR0008**: Entity names normalized before merge
//! - **BR0005**: Entity description max 512 tokens (summarization if exceeded)
//! - **BR0007**: Lineage records append-only (source_id accumulation)
//!
//! # WHY: Merge, Don't Replace
//!
//! When the same entity appears in multiple documents:
//!
//! 1. **Names match** (after normalization): Same graph node
//! 2. **Descriptions merge**: Combine via LLM summarization
//! 3. **Sources accumulate**: `source_id` = "chunk1|chunk2|chunk3"
//!
//! This strategy:
//! - Builds richer entity descriptions over time
//! - Maintains full provenance for source tracking
//! - Enables cascade delete via source_id filtering
//!
//! # Architecture
//!
//! - `entity`: Entity merge, update, and creation logic
//! - `relationship`: Relationship merge, update, creation, and placeholder node logic
//! - `description_merge`: SPEC-047 P7a LightRAG fragment-gate SSOT
//! - `merge_limits`: SPEC-047 P7b/P7d concurrency + SOURCE_IDS KEEP SSOT

mod description_merge;
mod entity;
mod entity_resolution;
mod entity_type_vote;
pub mod lineage;
mod merge_limits;
mod merge_progress;
mod metadata;
mod relationship;
mod weight_policy;

pub use description_merge::{
    approx_token_count, collect_unique_fragments, decide_description_merge,
    force_llm_summary_on_merge_from_env, join_description_fragments, split_description_fragments,
    summary_max_tokens_from_env, DescriptionMergeDecision, DescriptionMergePolicy,
    DEFAULT_FORCE_LLM_SUMMARY_ON_MERGE, DEFAULT_SUMMARY_MAX_TOKENS, GRAPH_FIELD_SEP,
};
pub use entity::description_similarity;
pub use entity_resolution::{
    cosine_similarity, entity_embed_er_enabled, er_llm_enabled, parse_er_on_flag,
    record_exact_merge, resolve_after_exact_miss, ErDecision, ErLadderPolicy,
    DEFAULT_EMBED_ER_THRESHOLD, ENTITY_EMBED_ER_ENV, ER_LLM_ENV,
};
pub use entity_type_vote::{apply_entity_type_vote, resolve_majority_type, ENTITY_TYPE_VOTES_KEY};
pub use lineage::{
    document_id_from_chunk_id, document_ids_from_chunk_ids, insert_chunk_lineage_properties,
    insert_document_lineage_properties, merge_and_insert_document_lineage, merge_document_ids,
    resolve_incoming_document_ids, source_document_ids_from_properties,
};
pub use merge_limits::{
    apply_local_merge_async_clamp, apply_source_ids_limit, max_source_ids_per_entity_from_env,
    max_source_ids_per_relation_from_env, merge_max_async_from_env, merge_source_ids,
    parse_max_source_ids, parse_merge_max_async, should_skip_description_update_keep,
    source_chunk_ids_from_properties, source_ids_limit_method_from_env, truncate_keep_doc_diverse,
    SourceIdsLimitMethod, DEFAULT_MAX_SOURCE_IDS, DEFAULT_MERGE_MAX_ASYNC, LOCAL_MERGE_MAX_ASYNC,
};
pub use weight_policy::WeightPolicy;

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use edgequake_observability::record_ingest_stage_duration;
use edgequake_storage::{GraphStorage, TextEmbedder, VectorStorage};

use crate::error::Result;
use crate::extractor::ExtractionResult;
use crate::summarizer::LLMSummarizer;

/// Backend for LLM description merges (Dependency Inversion for P7a tests).
#[async_trait]
pub trait DescriptionMergeBackend: Send + Sync {
    /// Merge entity description fragments into one summary.
    async fn merge_entity_descriptions(
        &self,
        entity_name: &str,
        descriptions: &[String],
    ) -> Result<String>;

    /// Merge relationship description fragments into one summary.
    async fn merge_relationship_descriptions(
        &self,
        source: &str,
        target: &str,
        descriptions: &[String],
    ) -> Result<String>;
}

#[async_trait]
impl DescriptionMergeBackend for LLMSummarizer {
    async fn merge_entity_descriptions(
        &self,
        entity_name: &str,
        descriptions: &[String],
    ) -> Result<String> {
        LLMSummarizer::merge_entity_descriptions(self, entity_name, descriptions).await
    }

    async fn merge_relationship_descriptions(
        &self,
        source: &str,
        target: &str,
        descriptions: &[String],
    ) -> Result<String> {
        LLMSummarizer::merge_relationship_descriptions(self, source, target, descriptions).await
    }
}

// ── Lineage Sink (SPEC-032 W-08) ─────────────────────────────────────────────

/// Trait for persisting chunk→entity and chunk→relation lineage links.
///
/// # WHY: Dependency Inversion Principle (SOLID-D)
///
/// The pipeline crate must not depend on `sqlx` or any concrete storage type.
/// Callers (edgequake-api) provide a concrete implementation when lineage
/// persistence is enabled. The default no-op implementation ensures backwards
/// compatibility.
///
/// # Implementations
///
/// - `NoopLineageSink` — default, writes nothing
/// - `PostgresLineageSink` in `edgequake-api` — inserts into `chunk_entity_links`
///   and `chunk_relation_links` (migration 066)
///
/// Chunk→entity lineage row (SPEC-047 P4 — batch writes, DRY with RelationLineageLink).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityLineageLink {
    pub chunk_id: String,
    pub entity_name: String,
    pub workspace_id: String,
}

/// Chunk→relation lineage row (SPEC-032 W-08 batch writes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationLineageLink {
    pub chunk_id: String,
    pub source_entity: String,
    pub target_entity: String,
    pub workspace_id: String,
}

#[async_trait]
pub trait LineageSink: Send + Sync {
    /// Record that chunk `chunk_id` contributed to entity `entity_name`.
    ///
    /// Best-effort: must NOT fail the ingestion on error.
    async fn record_entity_link(
        &self,
        chunk_id: &str,
        entity_name: &str,
        workspace_id: &str,
    ) -> Result<()>;

    /// Record that chunk `chunk_id` contributed to relation (source→target).
    ///
    /// Best-effort: must NOT fail the ingestion on error.
    async fn record_relation_link(
        &self,
        chunk_id: &str,
        source_entity: &str,
        target_entity: &str,
        workspace_id: &str,
    ) -> Result<()>;

    /// Append a description merge event to entity history.
    ///
    /// Called when `update_entity_node` merges a new description into an
    /// existing entity. The history is stored in `entities.description_history`
    /// as an append-only JSONB array.
    ///
    /// Best-effort: must NOT fail the ingestion on error.
    async fn append_description_history(
        &self,
        entity_name: &str,
        workspace_id: &str,
        description: &str,
        chunk_id: &str,
    ) -> Result<()>;

    /// Batch insert chunk→relation links (default: sequential singles).
    async fn record_relation_links_batch(&self, links: &[RelationLineageLink]) -> Result<()> {
        for link in links {
            self.record_relation_link(
                &link.chunk_id,
                &link.source_entity,
                &link.target_entity,
                &link.workspace_id,
            )
            .await?;
        }
        Ok(())
    }

    /// Batch insert chunk→entity links (default: sequential singles).
    ///
    /// SPEC-047 P4: kill O(E×S) N+1 when a concrete sink overrides this.
    async fn record_entity_links_batch(&self, links: &[EntityLineageLink]) -> Result<()> {
        for link in links {
            self.record_entity_link(&link.chunk_id, &link.entity_name, &link.workspace_id)
                .await?;
        }
        Ok(())
    }
}

/// No-op implementation — used when lineage tracking is disabled (default).
pub struct NoopLineageSink;

#[async_trait]
impl LineageSink for NoopLineageSink {
    async fn record_entity_link(&self, _: &str, _: &str, _: &str) -> Result<()> {
        Ok(())
    }
    async fn record_relation_link(&self, _: &str, _: &str, _: &str, _: &str) -> Result<()> {
        Ok(())
    }
    async fn append_description_history(&self, _: &str, _: &str, _: &str, _: &str) -> Result<()> {
        Ok(())
    }
    async fn record_relation_links_batch(&self, _: &[RelationLineageLink]) -> Result<()> {
        Ok(())
    }
    async fn record_entity_links_batch(&self, _: &[EntityLineageLink]) -> Result<()> {
        Ok(())
    }
}

// ── Relational CQRS Sink (SPEC-021 P3-01) ────────────────────────────────────

/// Trait for writing entity/relationship data to the relational CQRS read model.
///
/// # WHY: Dependency Inversion Principle (SPEC-021 R-SOLID)
///
/// The pipeline crate must not take a direct dependency on `sqlx::PgPool`
/// or any PostgreSQL type. Instead, callers (edgequake-api, edgequake-core)
/// provide a concrete implementation of this trait when dual-write is enabled.
///
/// # Default behaviour
///
/// When no sink is wired, the merger silently skips relational sync (old behaviour).
/// This ensures ascending compatibility: deployments without migration 039 applied
/// continue working exactly as before.
///
/// # Implementations
///
/// - `NoopEntitySink` — default, no-op (sync disabled)
/// - `PostgresEntitySink` in `edgequake-api` — writes to `entities` table
///
/// @implements SPEC-021 P3-01
/// One relational entity upsert (SPEC-091 IP1 batch row).
#[derive(Debug, Clone)]
pub struct EntitySinkRow {
    pub name: String,
    pub entity_type: String,
    pub description: String,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
    pub source_chunk_ids: Vec<String>,
}

/// One relational relationship upsert (SPEC-091 IP1 batch row).
#[derive(Debug, Clone)]
pub struct RelationshipSinkRow {
    pub source_name: String,
    pub target_name: String,
    pub relation_type: String,
    pub description: String,
    pub weight: f32,
    pub tenant_id: Option<String>,
    pub workspace_id: Option<String>,
}

/// SPEC-130: identity produced by relational relationship sink for in-session fleet mirror.
///
/// `ids` keys are legacy vector ids (`SRC->TGT:TYPE`) matching
/// [`edgequake_storage::format_relationship_legacy_key`].
#[derive(Debug, Clone, Default)]
pub struct RelationshipSinkReport {
    pub ids: std::collections::HashMap<String, uuid::Uuid>,
    pub missing_fk: u64,
}

#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait RelationalEntitySink: Send + Sync {
    /// Upsert an entity into the relational CQRS read model.
    ///
    /// Called after the primary graph write succeeds.
    /// Must be best-effort: implementors SHOULD NOT fail the ingestion on error
    /// unless the typed vector backend requires a fail-closed FK spine.
    async fn upsert_entity(
        &self,
        name: &str,
        entity_type: &str,
        description: &str,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
        source_chunk_ids: &[String],
    ) -> Result<()>;

    /// Upsert a relationship into the relational CQRS read model (typed fleet FK).
    ///
    /// Default no-op so legacy callers/tests stay green.
    async fn upsert_relationship(
        &self,
        _source_name: &str,
        _target_name: &str,
        _relation_type: &str,
        _description: &str,
        _weight: f32,
        _tenant_id: Option<&str>,
        _workspace_id: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }

    /// SPEC-091 IP1 / LAW-IP2: batch entity upsert (default loops single-row).
    async fn upsert_entities_batch(&self, rows: &[EntitySinkRow]) -> Result<()> {
        for row in rows {
            self.upsert_entity(
                &row.name,
                &row.entity_type,
                &row.description,
                row.tenant_id.as_deref(),
                row.workspace_id.as_deref(),
                &row.source_chunk_ids,
            )
            .await?;
        }
        Ok(())
    }

    /// SPEC-091 IP1 / LAW-IP2 / SPEC-130: batch relationship upsert.
    ///
    /// Default loops single-row and returns an empty id map (Noop/Spy). Production
    /// Postgres sink overrides to `RETURNING` relationship UUIDs for RelVectors.
    async fn upsert_relationships_batch(
        &self,
        rows: &[RelationshipSinkRow],
    ) -> Result<RelationshipSinkReport> {
        for row in rows {
            self.upsert_relationship(
                &row.source_name,
                &row.target_name,
                &row.relation_type,
                &row.description,
                row.weight,
                row.tenant_id.as_deref(),
                row.workspace_id.as_deref(),
            )
            .await?;
        }
        Ok(RelationshipSinkReport::default())
    }

    /// Remove or update source references when a document is deleted.
    ///
    /// Called from `delete_document()` after graph node deletion.
    async fn remove_entity_sources(
        &self,
        name: &str,
        workspace_id: Option<&str>,
        sources_to_remove: &[String],
        remaining_sources: &[String],
    ) -> Result<()>;
}

/// No-op implementation — used when dual-write is disabled (default).
pub struct NoopEntitySink;

#[async_trait]
impl RelationalEntitySink for NoopEntitySink {
    async fn upsert_entity(
        &self,
        _name: &str,
        _entity_type: &str,
        _description: &str,
        _tenant_id: Option<&str>,
        _workspace_id: Option<&str>,
        _source_chunk_ids: &[String],
    ) -> Result<()> {
        Ok(()) // best-effort no-op
    }

    async fn remove_entity_sources(
        &self,
        _name: &str,
        _workspace_id: Option<&str>,
        _sources_to_remove: &[String],
        _remaining_sources: &[String],
    ) -> Result<()> {
        Ok(()) // best-effort no-op
    }
}

// ── MergerConfig ──────────────────────────────────────────────────────────────

/// Configuration for the merger.
#[derive(Debug, Clone)]
pub struct MergerConfig {
    /// Maximum description length before summarization.
    pub max_description_length: usize,

    /// Weight decay for older descriptions.
    pub description_decay: f32,

    /// Minimum importance score to keep an entity.
    pub min_importance: f32,

    /// Maximum number of source references to keep.
    pub max_sources: usize,

    /// Use LLM for description merging (if summarizer is provided).
    pub use_llm_summarization: bool,

    /// Jaccard similarity threshold above which LLM summarization is skipped.
    ///
    /// WHY: If descriptions are ≥ threshold similar, the LLM call adds noise
    /// but costs ~500ms and API credits. At 0.85, only truly distinct
    /// descriptions trigger an LLM merge. Tunable via
    /// `EDGEQUAKE_MERGE_SIMILARITY_THRESHOLD` env var.
    ///
    /// Set to 0.0 to always summarize, 1.0 to never summarize.
    pub description_similarity_threshold: f32,

    /// SPEC-047 P7a / LightRAG: fragment count that forces LLM summary.
    ///
    /// Below this count (and under [`Self::summary_max_tokens`]), fragments are
    /// joined with [`GRAPH_FIELD_SEP`] without an LLM call.
    /// Env: `EDGEQUAKE_FORCE_LLM_SUMMARY_ON_MERGE` (alias `FORCE_LLM_SUMMARY_ON_MERGE`).
    pub force_llm_summary_on_merge: usize,

    /// Approx token budget for join-without-LLM (LightRAG `SUMMARY_MAX_TOKENS`).
    /// Env: `EDGEQUAKE_SUMMARY_MAX_TOKENS` (alias `SUMMARY_MAX_TOKENS`).
    pub summary_max_tokens: usize,

    /// SPEC-047 P7b / LightRAG `graph_max_async` — concurrent unique entity/rel merges.
    /// Env: `EDGEQUAKE_MERGE_MAX_ASYNC` (default `llm_max_async * 2` ≈ 8).
    pub merge_max_async: usize,

    /// SPEC-047 P7d / LightRAG `MAX_SOURCE_IDS_PER_ENTITY` (default 200).
    pub max_source_ids_per_entity: usize,

    /// SPEC-047 P7d / LightRAG `MAX_SOURCE_IDS_PER_RELATION` (default 200).
    pub max_source_ids_per_relation: usize,

    /// SPEC-047 P7d / LightRAG `SOURCE_IDS_LIMIT_METHOD` (default KEEP).
    pub source_ids_limit_method: SourceIdsLimitMethod,
}

impl Default for MergerConfig {
    fn default() -> Self {
        let threshold = std::env::var("EDGEQUAKE_MERGE_SIMILARITY_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.85);
        Self {
            max_description_length: 4096,
            description_decay: 0.9,
            min_importance: 0.1,
            max_sources: 10,
            use_llm_summarization: true, // Enable by default for SOTA quality
            description_similarity_threshold: threshold,
            force_llm_summary_on_merge: force_llm_summary_on_merge_from_env(),
            summary_max_tokens: summary_max_tokens_from_env(),
            merge_max_async: merge_max_async_from_env(),
            max_source_ids_per_entity: max_source_ids_per_entity_from_env(),
            max_source_ids_per_relation: max_source_ids_per_relation_from_env(),
            source_ids_limit_method: source_ids_limit_method_from_env(),
        }
    }
}

impl MergerConfig {
    /// SSOT policy view for P7a description merge decisions.
    pub fn description_merge_policy(&self) -> DescriptionMergePolicy {
        DescriptionMergePolicy::from_parts(
            self.use_llm_summarization,
            self.force_llm_summary_on_merge,
            self.summary_max_tokens,
            self.description_similarity_threshold,
            self.max_description_length,
        )
    }
}

/// Merges extracted entities and relationships into the knowledge graph.
/// @implements FEAT0005
pub struct KnowledgeGraphMerger<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> {
    pub(super) config: MergerConfig,
    pub(super) graph_storage: Arc<G>,
    pub(super) vector_storage: Arc<V>,
    pub(super) tenant_id: Option<String>,
    pub(super) workspace_id: Option<String>,
    /// Optional LLM backend for intelligent description merging (P7a DI).
    pub(super) summarizer: Option<Arc<dyn DescriptionMergeBackend>>,
    /// Optional text embedder for relation-endpoint placeholders (050 B7).
    /// When None, placeholders are AGE-only (pre-B7 behaviour).
    pub(super) text_embedder: Option<Arc<dyn TextEmbedder>>,
    /// Optional CQRS relational sink (SPEC-021 P3-01).
    /// When None, relational sync is skipped (backwards-compatible default).
    pub(super) relational_sink: Arc<dyn RelationalEntitySink>,
    /// Optional lineage sink (SPEC-032 W-08).
    /// When None, lineage links are not persisted.
    pub(super) lineage_sink: Arc<dyn LineageSink>,
    /// SPEC-091 IW2: optional typed fleet mirror (entity/relationship/report).
    pub(super) fleet_embedding_index:
        Option<Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>>,
}

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> KnowledgeGraphMerger<G, V> {
    /// Create a new merger.
    pub fn new(config: MergerConfig, graph_storage: Arc<G>, vector_storage: Arc<V>) -> Self {
        Self {
            config,
            graph_storage,
            vector_storage,
            tenant_id: None,
            workspace_id: None,
            summarizer: None,
            text_embedder: None,
            relational_sink: Arc::new(NoopEntitySink),
            lineage_sink: Arc::new(NoopLineageSink),
            fleet_embedding_index: None,
        }
    }

    /// SPEC-091 IW2: mirror entity/relationship/report vectors into typed tables.
    pub fn with_fleet_embedding_index(
        mut self,
        index: Arc<dyn edgequake_storage::traits::FleetEmbeddingIndex>,
    ) -> Self {
        self.fleet_embedding_index = Some(index);
        self
    }

    /// Wire a relational CQRS sink for dual-write (SPEC-021 P3-01).
    pub fn with_relational_sink(mut self, sink: Arc<dyn RelationalEntitySink>) -> Self {
        self.relational_sink = sink;
        self
    }

    /// Wire a lineage sink for chunk→entity/relation provenance (SPEC-032 W-08).
    pub fn with_lineage_sink(mut self, sink: Arc<dyn LineageSink>) -> Self {
        self.lineage_sink = sink;
        self
    }

    /// Wire text embedder for placeholder entity VDB upserts (050 B7 / LightRAG parity).
    pub fn with_text_embedder(mut self, embedder: Arc<dyn TextEmbedder>) -> Self {
        self.text_embedder = Some(embedder);
        self
    }

    /// Upsert vectors in `EDGEQUAKE_VECTOR_UPSERT_CHUNK`-sized slices with progress.
    ///
    /// `count_as_entities`: when true, `done` advances `entities_processed`;
    /// otherwise advances `relationships_processed` (entity totals stay complete).
    ///
    /// SPEC-059: only IDs returned by [`VectorStorage::upsert_report_created`]
    /// (atomic insert detection) are recorded in [`MergeArtifacts`] so compensate
    /// never deletes shared entity/rel vectors — no preflight TOCTOU.
    ///
    /// SPEC-130: `known_relationship_ids` passes sink-returned UUIDs into fleet
    /// mirror for relationship chunks (`count_as_entities == false`).
    #[allow(clippy::too_many_arguments)]
    async fn upsert_vectors_chunked(
        &self,
        batch: &[(String, Vec<f32>, serde_json::Value)],
        progress: Option<&MergeProgressCallback>,
        phase: MergePhase,
        entities_total: usize,
        relationships_total: usize,
        entities_created: usize,
        entities_updated: usize,
        count_as_entities: bool,
        artifacts: &mut MergeArtifacts,
        known_relationship_ids: Option<&std::collections::HashMap<String, uuid::Uuid>>,
    ) -> Result<()> {
        let chunk = edgequake_storage::vector_upsert_chunk_size();
        let mut done = 0usize;
        for slice in batch.chunks(chunk) {
            // SPEC-059: atomic insert report (Postgres xmax / memory write-lock).
            let created = self.vector_storage.upsert_report_created(slice).await?;
            let created_set: std::collections::HashSet<&str> =
                created.iter().map(|id| id.as_str()).collect();
            let skipped_shared = slice
                .iter()
                .filter(|(id, _, _)| !created_set.contains(id.as_str()))
                .count();
            if skipped_shared > 0 {
                edgequake_storage::record_compensate_shared_entity_skipped(skipped_shared);
            }

            // SPEC-057 P3 + SPEC-059: record only *created* IDs after each
            // successful chunk so compensate rolls back orphans without wiping
            // embeddings for entities shared across documents.
            for id in created {
                if count_as_entities {
                    artifacts.entity_vector_ids.push(id);
                } else {
                    artifacts.relationship_vector_ids.push(id);
                }
            }
            if let Some(ref fleet_index) = self.fleet_embedding_index {
                let known = if count_as_entities {
                    None
                } else {
                    known_relationship_ids
                };
                match fleet_index
                    .mirror_legacy_batch(slice, count_as_entities, known)
                    .await
                {
                    Ok(report) => {
                        let typed = edgequake_storage::vector_backend::vector_backend_reads_typed(
                            edgequake_storage::vector_backend_from_env(),
                        );
                        if typed && !slice.is_empty() {
                            if !report.invalid_workspace.is_empty() {
                                return Err(crate::error::PipelineError::StorageError(
                                    edgequake_storage::error::StorageError::Database(format!(
                                        "SPEC-098: typed fleet mirror invalid workspace_id on {}/{} rows \
                                         (sample: {:?})",
                                        report.invalid_workspace.len(),
                                        slice.len(),
                                        report.invalid_workspace
                                    )),
                                ));
                            }
                            if !report.is_complete() {
                                let detail = if count_as_entities {
                                    "relational entity FK miss or name mismatch — \
                                     bare entities.name must match entity:NAME; ensure \
                                     PostgresEntitySink wrote the entity spine before fleet mirror"
                                        .to_string()
                                } else {
                                    "relationship UUID missing from sink map or unresolved legacy key — \
                                     in-session RelVectors must use sink-returned relationships.id"
                                        .to_string()
                                };
                                return Err(crate::error::PipelineError::StorageError(
                                    edgequake_storage::error::StorageError::Database(format!(
                                        "SPEC-091: typed fleet mirror resolved {}/{} rows \
                                         ({detail}; SPEC-098 misses: {:?})",
                                        report.resolved, report.eligible, report.misses
                                    )),
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        let typed = edgequake_storage::vector_backend::vector_backend_reads_typed(
                            edgequake_storage::vector_backend_from_env(),
                        );
                        if typed {
                            return Err(crate::error::PipelineError::StorageError(e));
                        }
                        tracing::warn!(
                            error = %e,
                            count_as_entities,
                            "SPEC-091 IW2: typed fleet dual-write failed (legacy upsert succeeded)"
                        );
                    }
                }
            }
            done += slice.len();
            let (entities_processed, relationships_processed) = if count_as_entities {
                (done.min(entities_total), 0)
            } else {
                (entities_total, done.min(relationships_total))
            };
            emit_progress(
                progress,
                MergeProgress {
                    phase,
                    entities_processed,
                    entities_total,
                    relationships_processed,
                    relationships_total,
                    entities_created,
                    entities_updated,
                    relationships_created: 0,
                    relationships_updated: 0,
                },
            );
        }
        Ok(())
    }

    /// Set tenant and workspace IDs.
    pub fn with_tenant_context(
        mut self,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Self {
        self.tenant_id = tenant_id;
        self.workspace_id = workspace_id;
        self
    }

    /// Set the LLM summarizer for intelligent description merging.
    pub fn with_summarizer(self, summarizer: Arc<LLMSummarizer>) -> Self {
        self.with_merge_backend(summarizer)
    }

    /// Inject any [`DescriptionMergeBackend`] (tests / alternate summarizers).
    pub fn with_merge_backend(mut self, backend: Arc<dyn DescriptionMergeBackend>) -> Self {
        self.summarizer = Some(backend);
        self
    }

    /// Apply P7a policy then optionally LLM-merge entity descriptions.
    pub(super) async fn resolve_entity_description(
        &self,
        entity_name: &str,
        existing: &str,
        incoming: &str,
    ) -> Result<String> {
        let policy = self.config.description_merge_policy();
        match decide_description_merge(existing, incoming, &policy) {
            DescriptionMergeDecision::Resolved(s) => Ok(s),
            DescriptionMergeDecision::NeedsLlm { fragments } => {
                if let Some(backend) = &self.summarizer {
                    match backend
                        .merge_entity_descriptions(entity_name, &fragments)
                        .await
                    {
                        Ok(merged) => Ok(merged),
                        Err(e) => {
                            tracing::warn!(
                                entity = %entity_name,
                                error = %e,
                                "LLM entity description merge failed; joining fragments"
                            );
                            Ok(join_description_fragments(
                                &fragments,
                                self.config.max_description_length,
                            ))
                        }
                    }
                } else {
                    Ok(join_description_fragments(
                        &fragments,
                        self.config.max_description_length,
                    ))
                }
            }
        }
    }

    /// Apply P7a policy then optionally LLM-merge relationship descriptions.
    pub(super) async fn resolve_relationship_description(
        &self,
        source: &str,
        target: &str,
        existing: &str,
        incoming: &str,
    ) -> Result<String> {
        let policy = self.config.description_merge_policy();
        match decide_description_merge(existing, incoming, &policy) {
            DescriptionMergeDecision::Resolved(s) => Ok(s),
            DescriptionMergeDecision::NeedsLlm { fragments } => {
                if let Some(backend) = &self.summarizer {
                    match backend
                        .merge_relationship_descriptions(source, target, &fragments)
                        .await
                    {
                        Ok(merged) => Ok(merged),
                        Err(e) => {
                            tracing::warn!(
                                source = %source,
                                target = %target,
                                error = %e,
                                "LLM relationship description merge failed; joining fragments"
                            );
                            Ok(join_description_fragments(
                                &fragments,
                                self.config.max_description_length,
                            ))
                        }
                    }
                } else {
                    Ok(join_description_fragments(
                        &fragments,
                        self.config.max_description_length,
                    ))
                }
            }
        }
    }

    /// Merge extraction results into the knowledge graph.
    ///
    /// # WHY: Global batching (SPEC-032 W-02 + W-03)
    ///
    /// Previous implementation looped per `ExtractionResult` (one per chunk),
    /// causing N × 4 AGE round trips for a document with N chunks:
    ///   - get_nodes_batch  × N
    ///   - upsert_nodes_batch × N
    ///   - get_edges_for_nodes_batch × N
    ///   - upsert_edges_batch × N
    ///
    /// At 50 chunks × 5 ms/round-trip = 1 second of pure network overhead,
    /// before any actual Cypher execution time.
    ///
    /// New implementation collects ALL entities and relationships globally,
    /// deduplicates within-document, then issues exactly 2 AGE round trips:
    ///   - 1 × get_nodes_batch  (all unique entities)
    ///   - 1 × upsert_nodes_batch
    ///   - 1 × get_edges_for_nodes_batch  (all unique relationship endpoints)
    ///   - 1 × upsert_edges_batch
    ///
    /// 50 round trips → 2 round trips per document. ~96% reduction.
    pub async fn merge(&self, results: Vec<ExtractionResult>) -> Result<MergeStats> {
        self.merge_with_progress(results, None).await
    }

    /// Merge with optional per-batch progress callback (SPEC-032 W-04).
    ///
    /// Progress is emitted:
    ///   1. Before entity vector upsert
    ///   2. After entity graph merge batch (with running totals)
    ///   3. After relationship graph merge batch (with running totals)
    pub async fn merge_with_progress(
        &self,
        results: Vec<ExtractionResult>,
        progress: Option<&MergeProgressCallback>,
    ) -> Result<MergeStats> {
        let mut stats = MergeStats::default();

        // ── Totals for progress reporting ────────────────────────────────
        let total_entities: usize = results.iter().map(|r| r.entities.len()).sum();
        let total_relationships: usize = results.iter().map(|r| r.relationships.len()).sum();

        tracing::info!(
            total_entities,
            total_relationships,
            chunks = results.len(),
            "Merger: starting global batch merge (SPEC-032 W-02/W-03)"
        );

        emit_progress(
            progress,
            MergeProgress {
                phase: if edgequake_storage::vector_backend::vector_backend_reads_typed(
                    edgequake_storage::vector_backend_from_env(),
                ) {
                    MergePhase::EntityGraph
                } else {
                    MergePhase::EntityVectors
                },
                entities_processed: 0,
                entities_total: total_entities,
                relationships_processed: 0,
                relationships_total: total_relationships,
                entities_created: 0,
                entities_updated: 0,
                relationships_created: 0,
                relationships_updated: 0,
            },
        );

        // ── Entity / relationship vector + graph phases ─────────────────
        // Under typed_embeddings, graph merge MUST precede fleet vector write
        // (mirror_legacy_batch resolves entities/relationships by name FK).
        let typed_authority = edgequake_storage::vector_backend::vector_backend_reads_typed(
            edgequake_storage::vector_backend_from_env(),
        );
        let entity_vector_batch = self.collect_entity_vector_batch(&results);
        let all_rels: Vec<_> = results
            .iter()
            .flat_map(|r| r.relationships.iter().cloned())
            .collect();
        let all_rel_vectors = self.collect_relationship_vector_batch(&all_rels);
        let all_entities: Vec<_> = results
            .iter()
            .flat_map(|r| r.entities.iter().cloned())
            .collect();
        let all_relationships: Vec<_> = results
            .iter()
            .flat_map(|r| r.relationships.iter().cloned())
            .collect();

        // Legacy order: EntityVectors → EntityGraph → RelVectors → RelGraph.
        // Typed order:  EntityGraph → EntityVectors → RelGraph → RelVectors.
        // SPEC-130: capture sink relationship UUID map for typed RelVectors.
        let mut rel_sink_report = RelationshipSinkReport::default();
        if !typed_authority && !entity_vector_batch.is_empty() {
            let stage_start = Instant::now();
            let entity_vec_result = self
                .upsert_vectors_chunked(
                    &entity_vector_batch,
                    progress,
                    MergePhase::EntityVectors,
                    total_entities,
                    total_relationships,
                    0,
                    0,
                    true,
                    &mut stats.artifacts,
                    None,
                )
                .await;
            record_ingest_stage_duration(
                "entity_vector_upsert",
                stage_start.elapsed().as_secs_f64(),
            );
            if let Err(e) = entity_vec_result {
                stats.record_error(e.to_string());
                tracing::warn!(
                    error.source = "pipeline_merger",
                    error.action = "upsert_entity_vectors",
                    error.message = %e,
                    partial_entity_vectors = stats.artifacts.entity_vector_ids.len(),
                    "Failed entity vector upsert; returning partial MergeArtifacts"
                );
                return Ok(stats);
            }
        }

        emit_progress(
            progress,
            MergeProgress {
                phase: MergePhase::EntityGraph,
                entities_processed: 0,
                entities_total: total_entities,
                relationships_processed: 0,
                relationships_total: total_relationships,
                entities_created: 0,
                entities_updated: 0,
                relationships_created: 0,
                relationships_updated: 0,
            },
        );

        if !all_entities.is_empty() {
            let stage_start = Instant::now();
            let entity_graph_result = self.merge_entities_batch(all_entities, &mut stats).await;
            record_ingest_stage_duration("age_node_upsert", stage_start.elapsed().as_secs_f64());
            if let Err(e) = entity_graph_result {
                stats.record_error(e.to_string());
                tracing::warn!(
                    error.source = "pipeline_merger",
                    error.action = "merge_entities_batch_global",
                    error.message = %e,
                    "Failed to merge global entity batch; aborting remaining merge phases"
                );
                return Ok(stats);
            }
        }

        if typed_authority && !entity_vector_batch.is_empty() {
            emit_progress(
                progress,
                MergeProgress {
                    phase: MergePhase::EntityVectors,
                    entities_processed: 0,
                    entities_total: total_entities,
                    relationships_processed: 0,
                    relationships_total: total_relationships,
                    entities_created: stats.entities_created,
                    entities_updated: stats.entities_updated,
                    relationships_created: 0,
                    relationships_updated: 0,
                },
            );
            let stage_start = Instant::now();
            let entity_vec_result = self
                .upsert_vectors_chunked(
                    &entity_vector_batch,
                    progress,
                    MergePhase::EntityVectors,
                    total_entities,
                    total_relationships,
                    0,
                    0,
                    true,
                    &mut stats.artifacts,
                    None,
                )
                .await;
            record_ingest_stage_duration(
                "entity_vector_upsert",
                stage_start.elapsed().as_secs_f64(),
            );
            if let Err(e) = entity_vec_result {
                stats.record_error(e.to_string());
                tracing::warn!(
                    error.source = "pipeline_merger",
                    error.action = "upsert_entity_vectors",
                    error.message = %e,
                    partial_entity_vectors = stats.artifacts.entity_vector_ids.len(),
                    "Failed entity vector upsert; returning partial MergeArtifacts"
                );
                return Ok(stats);
            }
        }

        emit_progress(
            progress,
            MergeProgress {
                phase: if typed_authority {
                    MergePhase::RelationshipGraph
                } else {
                    MergePhase::RelationshipVectors
                },
                entities_processed: total_entities,
                entities_total: total_entities,
                relationships_processed: 0,
                relationships_total: total_relationships,
                entities_created: stats.entities_created,
                entities_updated: stats.entities_updated,
                relationships_created: 0,
                relationships_updated: 0,
            },
        );

        if !typed_authority && !all_rel_vectors.is_empty() {
            let stage_start = Instant::now();
            let rel_vec_result = self
                .upsert_vectors_chunked(
                    &all_rel_vectors,
                    progress,
                    MergePhase::RelationshipVectors,
                    total_entities,
                    total_relationships,
                    stats.entities_created,
                    stats.entities_updated,
                    false,
                    &mut stats.artifacts,
                    None,
                )
                .await;
            record_ingest_stage_duration("rel_vector_upsert", stage_start.elapsed().as_secs_f64());
            if let Err(e) = rel_vec_result {
                stats.record_error(e.to_string());
                tracing::warn!(
                    error.source = "pipeline_merger",
                    error.action = "upsert_relationship_vectors",
                    error.message = %e,
                    partial_relationship_vectors =
                        stats.artifacts.relationship_vector_ids.len(),
                    "Failed relationship vector upsert; returning partial MergeArtifacts"
                );
                return Ok(stats);
            }
        }

        if !typed_authority {
            emit_progress(
                progress,
                MergeProgress {
                    phase: MergePhase::RelationshipGraph,
                    entities_processed: total_entities,
                    entities_total: total_entities,
                    relationships_processed: 0,
                    relationships_total: total_relationships,
                    entities_created: stats.entities_created,
                    entities_updated: stats.entities_updated,
                    relationships_created: 0,
                    relationships_updated: 0,
                },
            );
        }

        if !all_relationships.is_empty() {
            let progress_ctx = progress.map(|cb| merge_progress::MergeProgressCtx {
                callback: Some(cb),
                entities_processed: total_entities,
                entities_total: total_entities,
                relationships_total: total_relationships,
                entities_created: stats.entities_created,
                entities_updated: stats.entities_updated,
            });

            let stage_start = Instant::now();
            let rel_graph_result = self
                .merge_relationships_batch(all_relationships, &mut stats, progress_ctx)
                .await;
            record_ingest_stage_duration("age_edge_upsert", stage_start.elapsed().as_secs_f64());
            match rel_graph_result {
                Ok(report) => {
                    rel_sink_report = report;
                }
                Err(e) => {
                    stats.record_error(e.to_string());
                    tracing::warn!(
                        error.source = "pipeline_merger",
                        error.action = "merge_relationships_batch_global",
                        error.message = %e,
                        "Failed to merge global relationship batch; aborting finalize"
                    );
                    return Ok(stats);
                }
            }
        }

        if typed_authority && !all_rel_vectors.is_empty() {
            emit_progress(
                progress,
                MergeProgress {
                    phase: MergePhase::RelationshipVectors,
                    entities_processed: total_entities,
                    entities_total: total_entities,
                    relationships_processed: 0,
                    relationships_total: total_relationships,
                    entities_created: stats.entities_created,
                    entities_updated: stats.entities_updated,
                    relationships_created: stats.relationships_created,
                    relationships_updated: stats.relationships_updated,
                },
            );
            let stage_start = Instant::now();
            let known_ids = if rel_sink_report.ids.is_empty() {
                None
            } else {
                Some(&rel_sink_report.ids)
            };
            let rel_vec_result = self
                .upsert_vectors_chunked(
                    &all_rel_vectors,
                    progress,
                    MergePhase::RelationshipVectors,
                    total_entities,
                    total_relationships,
                    stats.entities_created,
                    stats.entities_updated,
                    false,
                    &mut stats.artifacts,
                    known_ids,
                )
                .await;
            record_ingest_stage_duration("rel_vector_upsert", stage_start.elapsed().as_secs_f64());
            if let Err(e) = rel_vec_result {
                stats.record_error(e.to_string());
                tracing::warn!(
                    error.source = "pipeline_merger",
                    error.action = "upsert_relationship_vectors",
                    error.message = %e,
                    partial_relationship_vectors =
                        stats.artifacts.relationship_vector_ids.len(),
                    "Failed relationship vector upsert; returning partial MergeArtifacts"
                );
                return Ok(stats);
            }
        }

        emit_progress(
            progress,
            MergeProgress {
                phase: MergePhase::Finalizing,
                entities_processed: total_entities,
                entities_total: total_entities,
                relationships_processed: total_relationships,
                relationships_total: total_relationships,
                entities_created: stats.entities_created,
                entities_updated: stats.entities_updated,
                relationships_created: stats.relationships_created,
                relationships_updated: stats.relationships_updated,
            },
        );

        tracing::info!(
            entities_created = stats.entities_created,
            entities_updated = stats.entities_updated,
            relationships_created = stats.relationships_created,
            relationships_updated = stats.relationships_updated,
            errors = stats.errors,
            "Merger: global batch merge complete"
        );

        Ok(stats)
    }
}

// ── Progress types (SPEC-032 W-04) ───────────────────────────────────────────

/// Sub-phase of the knowledge graph merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergePhase {
    EntityVectors,
    EntityGraph,
    RelationshipVectors,
    RelationshipGraph,
    Finalizing,
}

impl MergePhase {
    pub fn label(&self) -> &'static str {
        match self {
            Self::EntityVectors => "Storing entity embeddings",
            Self::EntityGraph => "Merging entities into knowledge graph",
            Self::RelationshipVectors => "Storing relationship embeddings",
            Self::RelationshipGraph => "Merging relationships into knowledge graph",
            Self::Finalizing => "Finalizing",
        }
    }

    /// Snake_case identifier for serialization / event routing.
    pub fn label_id(&self) -> &'static str {
        match self {
            Self::EntityVectors => "entity_vectors",
            Self::EntityGraph => "entity_graph",
            Self::RelationshipVectors => "relationship_vectors",
            Self::RelationshipGraph => "relationship_graph",
            Self::Finalizing => "finalizing",
        }
    }
}

/// Snapshot of merge progress for progress callbacks.
#[derive(Debug, Clone)]
pub struct MergeProgress {
    pub phase: MergePhase,
    pub entities_processed: usize,
    pub entities_total: usize,
    pub relationships_processed: usize,
    pub relationships_total: usize,
    pub entities_created: usize,
    pub entities_updated: usize,
    pub relationships_created: usize,
    pub relationships_updated: usize,
}

/// Callback type for merge progress events. Fire-and-forget from caller's perspective.
pub type MergeProgressCallback = Box<dyn Fn(MergeProgress) + Send + Sync>;

pub(crate) fn emit_progress(cb: Option<&MergeProgressCallback>, progress: MergeProgress) {
    if let Some(f) = cb {
        f(progress);
    }
}

/// Statistics from a merge operation.
#[derive(Debug, Clone, Default)]
pub struct MergeStats {
    /// Number of new entities created.
    pub entities_created: usize,

    /// Number of existing entities updated.
    pub entities_updated: usize,

    /// Number of new relationships created.
    pub relationships_created: usize,

    /// Number of existing relationships updated.
    pub relationships_updated: usize,

    /// SPEC-047 P7d: entity updates skipped (KEEP saturated).
    pub entities_skipped_saturated: usize,

    /// SPEC-098: saturated KEEP still ensured relational spine (AGE skipped).
    pub entities_spine_ensured_saturated: usize,

    /// SPEC-047 P7d: relationship updates skipped (KEEP saturated).
    pub relationships_skipped_saturated: usize,

    /// Number of errors encountered.
    pub errors: usize,

    /// First underlying error message (LAW-5 — surface cause, not count-only).
    pub first_error: Option<String>,

    /// Graph/vector IDs written in this session — used for P-G5 saga compensation.
    pub artifacts: MergeArtifacts,
}

impl MergeStats {
    /// Increment `errors` and capture the first cause for persist/UI.
    pub fn record_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        if self.first_error.is_none() {
            self.first_error = Some(message);
        }
        self.errors += 1;
    }
}

/// IDs **created** during a single merge attempt (for rollback on failure).
///
/// SPEC-059: vector ID lists contain only rows **inserted** by this merge's
/// `upsert_report_created` (atomic). Shared entity/relationship vectors that
/// were updated must never appear here — compensate must not delete them.
#[derive(Debug, Clone, Default)]
pub struct MergeArtifacts {
    /// Entity vector IDs created (not updated) in this session.
    pub entity_vector_ids: Vec<String>,
    /// Relationship vector IDs created (not updated) in this session.
    pub relationship_vector_ids: Vec<String>,
    /// Graph node IDs created (not updated) in this session.
    pub graph_nodes_created: Vec<String>,
    /// Graph edge endpoints created (not updated) in this session.
    pub graph_edges_created: Vec<(String, String)>,
}

impl MergeStats {
    /// Get total entities processed.
    pub fn total_entities(&self) -> usize {
        self.entities_created + self.entities_updated
    }

    /// Get total relationships processed.
    pub fn total_relationships(&self) -> usize {
        self.relationships_created + self.relationships_updated
    }
}

/// Canonical entity key normalization (single source of truth: `prompts::normalizer`).
pub use crate::prompts::normalize_entity_name;

/// Merge two descriptions, avoiding duplication.
///
/// Retained for callers outside the P7a path; prefer
/// [`description_merge::decide_description_merge`] for graph updates.
#[allow(dead_code)]
fn merge_descriptions(existing: &str, new: &str, max_length: usize) -> String {
    if existing.is_empty() {
        return truncate_description(new, max_length);
    }

    if new.is_empty() || existing.contains(new) {
        return existing.to_string();
    }

    // Check if new content adds meaningful information
    let new_sentences: Vec<&str> = new.split(['.', '!', '?']).collect();
    let mut additions = Vec::new();

    for sentence in new_sentences {
        let sentence = sentence.trim();
        if !sentence.is_empty() && !existing.contains(sentence) {
            additions.push(sentence);
        }
    }

    if additions.is_empty() {
        return existing.to_string();
    }

    let combined = format!("{} {}", existing, additions.join(". "));
    truncate_description(&combined, max_length)
}

/// Truncate a description to a maximum length at sentence boundaries.
///
/// Delegates to [`description_merge::truncate_at_boundary`] (SSOT).
#[allow(dead_code)]
fn truncate_description(text: &str, max_length: usize) -> String {
    description_merge::truncate_at_boundary(text, max_length)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::{ExtractedEntity, ExtractedRelationship};
    use edgequake_storage::GraphStorageReadOps;

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(normalize_entity_name("John Doe"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("  Hello  World  "), "HELLO_WORLD");
        assert_eq!(normalize_entity_name("O'Brien"), "O'BRIEN");
        assert_eq!(normalize_entity_name("AI/ML"), "AI/ML");
        assert_eq!(normalize_entity_name("The Company"), "COMPANY");
    }

    #[test]
    fn test_merge_stats_record_error_keeps_first_cause() {
        let mut stats = MergeStats::default();
        stats.record_error("first boom");
        stats.record_error("second boom");
        assert_eq!(stats.errors, 2);
        assert_eq!(stats.first_error.as_deref(), Some("first boom"));
    }

    /// Parse→merge path must use the same key as extraction parsers (SPEC-017 P0).
    #[test]
    fn test_parse_merge_key_alignment() {
        use crate::prompts::normalize_entity_name as parser_normalize;
        let names = ["The Company", "John Doe", "the company", "O'Brien"];
        for name in names {
            assert_eq!(
                normalize_entity_name(name),
                parser_normalize(name),
                "merger and parser keys diverged for {:?}",
                name
            );
        }
    }

    // ── Fix #202: case-insensitive entity deduplication ──────────────────────

    /// WHY: Root cause of issue #202. When the LLM extracts "Systems Thinking"
    /// in one chunk and "systems thinking" in another, both must normalise to
    /// "SYSTEMS_THINKING" so only one graph node is created or updated.
    #[test]
    fn test_normalize_entity_name_case_variants_produce_same_key() {
        // All casing variants of the same phrase → identical key
        let variants = [
            "Systems Thinking",
            "systems thinking",
            "SYSTEMS THINKING",
            "Systems thinking",
        ];
        let keys: Vec<String> = variants.iter().map(|v| normalize_entity_name(v)).collect();
        let first = &keys[0];
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(
                key, first,
                "variant '{}' produced '{}', expected '{}'",
                variants[i], key, first
            );
        }
        assert_eq!(keys[0], "SYSTEMS_THINKING");
    }

    #[test]
    fn test_normalize_entity_name_real_world_case_variants() {
        // Real duplicates observed in issue #202 bug report
        assert_eq!(
            normalize_entity_name("Systems Theory"),
            normalize_entity_name("systems theory")
        );
        assert_eq!(
            normalize_entity_name("Chemical Plant"),
            normalize_entity_name("chemical plant")
        );
        assert_eq!(
            normalize_entity_name("Hazard"),
            normalize_entity_name("hazard")
        );
        // NOTE: "Organizations" vs "organization" produce DIFFERENT keys
        // (ORGANIZATIONS ≠ ORGANIZATION) — this is expected. normalize_entity_name
        // does NOT stem words; LLM should extract consistent forms.
        assert_eq!(normalize_entity_name("Organizations"), "ORGANIZATIONS");
        assert_eq!(normalize_entity_name("organization"), "ORGANIZATION");
    }

    #[test]
    fn test_normalize_entity_name_edge_cases() {
        assert_eq!(normalize_entity_name(""), "");
        assert_eq!(normalize_entity_name("---"), "---");
        assert_eq!(normalize_entity_name("rust"), "RUST");
        assert_eq!(normalize_entity_name("COVID 19"), "COVID_19");
        assert_eq!(normalize_entity_name("gpt-4o"), "GPT-4O");
        assert_eq!(normalize_entity_name("Hello   World"), "HELLO_WORLD");
        assert_eq!(normalize_entity_name("A\tB"), "A_B");
        assert_eq!(normalize_entity_name("A\nB"), "A_B");
    }

    #[test]
    fn test_normalize_entity_name_unicode_accented() {
        // Rust is_alphanumeric() returns true for Unicode accented chars,
        // so they are preserved (not stripped). The key insight is that
        // case variants still normalize to the same key.
        assert_eq!(normalize_entity_name("Rémi"), normalize_entity_name("rémi"));
        assert_eq!(normalize_entity_name("Rémi"), normalize_entity_name("RÉMI"));
        assert_eq!(normalize_entity_name("Rémi"), "RÉMI");
    }

    #[test]
    fn test_merge_descriptions() {
        assert_eq!(merge_descriptions("", "New text", 1000), "New text");
        assert_eq!(merge_descriptions("Existing", "", 1000), "Existing");
        assert_eq!(
            merge_descriptions("Existing text", "Existing text", 1000),
            "Existing text"
        );

        // New content should be appended
        let result = merge_descriptions("First sentence.", "Second sentence.", 1000);
        assert!(result.contains("First sentence"));
        assert!(result.contains("Second sentence"));
    }

    #[test]
    fn test_truncate_description() {
        let short = "Short text.";
        assert_eq!(truncate_description(short, 100), short);

        let long = "First sentence. Second sentence. Third sentence.";
        let truncated = truncate_description(long, 30);
        assert!(truncated.len() <= 30);
        assert!(truncated.ends_with('.'));
    }

    #[test]
    fn test_merge_stats() {
        let stats = MergeStats {
            entities_created: 5,
            entities_updated: 3,
            relationships_created: 10,
            relationships_updated: 2,
            ..Default::default()
        };

        assert_eq!(stats.total_entities(), 8);
        assert_eq!(stats.total_relationships(), 12);
    }

    /// SPEC-098: saturated KEEP still ensures relational spine (AGE skip preserved).
    #[tokio::test]
    async fn spec098_saturated_spine_ensure_stat() {
        use std::sync::Mutex;

        struct SpySink {
            calls: Mutex<Vec<String>>,
        }

        #[async_trait::async_trait]
        impl RelationalEntitySink for SpySink {
            async fn upsert_entity(
                &self,
                name: &str,
                _entity_type: &str,
                _description: &str,
                _tenant_id: Option<&str>,
                _workspace_id: Option<&str>,
                _source_chunk_ids: &[String],
            ) -> crate::error::Result<()> {
                self.calls.lock().unwrap().push(format!("upsert:{name}"));
                Ok(())
            }

            async fn remove_entity_sources(
                &self,
                _name: &str,
                _workspace_id: Option<&str>,
                _sources_to_remove: &[String],
                _remaining: &[String],
            ) -> crate::error::Result<()> {
                Ok(())
            }
        }

        let spy = Arc::new(SpySink {
            calls: Mutex::new(Vec::new()),
        });
        let graph = Arc::new(edgequake_storage::MemoryGraphStorage::new("spec098"));
        let vector = Arc::new(edgequake_storage::MemoryVectorStorage::new("spec098", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let config = MergerConfig {
            source_ids_limit_method: SourceIdsLimitMethod::Keep,
            max_source_ids_per_entity: 2,
            use_llm_summarization: false,
            ..Default::default()
        };

        let merger = KnowledgeGraphMerger::new(config, graph.clone(), vector)
            .with_relational_sink(spy.clone());

        for i in 0..2 {
            let entity = ExtractedEntity {
                name: "Saturated".to_string(),
                entity_type: "CONCEPT".to_string(),
                description: format!("seed {i}"),
                importance: 0.5,
                source_spans: vec![],
                source_chunk_ids: vec![format!("seed{i}")],
                embedding: None,
                source_document_id: None,
                source_file_path: None,
                display_name: None,
                page_num: None,
                figure_index: None,
                asset_id: None,
                mm_subtype: None,
            };
            let mut stats = MergeStats::default();
            merger
                .merge_entities_batch(vec![entity], &mut stats)
                .await
                .unwrap();
        }

        spy.calls.lock().unwrap().clear();

        let entity = ExtractedEntity {
            name: "Saturated".to_string(),
            entity_type: "CONCEPT".to_string(),
            description: "should not mutate graph".to_string(),
            importance: 0.5,
            source_spans: vec![],
            source_chunk_ids: vec!["brand-new".to_string()],
            embedding: None,
            source_document_id: None,
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        };
        let mut stats = MergeStats::default();
        merger
            .merge_entities_batch(vec![entity], &mut stats)
            .await
            .unwrap();

        assert!(
            stats.entities_skipped_saturated >= 1,
            "expected KEEP skip: {stats:?}"
        );
        assert!(
            stats.entities_spine_ensured_saturated >= 1,
            "SPEC-098: spine ensure on saturated: {stats:?}"
        );
        let calls = spy.calls.lock().unwrap().clone();
        assert!(
            calls.iter().any(|c| c.contains("SATURATED")),
            "saturated KEEP must still upsert spine, got {calls:?}"
        );
    }

    #[test]
    fn test_entity_source_tracking_serialization() {
        // Test that source tracking fields serialize correctly for storage
        let entity = ExtractedEntity::new("Sarah Chen", "PERSON", "Lead researcher")
            .with_source_chunk_id("chunk-001")
            .with_source_document_id("doc-abc123")
            .with_source_file_path("/documents/research.pdf");

        // Verify source tracking fields
        assert_eq!(entity.source_chunk_ids.len(), 1);
        assert_eq!(entity.source_chunk_ids[0], "chunk-001");
        assert_eq!(entity.source_document_id, Some("doc-abc123".to_string()));
        assert_eq!(
            entity.source_file_path,
            Some("/documents/research.pdf".to_string())
        );

        // Verify JSON serialization works
        let json = serde_json::json!({
            "source_chunk_ids": entity.source_chunk_ids,
            "source_document_id": entity.source_document_id,
            "source_file_path": entity.source_file_path,
        });

        assert!(json.get("source_chunk_ids").unwrap().is_array());
        assert_eq!(
            json.get("source_document_id").unwrap().as_str(),
            Some("doc-abc123")
        );
        assert_eq!(
            json.get("source_file_path").unwrap().as_str(),
            Some("/documents/research.pdf")
        );
    }

    #[test]
    fn test_relationship_source_tracking_serialization() {
        // Test that source tracking fields serialize correctly for storage
        let rel = ExtractedRelationship::new("Alice", "Bob", "KNOWS")
            .with_description("Alice knows Bob from work")
            .with_source_chunk_id("chunk-005")
            .with_source_document_id("doc-xyz789")
            .with_source_file_path("/documents/team.md");

        // Verify source tracking fields (049: Vec + singular mirror)
        assert_eq!(rel.source_chunk_id, Some("chunk-005".to_string()));
        assert_eq!(rel.all_source_chunk_ids(), vec!["chunk-005".to_string()]);
        assert_eq!(rel.source_document_id, Some("doc-xyz789".to_string()));
        assert_eq!(rel.source_file_path, Some("/documents/team.md".to_string()));

        // Verify JSON serialization works
        let json = serde_json::json!({
            "source_chunk_ids": rel.all_source_chunk_ids(),
            "source_document_id": rel.source_document_id,
            "source_file_path": rel.source_file_path,
        });

        assert!(json.get("source_chunk_ids").unwrap().is_array());
        assert_eq!(
            json.get("source_document_id").unwrap().as_str(),
            Some("doc-xyz789")
        );
        assert_eq!(
            json.get("source_file_path").unwrap().as_str(),
            Some("/documents/team.md")
        );
    }

    // ── SPEC-021 P3-01: RelationalEntitySink tests ───────────────────────────

    /// Verify NoopEntitySink always returns Ok (backward-compatible default).
    #[tokio::test]
    async fn test_noop_sink_always_ok() {
        let sink = NoopEntitySink;
        let result = sink
            .upsert_entity(
                "APPLE_INC",
                "ORGANIZATION",
                "Technology company",
                Some("tenant-1"),
                Some("workspace-1"),
                &["doc-1-chunk-0".to_string()],
            )
            .await;
        assert!(result.is_ok(), "NoopEntitySink must never fail");

        let result2 = sink
            .remove_entity_sources(
                "APPLE_INC",
                Some("workspace-1"),
                &["doc-1-chunk-0".to_string()],
                &[],
            )
            .await;
        assert!(result2.is_ok(), "NoopEntitySink must never fail");
    }

    /// Verify that a recording sink captures the expected calls from merge_entity.
    #[tokio::test]
    async fn test_relational_sink_called_during_merge() {
        use std::sync::Mutex;

        // Spy sink that records calls
        struct SpySink {
            calls: Mutex<Vec<String>>,
        }

        #[async_trait::async_trait]
        impl RelationalEntitySink for SpySink {
            async fn upsert_entity(
                &self,
                name: &str,
                _entity_type: &str,
                _description: &str,
                _tenant_id: Option<&str>,
                _workspace_id: Option<&str>,
                _source_chunk_ids: &[String],
            ) -> crate::error::Result<()> {
                self.calls.lock().unwrap().push(format!("upsert:{name}"));
                Ok(())
            }

            async fn remove_entity_sources(
                &self,
                name: &str,
                _workspace_id: Option<&str>,
                _sources_to_remove: &[String],
                remaining: &[String],
            ) -> crate::error::Result<()> {
                self.calls
                    .lock()
                    .unwrap()
                    .push(format!("remove:{name}:remaining={}", remaining.len()));
                Ok(())
            }
        }

        let spy = Arc::new(SpySink {
            calls: Mutex::new(Vec::new()),
        });
        let spy_clone = Arc::clone(&spy);

        // Build merger with the spy sink
        let graph = Arc::new(edgequake_storage::MemoryGraphStorage::new("test"));
        let vector = Arc::new(edgequake_storage::MemoryVectorStorage::new("test", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph, vector)
            .with_relational_sink(spy_clone);

        let entity = ExtractedEntity {
            name: "Apple Inc".to_string(),
            entity_type: "ORGANIZATION".to_string(),
            description: "Tech company".to_string(),
            importance: 0.8,
            source_spans: vec![],
            source_chunk_ids: vec!["doc-1-chunk-0".to_string()],
            embedding: None,
            source_document_id: None,
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        };

        let mut stats = super::MergeStats::default();
        merger
            .merge_entities_batch(vec![entity], &mut stats)
            .await
            .unwrap();

        let calls = spy.calls.lock().unwrap().clone();
        assert!(
            calls.iter().any(|c| c.starts_with("upsert:APPLE_INC")),
            "Expected sink.upsert_entity to be called with APPLE_INC, got: {calls:?}"
        );
    }

    /// Verify KnowledgeGraphMerger::with_relational_sink() sets the sink.
    #[test]
    fn test_merger_builder_sink() {
        let graph = Arc::new(edgequake_storage::MemoryGraphStorage::new("test"));
        let vector = Arc::new(edgequake_storage::MemoryVectorStorage::new("test", 4));
        let sink: Arc<dyn RelationalEntitySink> = Arc::new(NoopEntitySink);
        let _merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph, vector)
            .with_relational_sink(sink);
        // If it compiles and doesn't panic, the builder works
    }

    // ── SPEC-032: Global batch + similarity gate tests ────────────────────────

    /// SPEC-032 W-03: Entities from multiple ExtractionResults (chunks) for the same
    /// document must be deduplicated in-memory before the graph write.
    /// This means a single-entity graph should see exactly 1 node even when the same
    /// entity appears in 3 chunks.
    #[tokio::test]
    async fn test_global_batch_deduplication_across_chunks() {
        let graph = Arc::new(edgequake_storage::MemoryGraphStorage::new("test"));
        let vector = Arc::new(edgequake_storage::MemoryVectorStorage::new("test", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector);

        // Same entity in 3 "chunks" (ExtractionResults)
        let make_result = |chunk_id: &str| {
            let mut r = crate::extractor::ExtractionResult::new(chunk_id);
            r.entities.push(ExtractedEntity {
                name: "Alice".to_string(),
                entity_type: "PERSON".to_string(),
                description: format!("Alice from chunk {}", chunk_id),
                importance: 0.8,
                source_spans: vec![],
                source_chunk_ids: vec![chunk_id.to_string()],
                embedding: None,
                source_document_id: None,
                source_file_path: None,
                display_name: None,
                page_num: None,
                figure_index: None,
                asset_id: None,
                mm_subtype: None,
            });
            r
        };

        let results = vec![
            make_result("chunk-0"),
            make_result("chunk-1"),
            make_result("chunk-2"),
        ];

        let stats = merger.merge(results).await.unwrap();

        // Should create exactly 1 entity node (deduplicated)
        assert_eq!(
            stats.entities_created, 1,
            "Expected 1 entity created (deduplicated across 3 chunks), got {}",
            stats.entities_created
        );
        assert_eq!(
            stats.entities_updated, 0,
            "No updates expected on first merge"
        );

        // Second merge: same entity should be updated, not created
        let results2 = vec![make_result("chunk-3")];
        let stats2 = merger.merge(results2).await.unwrap();
        assert_eq!(stats2.entities_created, 0, "Entity should already exist");
        assert_eq!(
            stats2.entities_updated, 1,
            "Entity should be updated on second merge"
        );
    }

    /// SPEC-032 W-04: merge_with_progress emits progress callbacks for each phase.
    #[tokio::test]
    async fn test_merge_with_progress_emits_phases() {
        use std::sync::Mutex;

        let graph = Arc::new(edgequake_storage::MemoryGraphStorage::new("test"));
        let vector = Arc::new(edgequake_storage::MemoryVectorStorage::new("test", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph, vector);

        let phases_seen: Arc<Mutex<Vec<MergePhase>>> = Arc::new(Mutex::new(Vec::new()));
        let phases_clone = Arc::clone(&phases_seen);

        let cb: MergeProgressCallback = Box::new(move |p: MergeProgress| {
            phases_clone.lock().unwrap().push(p.phase);
        });

        let entity = ExtractedEntity {
            name: "Bob".to_string(),
            entity_type: "PERSON".to_string(),
            description: "Bob is a person".to_string(),
            importance: 0.5,
            source_spans: vec![],
            source_chunk_ids: vec!["c-0".to_string()],
            embedding: None,
            source_document_id: None,
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        };

        let mut result = crate::extractor::ExtractionResult::new("c-0");
        result.entities.push(entity);
        let results = vec![result];

        merger
            .merge_with_progress(results, Some(&cb))
            .await
            .unwrap();

        let phases = phases_seen.lock().unwrap().clone();
        // Typed default: EntityGraph precedes EntityVectors; EntityVectors is
        // only emitted when the batch has embeddings. This fixture has none.
        assert!(
            phases.contains(&MergePhase::EntityGraph),
            "Expected EntityGraph phase"
        );
        assert!(
            phases.contains(&MergePhase::Finalizing),
            "Expected Finalizing phase"
        );
        if edgequake_storage::vector_backend::vector_backend_reads_typed(
            edgequake_storage::vector_backend_from_env(),
        ) {
            assert!(
                !phases.contains(&MergePhase::EntityVectors),
                "EntityVectors should be skipped when embeddings are absent under typed authority"
            );
        }
    }

    /// SPEC-032 W-06: Similarity gate — identical descriptions must not call LLM.
    #[test]
    fn test_description_similarity_gate() {
        use crate::merger::entity::description_similarity;

        // Identical → similarity = 1.0
        assert_eq!(
            description_similarity("Alice is a researcher", "Alice is a researcher"),
            1.0
        );

        // Completely different
        let sim = description_similarity("apple tree fruit", "database network protocol");
        assert!(
            sim < 0.1,
            "Expected very low similarity for unrelated texts, got {}",
            sim
        );

        // Near-identical (one word added) — should be above 0.6
        // Jaccard("Alice is a researcher at MIT", "Alice is a researcher at MIT who studies AI")
        // Shared: {Alice, is, a, researcher, at, MIT} = 6
        // Union:  {Alice, is, a, researcher, at, MIT, who, studies, AI} = 9
        // Similarity = 6/9 ≈ 0.667
        let sim2 = description_similarity(
            "Alice is a researcher at MIT",
            "Alice is a researcher at MIT who studies AI",
        );
        assert!(
            sim2 > 0.6,
            "Expected similarity > 0.6 for near-identical texts, got {}",
            sim2
        );

        // Empty strings
        assert_eq!(description_similarity("", ""), 0.0);
        assert_eq!(description_similarity("hello", ""), 0.0);
    }

    /// SPEC-032 W-06: MergerConfig picks up EDGEQUAKE_MERGE_SIMILARITY_THRESHOLD env var.
    #[test]
    fn test_merger_config_similarity_threshold_default() {
        // Default (no env var set in test): 0.85
        let config = MergerConfig::default();
        // The env var may or may not be set in CI, just verify it's a valid float in [0,1]
        assert!(
            config.description_similarity_threshold >= 0.0
                && config.description_similarity_threshold <= 1.0,
            "Similarity threshold must be in [0,1], got {}",
            config.description_similarity_threshold
        );
    }

    /// D-30: distinct relation types between the same endpoints both persist.
    #[tokio::test]
    async fn test_relationship_multigraph_two_types_persist() {
        let graph = Arc::new(edgequake_storage::MemoryGraphStorage::new("rel-dedupe"));
        let vector = Arc::new(edgequake_storage::MemoryVectorStorage::new("rel-dedupe", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector);

        let mut r0 = crate::extractor::ExtractionResult::new("chunk-0");
        r0.entities.push(ExtractedEntity {
            name: "Alice".to_string(),
            entity_type: "PERSON".to_string(),
            description: "Alice".to_string(),
            importance: 0.8,
            source_spans: vec![],
            source_chunk_ids: vec!["chunk-0".to_string()],
            embedding: None,
            source_document_id: None,
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        });
        r0.entities.push(ExtractedEntity {
            name: "Bob".to_string(),
            entity_type: "PERSON".to_string(),
            description: "Bob".to_string(),
            importance: 0.8,
            source_spans: vec![],
            source_chunk_ids: vec!["chunk-0".to_string()],
            embedding: None,
            source_document_id: None,
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        });
        r0.relationships.push(
            ExtractedRelationship::new("Alice", "Bob", "KNOWS")
                .with_description("knows")
                .with_source_chunk_id("chunk-0"),
        );

        let mut r1 = crate::extractor::ExtractionResult::new("chunk-1");
        r1.relationships.push(
            ExtractedRelationship::new("Alice", "Bob", "WORKS_WITH")
                .with_description("works with Alice and Bob — longer")
                .with_weight(0.9)
                .with_source_chunk_id("chunk-1"),
        );

        let stats = merger
            .merge(vec![r0, r1])
            .await
            .expect("merge must succeed");
        assert_eq!(
            stats.errors, 0,
            "multigraph merge must not count as merge errors"
        );
        assert_eq!(
            stats.relationships_created, 2,
            "expected two edges (KNOWS + WORKS_WITH), got {}",
            stats.relationships_created
        );

        let alice_bob = graph
            .get_edges_for_nodes_batch(&["ALICE".into(), "BOB".into()])
            .await
            .unwrap();
        assert_eq!(
            alice_bob.len(),
            2,
            "expected multigraph pair, got {alice_bob:?}"
        );
        let types: Vec<_> = alice_bob
            .iter()
            .filter_map(|e| e.properties.get("relation_type").and_then(|v| v.as_str()))
            .collect();
        assert!(types.contains(&"KNOWS"), "{types:?}");
        assert!(types.contains(&"WORKS_WITH"), "{types:?}");
    }

    /// SPEC-058: shared entity vector must not appear in compensate artifacts.
    #[tokio::test]
    async fn spec058_shared_entity_vector_excluded_from_compensate_artifacts() {
        let graph = Arc::new(edgequake_storage::MemoryGraphStorage::new("spec058-share"));
        let vector = Arc::new(edgequake_storage::MemoryVectorStorage::new(
            "spec058-share",
            4,
        ));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();

        let emb = vec![0.1, 0.2, 0.3, 0.4];
        let entity_id = edgequake_storage::EntityId::new("Shared Person");
        let vector_id = entity_id.as_vector_id();

        // Doc A already wrote the shared entity embedding.
        vector
            .upsert(&[(
                vector_id.clone(),
                emb.clone(),
                serde_json::json!({"type": "entity"}),
            )])
            .await
            .unwrap();

        let merger = KnowledgeGraphMerger::new(MergerConfig::default(), graph, vector.clone());
        let mut r = crate::extractor::ExtractionResult::new("chunk-b");
        r.entities.push(ExtractedEntity {
            name: "Shared Person".to_string(),
            entity_type: "PERSON".to_string(),
            description: "From doc B".to_string(),
            importance: 0.9,
            source_spans: vec![],
            source_chunk_ids: vec!["chunk-b".to_string()],
            embedding: Some(emb),
            source_document_id: Some("doc-b".to_string()),
            source_file_path: None,
            display_name: None,
            page_num: None,
            figure_index: None,
            asset_id: None,
            mm_subtype: None,
        });

        let stats = merger.merge(vec![r]).await.expect("merge");
        assert!(
            !stats.artifacts.entity_vector_ids.contains(&vector_id),
            "shared entity vector must not be in compensate list: {:?}",
            stats.artifacts.entity_vector_ids
        );
        assert!(
            vector.get_by_id(&vector_id).await.unwrap().is_some(),
            "shared entity embedding must remain after merge"
        );
    }

    /// SPEC-059: second upsert_report_created of same ID returns empty created list.
    #[tokio::test]
    async fn spec059_upsert_report_created_second_call_empty() {
        use edgequake_storage::VectorStorage;
        let vector = edgequake_storage::MemoryVectorStorage::new("spec059-report", 4);
        vector.initialize().await.unwrap();
        let emb = vec![0.1, 0.2, 0.3, 0.4];
        let id = "ent-shared".to_string();
        let first = vector
            .upsert_report_created(&[(
                id.clone(),
                emb.clone(),
                serde_json::json!({"type": "entity"}),
            )])
            .await
            .unwrap();
        assert_eq!(first, vec![id.clone()]);
        let second = vector
            .upsert_report_created(&[(
                id.clone(),
                emb,
                serde_json::json!({"type": "entity", "v": 2}),
            )])
            .await
            .unwrap();
        assert!(
            second.is_empty(),
            "update must not report as created: {second:?}"
        );
    }

    /// SPEC-130 T4: relationship fail-closed hint names sink-returned identity.
    #[test]
    fn contract_spec130_rel_mirror_hint_names_sink_id() {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/merger/mod.rs"));
        assert!(
            src.contains("sink-returned relationships.id"),
            "RelVectors fail-closed hint must name sink-returned relationships.id"
        );
        assert!(
            src.contains("PostgresEntitySink wrote the entity spine before fleet mirror"),
            "EntityVectors hint must keep entity-spine language"
        );
    }

    /// SPEC-130 T1: typed path sequences RelGraph before RelVectors in source.
    #[test]
    fn contract_spec130_typed_relgraph_before_relvectors() {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/merger/mod.rs"));
        let typed_comment = src
            .find("Typed order:  EntityGraph → EntityVectors → RelGraph → RelVectors")
            .expect("typed order comment");
        let rel_graph = src[typed_comment..]
            .find("merge_relationships_batch")
            .expect("RelGraph call");
        let rel_vectors = src[typed_comment..]
            .find("if typed_authority && !all_rel_vectors.is_empty()")
            .expect("typed RelVectors gate");
        assert!(
            rel_graph < rel_vectors,
            "RelGraph must appear before typed RelVectors in merge_with_progress"
        );
    }
}
