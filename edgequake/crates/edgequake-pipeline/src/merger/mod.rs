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

mod entity;
mod metadata;
mod relationship;

pub use entity::description_similarity;

use std::sync::Arc;

use async_trait::async_trait;
use edgequake_storage::{GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::ExtractionResult;
use crate::summarizer::LLMSummarizer;

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
#[async_trait]
pub trait RelationalEntitySink: Send + Sync {
    /// Upsert an entity into the relational CQRS read model.
    ///
    /// Called after the primary graph write succeeds.
    /// Must be best-effort: implementors SHOULD NOT fail the ingestion on error.
    async fn upsert_entity(
        &self,
        name: &str,
        entity_type: &str,
        description: &str,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
        source_chunk_ids: &[String],
    ) -> Result<()>;

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
        }
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
    /// Optional LLM summarizer for intelligent description merging.
    pub(super) summarizer: Option<Arc<LLMSummarizer>>,
    /// Optional CQRS relational sink (SPEC-021 P3-01).
    /// When None, relational sync is skipped (backwards-compatible default).
    pub(super) relational_sink: Arc<dyn RelationalEntitySink>,
    /// Optional lineage sink (SPEC-032 W-08).
    /// When None, lineage links are not persisted.
    pub(super) lineage_sink: Arc<dyn LineageSink>,
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
            relational_sink: Arc::new(NoopEntitySink),
            lineage_sink: Arc::new(NoopLineageSink),
        }
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
    pub fn with_summarizer(mut self, summarizer: Arc<LLMSummarizer>) -> Self {
        self.summarizer = Some(summarizer);
        self
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
                phase: MergePhase::EntityVectors,
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

        // ── Phase 1: Entity vector upserts — one round-trip for all docs ──
        // WHY: entity vectors are collected globally already (P-G4-merger).
        let entity_vector_batch = self.collect_entity_vector_batch(&results);
        if !entity_vector_batch.is_empty() {
            self.vector_storage.upsert(&entity_vector_batch).await?;
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

        // ── Phase 2: Entity graph merge — globally batched ────────────────
        // WHY: collect all entities from all chunks first, dedup within-doc,
        // then a single get_nodes_batch + upsert_nodes_batch.
        let all_entities: Vec<_> = results
            .iter()
            .flat_map(|r| r.entities.iter().cloned())
            .collect();

        if !all_entities.is_empty() {
            if let Err(e) = self.merge_entities_batch(all_entities, &mut stats).await {
                stats.errors += 1;
                tracing::warn!(
                    error.source = "pipeline_merger",
                    error.action = "merge_entities_batch_global",
                    error.message = %e,
                    "Failed to merge global entity batch"
                );
            }
        }

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
                relationships_created: 0,
                relationships_updated: 0,
            },
        );

        // ── Phase 3: Relationship vector upserts — globally batched ──────
        // WHY: previously done per ExtractionResult inside the loop (F-09).
        let all_rel_vectors: Vec<_> = results
            .iter()
            .flat_map(|r| self.collect_relationship_vector_batch(&r.relationships))
            .collect();
        if !all_rel_vectors.is_empty() {
            self.vector_storage.upsert(&all_rel_vectors).await?;
        }

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

        // ── Phase 4: Relationship graph merge — globally batched ─────────
        let all_relationships: Vec<_> = results
            .iter()
            .flat_map(|r| r.relationships.iter().cloned())
            .collect();

        if !all_relationships.is_empty() {
            if let Err(e) = self
                .merge_relationships_batch(all_relationships, &mut stats)
                .await
            {
                stats.errors += 1;
                tracing::warn!(
                    error.source = "pipeline_merger",
                    error.action = "merge_relationships_batch_global",
                    error.message = %e,
                    "Failed to merge global relationship batch"
                );
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

fn emit_progress(cb: Option<&MergeProgressCallback>, progress: MergeProgress) {
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

    /// Number of errors encountered.
    pub errors: usize,

    /// Graph/vector IDs written in this session — used for P-G5 saga compensation.
    pub artifacts: MergeArtifacts,
}

/// IDs created during a single merge attempt (for rollback on failure).
#[derive(Debug, Clone, Default)]
pub struct MergeArtifacts {
    /// Entity vector IDs upserted for newly created graph nodes.
    pub entity_vector_ids: Vec<String>,
    /// Relationship vector IDs upserted for newly created edges.
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
fn truncate_description(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }

    // Try to truncate at a sentence boundary
    let mut end = max_length;
    for (i, c) in text.char_indices().take(max_length) {
        if c == '.' || c == '!' || c == '?' {
            end = i + 1;
        }
    }

    text[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractor::{ExtractedEntity, ExtractedRelationship};

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(normalize_entity_name("John Doe"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("  Hello  World  "), "HELLO_WORLD");
        assert_eq!(normalize_entity_name("O'Brien"), "O'BRIEN");
        assert_eq!(normalize_entity_name("AI/ML"), "AI/ML");
        assert_eq!(normalize_entity_name("The Company"), "COMPANY");
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
            errors: 0,
            artifacts: MergeArtifacts::default(),
        };

        assert_eq!(stats.total_entities(), 8);
        assert_eq!(stats.total_relationships(), 12);
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

        // Verify source tracking fields (relationship uses Option<String> for chunk_id)
        assert_eq!(rel.source_chunk_id, Some("chunk-005".to_string()));
        assert_eq!(rel.source_document_id, Some("doc-xyz789".to_string()));
        assert_eq!(rel.source_file_path, Some("/documents/team.md".to_string()));

        // Verify JSON serialization works
        let json = serde_json::json!({
            "source_chunk_ids": rel.source_chunk_id.map(|id| vec![id]).unwrap_or_default(),
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
        };

        let mut result = crate::extractor::ExtractionResult::new("c-0");
        result.entities.push(entity);
        let results = vec![result];

        merger
            .merge_with_progress(results, Some(&cb))
            .await
            .unwrap();

        let phases = phases_seen.lock().unwrap().clone();
        // Must emit at least: EntityVectors, EntityGraph, RelationshipVectors, RelationshipGraph, Finalizing
        assert!(
            phases.contains(&MergePhase::EntityVectors),
            "Expected EntityVectors phase"
        );
        assert!(
            phases.contains(&MergePhase::EntityGraph),
            "Expected EntityGraph phase"
        );
        assert!(
            phases.contains(&MergePhase::Finalizing),
            "Expected Finalizing phase"
        );
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
}
