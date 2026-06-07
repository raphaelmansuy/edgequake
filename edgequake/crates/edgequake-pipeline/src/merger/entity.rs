//! Entity merge, update, and creation logic for the knowledge graph.

use std::collections::HashMap;

use edgequake_storage::{GraphNode, GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::ExtractedEntity;

use super::{merge_descriptions, metadata, normalize_entity_name};

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> super::KnowledgeGraphMerger<G, V> {
    /// Merge a single entity, returning true if it was newly created.
    pub(super) async fn merge_entity(&self, entity: ExtractedEntity) -> Result<bool> {
        let entity_key = normalize_entity_name(&entity.name);

        // Store entity embedding with type metadata (for Local query mode)
        if let Some(embedding) = &entity.embedding {
            let scope = metadata::TenantScope {
                tenant_id: &self.tenant_id,
                workspace_id: &self.workspace_id,
            };
            let metadata = metadata::entity_vector_metadata(&entity, scope);

            self.vector_storage
                .upsert(&[(entity_key.clone(), embedding.clone(), metadata)])
                .await?;
        }

        // Check if entity exists
        let existing = self.graph_storage.get_node(&entity_key).await?;

        match existing {
            Some(mut node) => {
                // Update existing entity
                self.update_entity_node(&mut node, &entity).await?;
                self.graph_storage
                    .upsert_node(&node.id, node.properties)
                    .await?;
                Ok(false)
            }
            None => {
                // Create new entity
                let node = self.create_entity_node(&entity)?;
                self.graph_storage
                    .upsert_node(&node.id, node.properties)
                    .await?;
                Ok(true)
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
        let entity_key = normalize_entity_name(&entity.name);

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
