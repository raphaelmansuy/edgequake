//! Entity / relationship spine and fleet vectors for the durable-commit path.
//!
//! # WHY this exists
//!
//! The durable committer (`IngestionAuthority::DurableCommitter`) writes chunks,
//! graph facts and **chunk** embeddings in one transaction and returns before
//! the merger runs. Entity and relationship embeddings are not part of that
//! command, so without this step the `entities` / `relationships` spine and the
//! `entity_embeddings` / `relationship_embeddings` fleet tables stay empty.
//! Mix/Local/Global then lose their vector arms and answer quality drops.
//!
//! This module runs only the relational half of the merger: spine rows, then
//! the same fleet mirror used by the legacy merge. It never touches the graph
//! (projection owns that).

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use edgequake_storage::{EntityId, GraphStorage, VectorStorage};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use uuid::Uuid;

use super::merge_limits::fleet_vector_concurrency_from_env;
use crate::error::Result;
use crate::extractor::{ExtractedRelationship, ExtractionResult};
use crate::merger::{
    EntitySinkRow, KnowledgeGraphMerger, MergeArtifacts, MergePhase, RelationshipSinkRow,
};

/// One vector row: `(vector id, embedding, metadata)`.
type VectorRow = (String, Vec<f32>, serde_json::Value);

/// Document-wide counts the progress phases are measured against.
#[derive(Clone, Copy)]
struct VectorTotals {
    entities: usize,
    relationships: usize,
}

