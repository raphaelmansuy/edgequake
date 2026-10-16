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

use std::collections::HashSet;

use edgequake_storage::{EntityId, GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::{ExtractedRelationship, ExtractionResult};
use crate::merger::{
    EntitySinkRow, KnowledgeGraphMerger, MergeArtifacts, MergePhase, RelationshipSinkRow,
};

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
        let rel_rows = self.durable_relationship_rows(&all_rels);
        stats.relationship_rows = rel_rows.len();
        let rel_report = if rel_rows.is_empty() {
            Default::default()
        } else {
            self.relational_sink
                .upsert_relationships_batch(&rel_rows)
                .await?
        };

        let total_entities: usize = results.iter().map(|r| r.entities.len()).sum();
        let mut artifacts = MergeArtifacts::default();

        let entity_batch = self.collect_entity_vector_batch(results);
        stats.entity_vectors = entity_batch.len();
        if !entity_batch.is_empty() {
            self.upsert_vectors_chunked(
                &entity_batch,
                None,
                MergePhase::EntityVectors,
                total_entities,
                all_rels.len(),
                0,
                0,
                true,
                &mut artifacts,
                None,
            )
            .await?;
        }

        let rel_batch = self.collect_relationship_vector_batch(&all_rels);
        stats.relationship_vectors = rel_batch.len();
        if !rel_batch.is_empty() {
            let known = (!rel_report.ids.is_empty()).then_some(&rel_report.ids);
            self.upsert_vectors_chunked(
                &rel_batch,
                None,
                MergePhase::RelationshipVectors,
                total_entities,
                all_rels.len(),
                0,
                0,
                false,
                &mut artifacts,
                known,
            )
            .await?;
        }

        Ok(stats)
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
    fn durable_relationship_rows(
        &self,
        rels: &[ExtractedRelationship],
    ) -> Vec<RelationshipSinkRow> {
        let ws = self.workspace_id.as_deref();
        let mut seen: HashSet<(String, String, String)> = HashSet::new();
        let mut out = Vec::with_capacity(rels.len());
        for rel in rels {
            let source = EntityId::new(&rel.source).graph_node_id_for_workspace(ws);
            let target = EntityId::new(&rel.target).graph_node_id_for_workspace(ws);
            if source.is_empty() || target.is_empty() || source == target {
                continue;
            }
            let rel_type = edgequake_storage::normalize_relation_type_str(&rel.relation_type);
            if !seen.insert((source.clone(), target.clone(), rel_type.clone())) {
                continue;
            }
            out.push(RelationshipSinkRow {
                source_name: source,
                target_name: target,
                relation_type: rel_type,
                description: rel.description.clone(),
                weight: rel.weight,
                tenant_id: self.tenant_id.clone(),
                workspace_id: self.workspace_id.clone(),
            });
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
    async fn never_writes_graph_nodes() {
        let sink = Arc::new(RecordingSink::default());
        let (merger, graph) = merger_with(sink).await;

        merger.persist_fleet_vectors(&sample()).await.unwrap();

        assert!(
            graph.node_count().await.unwrap() == 0,
            "graph facts belong to the projection, not this step"
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
