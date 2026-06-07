//! Relationship merge, update, creation, and placeholder node logic.

use std::collections::HashMap;

use edgequake_storage::{GraphEdge, GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::ExtractedRelationship;

use super::{merge_descriptions, metadata, normalize_entity_name};

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> super::KnowledgeGraphMerger<G, V> {
    /// Merge a single relationship, returning true if it was newly created.
    pub(super) async fn merge_relationship(&self, rel: ExtractedRelationship) -> Result<bool> {
        let source_key = normalize_entity_name(&rel.source);
        let target_key = normalize_entity_name(&rel.target);

        // BR0006: Same-entity relationships forbidden (secondary defense)
        // WHY: The parser should filter these, but defense-in-depth prevents
        // self-loops from reaching the graph storage layer.
        if source_key == target_key {
            tracing::debug!(
                source = %source_key,
                "Merger: skipping self-referencing relationship (BR0006)"
            );
            return Ok(false);
        }

        // Skip relationships with empty normalized endpoints
        if source_key.is_empty() || target_key.is_empty() {
            tracing::debug!(
                raw_source = %rel.source,
                raw_target = %rel.target,
                "Merger: skipping relationship with empty normalized endpoint"
            );
            return Ok(false);
        }

        // Store relationship embedding with type metadata (for Global query mode)
        if let Some(embedding) = &rel.embedding {
            let rel_id = format!("{}->{}:{}", source_key, target_key, rel.relation_type);
            let scope = metadata::TenantScope {
                tenant_id: &self.tenant_id,
                workspace_id: &self.workspace_id,
            };
            let metadata =
                metadata::relationship_vector_metadata(&rel, &source_key, &target_key, scope);

            self.vector_storage
                .upsert(&[(rel_id, embedding.clone(), metadata)])
                .await?;
        }

        // Check if edge exists
        let existing = self
            .graph_storage
            .get_edge(&source_key, &target_key)
            .await?;

        match existing {
            Some(mut edge) => {
                // Update existing relationship
                self.update_relationship_edge(&mut edge, &rel).await?;
                self.graph_storage
                    .upsert_edge(&edge.source, &edge.target, edge.properties)
                    .await?;
                Ok(false)
            }
            None => {
                // Ensure both nodes exist
                self.ensure_node_exists(&source_key, &rel.source).await?;
                self.ensure_node_exists(&target_key, &rel.target).await?;

                // Create new relationship
                let edge = self.create_relationship_edge(&source_key, &target_key, &rel)?;
                self.graph_storage
                    .upsert_edge(&edge.source, &edge.target, edge.properties)
                    .await?;
                Ok(true)
            }
        }
    }

    /// Update an existing relationship edge.
    async fn update_relationship_edge(
        &self,
        edge: &mut GraphEdge,
        rel: &ExtractedRelationship,
    ) -> Result<()> {
        // Merge descriptions
        let existing_desc = edge
            .properties
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Use LLM summarizer if available and enabled
        let merged_desc = if self.config.use_llm_summarization {
            if let Some(summarizer) = &self.summarizer {
                // Use LLM to intelligently merge relationship descriptions
                let descriptions = vec![existing_desc.to_string(), rel.description.clone()];
                match summarizer
                    .merge_relationship_descriptions(&rel.source, &rel.target, &descriptions)
                    .await
                {
                    Ok(merged) => merged,
                    Err(e) => {
                        tracing::warn!(
                            source = %rel.source,
                            target = %rel.target,
                            error = %e,
                            "LLM summarization failed, falling back to simple merge"
                        );
                        merge_descriptions(
                            existing_desc,
                            &rel.description,
                            self.config.max_description_length,
                        )
                    }
                }
            } else {
                merge_descriptions(
                    existing_desc,
                    &rel.description,
                    self.config.max_description_length,
                )
            }
        } else {
            merge_descriptions(
                existing_desc,
                &rel.description,
                self.config.max_description_length,
            )
        };

        edge.properties.insert(
            "description".to_string(),
            serde_json::Value::String(merged_desc),
        );

        // Update weight (use weighted average)
        let existing_weight = edge
            .properties
            .get("weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let new_weight = (existing_weight + rel.weight) / 2.0;
        edge.properties.insert(
            "weight".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(new_weight as f64).unwrap()),
        );

        // Merge keywords
        let mut keywords: Vec<String> = edge
            .properties
            .get("keywords")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        for keyword in &rel.keywords {
            if !keywords.contains(keyword) {
                keywords.push(keyword.clone());
            }
        }

        // BR0004: Relationship keywords max 5 per edge
        // WHY: Excessive keywords dilute semantic relevance and inflate storage.
        // Keep the first 5 (oldest = most established context).
        keywords.truncate(5);

        edge.properties
            .insert("keywords".to_string(), serde_json::json!(keywords));

        Ok(())
    }

    /// Create a new relationship edge.
    fn create_relationship_edge(
        &self,
        source_key: &str,
        target_key: &str,
        rel: &ExtractedRelationship,
    ) -> Result<GraphEdge> {
        let mut properties = HashMap::new();
        properties.insert(
            "relation_type".to_string(),
            serde_json::Value::String(rel.relation_type.clone()),
        );
        properties.insert(
            "description".to_string(),
            serde_json::Value::String(rel.description.clone()),
        );
        properties.insert(
            "weight".to_string(),
            serde_json::Value::Number(serde_json::Number::from_f64(rel.weight as f64).unwrap()),
        );
        properties.insert("keywords".to_string(), serde_json::json!(rel.keywords));
        properties.insert(
            "relation_type".to_string(),
            serde_json::Value::String(rel.relation_type.clone()),
        );

        // Source tracking for citations (LightRAG parity)
        if let Some(ref chunk_id) = rel.source_chunk_id {
            properties.insert(
                "source_chunk_id".to_string(),
                serde_json::Value::String(chunk_id.clone()),
            );
        }
        if let Some(ref doc_id) = rel.source_document_id {
            properties.insert(
                "source_document_id".to_string(),
                serde_json::Value::String(doc_id.clone()),
            );
        }
        if let Some(ref file_path) = rel.source_file_path {
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

        Ok(GraphEdge {
            source: source_key.to_string(),
            target: target_key.to_string(),
            properties,
        })
    }

    /// Ensure a node exists, creating a placeholder if needed.
    async fn ensure_node_exists(&self, key: &str, label: &str) -> Result<()> {
        if self.graph_storage.get_node(key).await?.is_none() {
            let mut properties = HashMap::new();
            properties.insert(
                "entity_type".to_string(),
                serde_json::Value::String("UNKNOWN".to_string()),
            );
            properties.insert(
                "description".to_string(),
                serde_json::Value::String(String::new()),
            );
            properties.insert(
                "label".to_string(),
                serde_json::Value::String(label.to_string()),
            );

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

            self.graph_storage.upsert_node(key, properties).await?;
        }
        Ok(())
    }
}