/// Counts written by [`KnowledgeGraphMerger::persist_fleet_vectors`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FleetVectorStats {
    pub entity_rows: usize,
    pub relationship_rows: usize,
    pub entity_vectors: usize,
    pub relationship_vectors: usize,
}

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> KnowledgeGraphMerger<G, V> {
    /// Write the relational spine and fleet vectors for `results`.
    ///
    /// Order matters: spine rows first (fleet vectors FK to them), entities
    /// before relationships (relationship rows FK to both endpoints).
    pub async fn persist_fleet_vectors(
        &self,
        results: &[ExtractionResult],
    ) -> Result<FleetVectorStats> {
        let mut stats = FleetVectorStats::default();

        let entity_rows = self.durable_entity_rows(results);
        stats.entity_rows = entity_rows.len();
        if !entity_rows.is_empty() {
            self.relational_sink
                .upsert_entities_batch(&entity_rows)
                .await?;
        }

        let all_rels: Vec<ExtractedRelationship> = results
            .iter()
            .flat_map(|r| r.relationships.iter().cloned())
            .collect();
        let rel_rows = self.durable_relationship_rows(results);
        stats.relationship_rows = rel_rows.len();
        let rel_report = if rel_rows.is_empty() {
            Default::default()
        } else {
            self.relational_sink
                .upsert_relationships_batch(&rel_rows)
                .await?
        };

        let entity_batch = self.collect_entity_vector_batch(results);
        stats.entity_vectors = entity_batch.len();
        let rel_batch = self.collect_relationship_vector_batch(&all_rels);
        stats.relationship_vectors = rel_batch.len();

        let totals = VectorTotals {
            entities: results.iter().map(|r| r.entities.len()).sum(),
            relationships: all_rels.len(),
        };
        let known = (!rel_report.ids.is_empty()).then_some(&rel_report.ids);
        self.upsert_fleet_slices(&entity_batch, &rel_batch, known, totals)
            .await?;

        Ok(stats)
    }

    /// Write entity and relationship vector slices with bounded concurrency.
    ///
    /// # WHY concurrent
    ///
    /// Each slice is one idempotent `INSERT .. ON CONFLICT` into an HNSW-indexed
    /// table, and the HNSW insert (not the round trip) is the cost: sequential
    /// slices left 3 pool connections idle while one walked the graph. Entity
    /// and relationship rows go to different tables, and ids are already unique
    /// per batch (`dedupe_*_by_*`), so slices never contend on the same row.
    /// Concurrency is capped by `EDGEQUAKE_FLEET_VECTOR_CONCURRENCY` so one
    /// document cannot starve the ingest pool.
    async fn upsert_fleet_slices(
        &self,
        entity_batch: &[VectorRow],
        rel_batch: &[VectorRow],
        known_relationship_ids: Option<&HashMap<String, Uuid>>,
        totals: VectorTotals,
    ) -> Result<()> {
        let slice_rows = edgequake_storage::vector_upsert_chunk_size();
        let limit = fleet_vector_concurrency_from_env();
        let work = entity_batch
            .chunks(slice_rows)
            .map(|slice| (slice, true))
            .chain(rel_batch.chunks(slice_rows).map(|slice| (slice, false)));

        let mut in_flight = FuturesUnordered::new();
        for (slice, as_entities) in work {
            if in_flight.len() >= limit {
                if let Some(done) = in_flight.next().await {
                    done?;
                }
            }
            in_flight.push(self.upsert_fleet_slice(
                slice,
                as_entities,
                known_relationship_ids,
                totals,
            ));
        }
        while let Some(done) = in_flight.next().await {
            done?;
        }
        Ok(())
    }

    /// One slice of entity (`as_entities`) or relationship vectors.
    async fn upsert_fleet_slice(
        &self,
        slice: &[VectorRow],
        as_entities: bool,
        known_relationship_ids: Option<&HashMap<String, Uuid>>,
        totals: VectorTotals,
    ) -> Result<()> {
        let (phase, known) = if as_entities {
            (MergePhase::EntityVectors, None)
        } else {
            (MergePhase::RelationshipVectors, known_relationship_ids)
        };
        // Artifacts only feed compensate, which this path never runs.
        let mut artifacts = MergeArtifacts::default();
        self.upsert_vectors_chunked(
            slice,
            None,
            phase,
            totals.entities,
            totals.relationships,
            0,
            0,
            as_entities,
            &mut artifacts,
            known,
        )
        .await
    }

    /// One spine row per distinct entity, plus an `UNKNOWN` row for every
    /// relationship endpoint that no extraction named as an entity.
    fn durable_entity_rows(&self, results: &[ExtractionResult]) -> Vec<EntitySinkRow> {
        let mut rows: Vec<EntitySinkRow> = Vec::new();
        let mut named: HashSet<String> = HashSet::new();

        for extraction in results {
            let chunk = extraction.source_chunk_id.as_str();
            for entity in &extraction.entities {
                let name = EntityId::new(&entity.name).as_graph_node_id().to_string();
                if name.is_empty() {
                    continue;
                }
                named.insert(name.clone());
                rows.push(EntitySinkRow {
                    name,
                    entity_type: entity.entity_type.clone(),
                    description: entity.description.clone(),
                    tenant_id: self.tenant_id.clone(),
                    workspace_id: self.workspace_id.clone(),
                    source_chunk_ids: chunk_ids_for(&entity.source_chunk_ids, chunk),
                });
            }
        }

        for extraction in results {
            let chunk = extraction.source_chunk_id.as_str();
            for rel in &extraction.relationships {
                for raw in [rel.source.as_str(), rel.target.as_str()] {
                    let name = EntityId::new(raw).as_graph_node_id().to_string();
                    if name.is_empty() || !named.insert(name.clone()) {
                        continue;
                    }
                    rows.push(EntitySinkRow {
                        name,
                        entity_type: "UNKNOWN".to_string(),
                        description: rel.description.clone(),
                        tenant_id: self.tenant_id.clone(),
                        workspace_id: self.workspace_id.clone(),
                        source_chunk_ids: chunk_ids_for(&rel.all_source_chunk_ids(), chunk),
                    });
                }
            }
        }
        rows
    }

    /// One spine row per distinct `(source, target, type)`; self-loops skipped.
    /// A pair named by several chunks keeps the union of their chunk keys: the
    /// keys are the row's document lineage, so a first-wins dedupe would drop
    /// every document but one.
    fn durable_relationship_rows(&self, results: &[ExtractionResult]) -> Vec<RelationshipSinkRow> {
        let ws = self.workspace_id.as_deref();
        let mut at: HashMap<(String, String, String), usize> = HashMap::new();
        let mut out: Vec<RelationshipSinkRow> = Vec::new();
        for extraction in results {
            let chunk = extraction.source_chunk_id.as_str();
            for rel in &extraction.relationships {
                let source = EntityId::new(&rel.source).graph_node_id_for_workspace(ws);
                let target = EntityId::new(&rel.target).graph_node_id_for_workspace(ws);
                if source.is_empty() || target.is_empty() || source == target {
                    continue;
                }
                let rel_type = edgequake_storage::normalize_relation_type_str(&rel.relation_type);
                let keys = chunk_ids_for(&rel.all_source_chunk_ids(), chunk);
                match at.entry((source.clone(), target.clone(), rel_type.clone())) {
                    Entry::Occupied(seen) => {
                        out[*seen.get()].absorb_chunk_keys(keys);
                    }
                    Entry::Vacant(slot) => {
                        slot.insert(out.len());
                        out.push(RelationshipSinkRow {
                            source_name: source,
                            target_name: target,
                            relation_type: rel_type,
                            description: rel.description.clone(),
                            weight: rel.weight,
                            tenant_id: self.tenant_id.clone(),
                            workspace_id: self.workspace_id.clone(),
                            source_chunk_ids: keys,
                        });
                    }
                }
            }
        }
        out
    }
}

