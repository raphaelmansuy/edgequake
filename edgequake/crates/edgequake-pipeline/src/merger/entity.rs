//! Entity merge, update, and creation logic for the knowledge graph.
//!
//! # SPEC-032 changes
//! - W-03: `merge_entities_batch` deduplicates within-document before graph read
//! - W-06: Similarity gate skips LLM summarizer when descriptions are near-identical

use std::collections::HashMap;

use edgequake_storage::{EntityId, GraphNode, GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::{ExtractedEntity, ExtractionResult};

use super::{merge_descriptions, metadata, MergeArtifacts, MergeStats};

/// Jaccard word-overlap similarity between two strings (pub for tests and external use).
pub fn description_similarity(a: &str, b: &str) -> f32 {
    if a == b {
        // Identical strings: if both empty → 0 (no overlap to measure); if same non-empty → 1.0
        return if a.is_empty() { 0.0 } else { 1.0 };
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
    let intersection = a_words.intersection(&b_words).count();
    let union = a_words.union(&b_words).count();
    if union == 0 {
        1.0
    } else {
        intersection as f32 / union as f32
    }
}

/// Threshold above which we skip LLM summarization (descriptions are near-identical).
/// Tunable via `MergerConfig.description_similarity_threshold`.
/// Default exposed here for tests; runtime value comes from `MergerConfig`.
#[cfg(test)]
pub(crate) const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.85;

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> super::KnowledgeGraphMerger<G, V> {
    /// Collect batched entity vector upserts for all extractions (P-G4-merger).
    pub(super) fn collect_entity_vector_batch(
        &self,
        results: &[ExtractionResult],
    ) -> Vec<(String, Vec<f32>, serde_json::Value)> {
        let mut batch = Vec::new();
        for result in results {
            for entity in &result.entities {
                let entity_id = EntityId::new(&entity.name);
                if entity_id.is_empty() {
                    continue;
                }
                let Some(embedding) = entity.embedding.as_ref() else {
                    continue;
                };
                let scope = metadata::TenantScope {
                    tenant_id: &self.tenant_id,
                    workspace_id: &self.workspace_id,
                };
                let metadata = metadata::entity_vector_metadata(entity, &entity_id, scope);
                batch.push((entity_id.as_vector_id(), embedding.clone(), metadata));
            }
        }
        batch
    }

    /// Merge entities with one `get_nodes_batch` + one `upsert_nodes_batch` (P-G4-graph).
    ///
    /// # SPEC-032 W-03: Within-document deduplication
    ///
    /// When the same entity appears in multiple chunks of the same document,
    /// the previous per-chunk loop would issue N get+upsert pairs.
    /// Now we deduplicate first: same-named entities within one call are merged
    /// in-memory (concatenating source_chunk_ids, taking the longer description)
    /// before a single get_nodes_batch reads the existing graph state.
    ///
    /// Edge case: entity with same name but different types → keep first type,
    /// append description (LightRAG convention: type is stable once set).
    pub(super) async fn merge_entities_batch(
        &self,
        entities: Vec<ExtractedEntity>,
        stats: &mut MergeStats,
    ) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }

        // ── Within-batch deduplication (SPEC-032 W-03) ───────────────────
        // If the same entity name appears in multiple chunks of this document,
        // merge them in-memory before hitting the database.
        // Use Vec<(key, entity)> to preserve first-seen order for determinism.
        let mut dedup_keys: Vec<String> = Vec::new();
        let mut dedup_map: HashMap<String, ExtractedEntity> = HashMap::new();

        for entity in entities {
            let entity_id = EntityId::new(&entity.name);
            if entity_id.is_empty() {
                tracing::warn!(
                    raw_name = %entity.name,
                    "Skipping entity with empty normalized name"
                );
                continue;
            }
            let key = entity_id.as_graph_node_id().to_string();
            if let Some(existing) = dedup_map.get_mut(&key) {
                // Merge descriptions: keep longer (richer)
                if entity.description.len() > existing.description.len() {
                    existing.description = entity.description.clone();
                }
                // Accumulate source chunks
                for cid in &entity.source_chunk_ids {
                    if !existing.source_chunk_ids.contains(cid) {
                        existing.source_chunk_ids.push(cid.clone());
                    }
                }
                // Merge source spans
                for span in &entity.source_spans {
                    if !existing.source_spans.contains(span) {
                        existing.source_spans.push(span.clone());
                    }
                }
                // Take max importance
                if entity.importance > existing.importance {
                    existing.importance = entity.importance;
                }
            } else {
                dedup_keys.push(key.clone());
                dedup_map.insert(key, entity);
            }
        }

        // Collect in insertion order (deterministic)
        let (keys, valid): (Vec<String>, Vec<ExtractedEntity>) = dedup_keys
            .into_iter()
            .filter_map(|k| dedup_map.remove(&k).map(|e| (k, e)))
            .unzip();

        if valid.is_empty() {
            return Ok(());
        }

        // Store entity types for relational sink (borrow before move into loop)
        let entity_types: Vec<String> = valid.iter().map(|e| e.entity_type.clone()).collect();
        let descriptions: Vec<String> = valid.iter().map(|e| e.description.clone()).collect();
        let source_chunk_ids: Vec<Vec<String>> =
            valid.iter().map(|e| e.source_chunk_ids.clone()).collect();

        let existing_map = self.graph_storage.get_nodes_batch(&keys).await?;
        let mut node_batch: Vec<(String, HashMap<String, serde_json::Value>)> =
            Vec::with_capacity(valid.len());

        for (i, (entity, key)) in valid.into_iter().zip(keys.iter()).enumerate() {
            match self
                .build_entity_node_batch_entry(&entity, existing_map.get(key), &mut stats.artifacts)
                .await
            {
                Ok((node_id, properties, is_new)) => {
                    node_batch.push((node_id, properties));
                    if is_new {
                        stats.entities_created += 1;
                    } else {
                        stats.entities_updated += 1;
                    }
                    // CQRS relational sink (SPEC-021)
                    self.relational_sink
                        .upsert_entity(
                            key,
                            &entity_types[i],
                            &descriptions[i],
                            self.tenant_id.as_deref(),
                            self.workspace_id.as_deref(),
                            &source_chunk_ids[i],
                        )
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!(
                                entity = %key,
                                error = %e,
                                "Relational entity sink failed (best-effort; graph write succeeded)"
                            );
                        });
                    // Lineage sink (SPEC-032 W-08): record chunk→entity links
                    let ws = self.workspace_id.as_deref().unwrap_or("default");
                    for chunk_id in &source_chunk_ids[i] {
                        self.lineage_sink
                            .record_entity_link(chunk_id, key, ws)
                            .await
                            .unwrap_or_else(|e| {
                                tracing::warn!(
                                    entity = %key,
                                    chunk = %chunk_id,
                                    error = %e,
                                    "Lineage sink record_entity_link failed (best-effort)"
                                );
                            });
                    }
                }
                Err(e) => {
                    stats.errors += 1;
                    tracing::warn!(
                        error.source = "pipeline_merger",
                        error.action = "merge_entity",
                        error.message = %e,
                        "Failed to merge entity"
                    );
                }
            }
        }

        if !node_batch.is_empty() {
            self.graph_storage.upsert_nodes_batch(&node_batch).await?;
        }

        Ok(())
    }

    async fn build_entity_node_batch_entry(
        &self,
        entity: &ExtractedEntity,
        existing: Option<&GraphNode>,
        artifacts: &mut MergeArtifacts,
    ) -> Result<(String, HashMap<String, serde_json::Value>, bool)> {
        let entity_id = EntityId::new(&entity.name);
        let entity_key = entity_id.as_graph_node_id().to_string();

        match existing.cloned() {
            Some(mut node) => {
                self.update_entity_node(&mut node, entity).await?;
                Ok((node.id.clone(), node.properties, false))
            }
            None => {
                let node = self.create_entity_node(entity)?;
                if entity.embedding.is_some() {
                    artifacts.entity_vector_ids.push(entity_id.as_vector_id());
                }
                artifacts.graph_nodes_created.push(entity_key);
                Ok((node.id, node.properties, true))
            }
        }
    }

    /// Update an existing entity node with new information.
    ///
    /// # SPEC-032 W-06: Similarity gate for LLM summarizer
    ///
    /// WHY: When the same entity appears across multiple chunks or documents,
    /// its description is often near-identical (same sentence, slightly reworded).
    /// Calling the LLM to "merge" two 95%-similar descriptions costs ~500ms and
    /// API credits while adding minimal value.
    ///
    /// Gate: if Jaccard(existing_desc, new_desc) ≥ threshold → keep the longer
    /// description (richer), skip the LLM call. Below threshold → use LLM.
    async fn update_entity_node(
        &self,
        node: &mut GraphNode,
        entity: &ExtractedEntity,
    ) -> Result<()> {
        // Merge descriptions
        let existing_desc = node
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // ── SPEC-032 W-06: Similarity gate ───────────────────────────────
        let similarity = super::entity::description_similarity(existing_desc, &entity.description);
        let use_llm = self.config.use_llm_summarization
            && self.summarizer.is_some()
            && similarity < self.config.description_similarity_threshold;

        let merged_desc = if use_llm {
            if let Some(summarizer) = &self.summarizer {
                let descriptions = vec![existing_desc.to_string(), entity.description.clone()];
                match summarizer
                    .merge_entity_descriptions(&entity.name, &descriptions)
                    .await
                {
                    Ok(merged) => {
                        tracing::debug!(
                            entity = %entity.name,
                            similarity,
                            "LLM description merge completed"
                        );
                        merged
                    }
                    Err(e) => {
                        tracing::warn!(
                            entity = %entity.name,
                            error = %e,
                            "LLM summarization failed, falling back to simple merge"
                        );
                        merge_descriptions(
                            existing_desc,
                            &entity.description,
                            self.config.max_description_length,
                        )
                    }
                }
            } else {
                merge_descriptions(
                    existing_desc,
                    &entity.description,
                    self.config.max_description_length,
                )
            }
        } else {
            // Similarity gate: descriptions overlap enough → keep the longer one
            if similarity >= self.config.description_similarity_threshold {
                tracing::debug!(
                    entity = %entity.name,
                    similarity,
                    threshold = self.config.description_similarity_threshold,
                    "Similarity gate: skipping LLM summarizer (descriptions near-identical)"
                );
            }
            if entity.description.len() > existing_desc.len() {
                entity.description.clone()
            } else {
                existing_desc.to_string()
            }
        };

        node.properties.insert(
            "description".to_string(),
            serde_json::Value::String(merged_desc),
        );

        // Update importance (take max)
        let existing_importance = node
            .properties
            .get("importance")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let new_importance = existing_importance.max(entity.importance);
        node.properties.insert(
            "importance".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(new_importance as f64).unwrap()),
        );

        // Merge source spans
        let mut sources: Vec<String> = node
            .properties
            .get("sources")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        for span in &entity.source_spans {
            if !sources.contains(span) && sources.len() < self.config.max_sources {
                sources.push(span.clone());
            }
        }

        node.properties
            .insert("sources".to_string(), serde_json::json!(sources));

        // Merge source chunk IDs (for citation tracking)
        let mut source_chunk_ids: Vec<String> = node
            .properties
            .get("source_chunk_ids")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        for chunk_id in &entity.source_chunk_ids {
            if !source_chunk_ids.contains(chunk_id) {
                source_chunk_ids.push(chunk_id.clone());
            }
        }

        node.properties.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(source_chunk_ids),
        );

        // Update source document ID and file path if not already set
        if !node.properties.contains_key("source_document_id") {
            if let Some(ref doc_id) = entity.source_document_id {
                node.properties.insert(
                    "source_document_id".to_string(),
                    serde_json::Value::String(doc_id.clone()),
                );
            }
        }
        if !node.properties.contains_key("source_file_path") {
            if let Some(ref file_path) = entity.source_file_path {
                node.properties.insert(
                    "source_file_path".to_string(),
                    serde_json::Value::String(file_path.clone()),
                );
            }
        }

        Ok(())
    }

    /// Create a new entity node.
    fn create_entity_node(&self, entity: &ExtractedEntity) -> Result<GraphNode> {
        let entity_id = EntityId::new(&entity.name);
        let entity_key = entity_id.as_graph_node_id().to_string();

        let mut properties = HashMap::new();
        properties.insert(
            "entity_type".to_string(),
            serde_json::Value::String(entity.entity_type.clone()),
        );
        properties.insert(
            "description".to_string(),
            serde_json::Value::String(entity.description.clone()),
        );
        properties.insert(
            "importance".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(entity.importance as f64).unwrap(),
            ),
        );
        properties.insert(
            "sources".to_string(),
            serde_json::json!(entity.source_spans),
        );
        properties.insert(
            "label".to_string(),
            serde_json::Value::String(entity.name.clone()),
        );

        // Source tracking for citations (LightRAG parity)
        properties.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(entity.source_chunk_ids),
        );
        if let Some(ref doc_id) = entity.source_document_id {
            properties.insert(
                "source_document_id".to_string(),
                serde_json::Value::String(doc_id.clone()),
            );
        }
        if let Some(ref file_path) = entity.source_file_path {
            properties.insert(
                "source_file_path".to_string(),
                serde_json::Value::String(file_path.clone()),
            );
        }

        // Add tenant context
        if let Some(tenant_id) = &self.tenant_id {
            properties.insert(
                "tenant_id".to_string(),
                serde_json::Value::String(tenant_id.clone()),
            );
        }
        if let Some(workspace_id) = &self.workspace_id {
            properties.insert(
                "workspace_id".to_string(),
                serde_json::Value::String(workspace_id.clone()),
            );
        }

        Ok(GraphNode {
            id: entity_key,
            properties,
        })
    }
}
