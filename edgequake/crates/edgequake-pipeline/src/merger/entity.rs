//! Entity merge, update, and creation logic for the knowledge graph.

use std::collections::HashMap;

use edgequake_storage::{EntityId, GraphNode, GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::{ExtractedEntity, ExtractionResult};

use super::{merge_descriptions, metadata, MergeArtifacts, MergeStats};

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
    pub(super) async fn merge_entities_batch(
        &self,
        entities: Vec<ExtractedEntity>,
        stats: &mut MergeStats,
    ) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }

        let mut keys = Vec::new();
        let mut valid = Vec::new();
        for entity in entities {
            let entity_id = EntityId::new(&entity.name);
            if entity_id.is_empty() {
                tracing::warn!(
                    raw_name = %entity.name,
                    "Skipping entity with empty normalized name"
                );
                continue;
            }
            keys.push(entity_id.as_graph_node_id().to_string());
            valid.push(entity);
        }

        if valid.is_empty() {
            return Ok(());
        }

        let existing_map = self.graph_storage.get_nodes_batch(&keys).await?;
        let mut node_batch: Vec<(String, HashMap<String, serde_json::Value>)> =
            Vec::with_capacity(valid.len());

        for (entity, key) in valid.into_iter().zip(keys.iter()) {
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
                    self.relational_sink
                        .upsert_entity(
                            key,
                            &entity.entity_type,
                            &entity.description,
                            self.tenant_id.as_deref(),
                            self.workspace_id.as_deref(),
                            &entity.source_chunk_ids,
                        )
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!(
                                entity = %key,
                                error = %e,
                                "Relational entity sink failed (best-effort; graph write succeeded)"
                            );
                        });
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

        // Use LLM summarizer if available and enabled
        let merged_desc = if self.config.use_llm_summarization {
            if let Some(summarizer) = &self.summarizer {
                // Use LLM to intelligently merge descriptions
                let descriptions = vec![existing_desc.to_string(), entity.description.clone()];
                match summarizer
                    .merge_entity_descriptions(&entity.name, &descriptions)
                    .await
                {
                    Ok(merged) => merged,
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
                // No summarizer provided, use simple merge
                merge_descriptions(
                    existing_desc,
                    &entity.description,
                    self.config.max_description_length,
                )
            }
        } else {
            // LLM summarization disabled
            merge_descriptions(
                existing_desc,
                &entity.description,
                self.config.max_description_length,
            )
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