/// Entity-level chunk ids when present, else the extraction's own chunk.
fn chunk_ids_for(own: &[String], extraction_chunk: &str) -> Vec<String> {
    if !own.is_empty() {
        return own.to_vec();
    }
    if extraction_chunk.is_empty() {
        return Vec::new();
    }
    vec![extraction_chunk.to_string()]
}
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use edgequake_storage::{
        GraphStorage, GraphStorageAnalyticsOps, MemoryGraphStorage, MemoryVectorStorage,
        VectorStorage,
    };

    use super::*;
    use crate::extractor::ExtractedEntity;
    use crate::merger::{MergerConfig, RelationalEntitySink, RelationshipSinkReport};

    #[derive(Default)]
    struct RecordingSink {
        entities: Mutex<Vec<EntitySinkRow>>,
        relationships: Mutex<Vec<RelationshipSinkRow>>,
    }

    #[async_trait]
    impl RelationalEntitySink for RecordingSink {
        async fn upsert_entity(
            &self,
            _name: &str,
            _entity_type: &str,
            _description: &str,
            _tenant_id: Option<&str>,
            _workspace_id: Option<&str>,
            _source_chunk_ids: &[String],
        ) -> Result<()> {
            Ok(())
        }

        async fn upsert_entities_batch(&self, rows: &[EntitySinkRow]) -> Result<()> {
            self.entities.lock().unwrap().extend_from_slice(rows);
            Ok(())
        }

        async fn upsert_relationships_batch(
            &self,
            rows: &[RelationshipSinkRow],
        ) -> Result<RelationshipSinkReport> {
            self.relationships.lock().unwrap().extend_from_slice(rows);
            Ok(RelationshipSinkReport::default())
        }

        async fn remove_entity_sources(
            &self,
            _name: &str,
            _workspace_id: Option<&str>,
            _sources_to_remove: &[String],
            _remaining: &[String],
        ) -> Result<()> {
            Ok(())
        }
    }

    async fn merger_with(
        sink: Arc<RecordingSink>,
    ) -> (
        KnowledgeGraphMerger<MemoryGraphStorage, MemoryVectorStorage>,
        Arc<MemoryGraphStorage>,
    ) {
        let graph = Arc::new(MemoryGraphStorage::new("durable-fleet"));
        let vector = Arc::new(MemoryVectorStorage::new("durable-fleet", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();
        let config = MergerConfig {
            use_llm_summarization: false,
            ..Default::default()
        };
        let merger = KnowledgeGraphMerger::new(config, graph.clone(), vector)
            .with_relational_sink(sink)
            .with_tenant_context(None, None);
        (merger, graph)
    }

    fn sample() -> Vec<ExtractionResult> {
        let mut a = ExtractionResult::new("doc-chunk-0");
        a.entities.push(ExtractedEntity::new(
            "Lymphoma",
            "DISEASE",
            "A blood cancer",
        ));
        a.relationships.push(ExtractedRelationship::new(
            "Lymphoma",
            "Biopsy",
            "DIAGNOSED_BY",
        ));
        let mut b = ExtractionResult::new("doc-chunk-1");
        b.entities
            .push(ExtractedEntity::new("lymphoma", "DISEASE", "Again"));
        b.relationships
            .push(ExtractedRelationship::new("Lymphoma", "Lymphoma", "SELF"));
        vec![a, b]
    }

    #[tokio::test]
    async fn spine_rows_cover_named_entities_and_relationship_endpoints() {
        let sink = Arc::new(RecordingSink::default());
        let (merger, _graph) = merger_with(sink.clone()).await;

        let stats = merger.persist_fleet_vectors(&sample()).await.unwrap();

        let names: HashSet<String> = sink
            .entities
            .lock()
            .unwrap()
            .iter()
            .map(|r| r.name.clone())
            .collect();
        assert!(names.contains("LYMPHOMA"), "named entity: {names:?}");
        assert!(
            names.contains("BIOPSY"),
            "unnamed endpoint gets a row: {names:?}"
        );
        assert_eq!(stats.entity_rows, sink.entities.lock().unwrap().len());
    }

    #[tokio::test]
    async fn relationship_rows_skip_self_loops_and_duplicates() {
        let sink = Arc::new(RecordingSink::default());
        let (merger, _graph) = merger_with(sink.clone()).await;

        let stats = merger.persist_fleet_vectors(&sample()).await.unwrap();

        assert_eq!(stats.relationship_rows, 1, "self-loop must be dropped");
        let rels = sink.relationships.lock().unwrap();
        assert_eq!(rels[0].relation_type, "DIAGNOSED_BY");
    }

    #[tokio::test]
    async fn a_pair_named_by_several_chunks_keeps_every_chunk_key() {
        let sink = Arc::new(RecordingSink::default());
        let (merger, _graph) = merger_with(sink.clone()).await;
        let mut a = ExtractionResult::new("docA-chunk-0");
        a.relationships.push(ExtractedRelationship::new(
            "Lymphoma",
            "Biopsy",
            "DIAGNOSED_BY",
        ));
        let mut b = ExtractionResult::new("docB-chunk-3");
        b.relationships.push(ExtractedRelationship::new(
            "lymphoma",
            "biopsy",
            "diagnosed_by",
        ));
        // The relationship's own provenance wins over the extraction's chunk.
        let mut c = ExtractionResult::new("docC-chunk-1");
        c.relationships.push(
            ExtractedRelationship::new("Lymphoma", "Biopsy", "DIAGNOSED_BY")
                .with_source_chunk_id("docD-chunk-9"),
        );

        let stats = merger.persist_fleet_vectors(&[a, b, c]).await.unwrap();

        assert_eq!(stats.relationship_rows, 1);
        let rels = sink.relationships.lock().unwrap();
        assert_eq!(
            rels[0].source_chunk_ids,
            ["docA-chunk-0", "docB-chunk-3", "docD-chunk-9"],
            "first-wins dedupe would keep only docA"
        );
    }

    #[test]
    fn absorbing_chunk_keys_appends_new_keys_in_order_without_duplicates() {
        let mut row = RelationshipSinkRow {
            source_name: "A".into(),
            target_name: "B".into(),
            relation_type: "R".into(),
            description: String::new(),
            weight: 1.0,
            tenant_id: None,
            workspace_id: None,
            source_chunk_ids: vec!["k1".into()],
        };
        row.absorb_chunk_keys(["k2".to_string(), "k1".to_string(), "k3".to_string()]);
        assert_eq!(row.source_chunk_ids, ["k1", "k2", "k3"]);
    }

    #[tokio::test]
    async fn never_writes_graph_nodes() {
        let sink = Arc::new(RecordingSink::default());
        let (merger, graph) = merger_with(sink).await;

        merger.persist_fleet_vectors(&sample()).await.unwrap();

        assert!(
            graph.node_count().await.unwrap() == 0,
            "graph facts belong to the projection, not this step"
        );
    }

    /// `entities` embedded entities plus one embedded relationship between each pair.
    fn embedded(entities: usize) -> Vec<ExtractionResult> {
        let mut extraction = ExtractionResult::new("doc-chunk-0");
        for index in 0..entities {
            let mut entity = ExtractedEntity::new(format!("ENTITY_{index}"), "CONCEPT", "d");
            entity.embedding = Some(vec![0.1, 0.2, 0.3, 0.4]);
            extraction.entities.push(entity);
        }
        for index in 0..entities.saturating_sub(1) {
            let mut rel = ExtractedRelationship::new(
                format!("ENTITY_{index}"),
                format!("ENTITY_{}", index + 1),
                "RELATED_TO",
            );
            rel.embedding = Some(vec![0.4, 0.3, 0.2, 0.1]);
            extraction.relationships.push(rel);
        }
        vec![extraction]
    }

    #[tokio::test]
    async fn every_vector_slice_lands_when_the_batch_spans_several_slices() {
        // 2_100 entity rows + 2_099 relationship rows = 3 + 3 slices at 1_000 rows.
        let sink = Arc::new(RecordingSink::default());
        let graph = Arc::new(MemoryGraphStorage::new("durable-fleet-slices"));
        let vector = Arc::new(MemoryVectorStorage::new("durable-fleet-slices", 4));
        graph.initialize().await.unwrap();
        vector.initialize().await.unwrap();
        let config = MergerConfig {
            use_llm_summarization: false,
            ..Default::default()
        };
        let merger = KnowledgeGraphMerger::new(config, graph, vector.clone())
            .with_relational_sink(sink)
            .with_tenant_context(None, None);

        let stats = merger
            .persist_fleet_vectors(&embedded(2_100))
            .await
            .unwrap();

        assert_eq!(stats.entity_vectors, 2_100);
        assert_eq!(stats.relationship_vectors, 2_099);
        assert_eq!(
            vector.count().await.unwrap(),
            2_100 + 2_099,
            "concurrent slices must not drop or duplicate rows"
        );
    }

    #[tokio::test]
    async fn empty_input_writes_nothing() {
        let sink = Arc::new(RecordingSink::default());
        let (merger, _graph) = merger_with(sink.clone()).await;

        let stats = merger.persist_fleet_vectors(&[]).await.unwrap();

        assert_eq!(stats, FleetVectorStats::default());
        assert!(sink.entities.lock().unwrap().is_empty());
    }
}
