//! Entity and relationship merging.
//!
//! This module provides functionality for merging extracted entities
//! and relationships into the knowledge graph, handling deduplication
//! and description aggregation.

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_storage::{GraphEdge, GraphNode, GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};

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
}

impl Default for MergerConfig {
    fn default() -> Self {
        Self {
            max_description_length: 4096,
            description_decay: 0.9,
            min_importance: 0.1,
            max_sources: 10,
        }
    }
}

/// Merges extracted entities and relationships into the knowledge graph.
pub struct KnowledgeGraphMerger<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> {
    config: MergerConfig,
    graph_storage: Arc<G>,
    vector_storage: Arc<V>,
}

impl<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> KnowledgeGraphMerger<G, V> {
    /// Create a new merger.
    pub fn new(config: MergerConfig, graph_storage: Arc<G>, vector_storage: Arc<V>) -> Self {
        Self {
            config,
            graph_storage,
            vector_storage,
        }
    }

    /// Merge extraction results into the knowledge graph.
    pub async fn merge(&self, results: Vec<ExtractionResult>) -> Result<MergeStats> {
        let mut stats = MergeStats::default();

        for result in results {
            // Merge entities first
            for entity in result.entities {
                match self.merge_entity(entity).await {
                    Ok(was_new) => {
                        if was_new {
                            stats.entities_created += 1;
                        } else {
                            stats.entities_updated += 1;
                        }
                    }
                    Err(e) => {
                        stats.errors += 1;
                        tracing::warn!("Failed to merge entity: {}", e);
                    }
                }
            }

            // Then merge relationships
            for rel in result.relationships {
                match self.merge_relationship(rel).await {
                    Ok(was_new) => {
                        if was_new {
                            stats.relationships_created += 1;
                        } else {
                            stats.relationships_updated += 1;
                        }
                    }
                    Err(e) => {
                        stats.errors += 1;
                        tracing::warn!("Failed to merge relationship: {}", e);
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Merge a single entity, returning true if it was newly created.
    async fn merge_entity(&self, entity: ExtractedEntity) -> Result<bool> {
        let entity_key = normalize_entity_name(&entity.name);

        // Store entity embedding with type metadata (for Local query mode)
        if let Some(embedding) = &entity.embedding {
            self.vector_storage
                .upsert(&[(
                    entity_key.clone(),
                    embedding.clone(),
                    serde_json::json!({
                        "type": "entity",  // Mark as entity for retrieval filtering
                        "entity_name": entity.name,
                        "entity_type": entity.entity_type,
                        "description": entity.description
                    }),
                )])
                .await?;
        }

        // Check if entity exists
        let existing = self.graph_storage.get_node(&entity_key).await?;

        match existing {
            Some(mut node) => {
                // Update existing entity
                self.update_entity_node(&mut node, &entity)?;
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

    /// Merge a single relationship, returning true if it was newly created.
    async fn merge_relationship(&self, rel: ExtractedRelationship) -> Result<bool> {
        let source_key = normalize_entity_name(&rel.source);
        let target_key = normalize_entity_name(&rel.target);

        // Store relationship embedding with type metadata (for Global query mode)
        if let Some(embedding) = &rel.embedding {
            let rel_id = format!("{}->{}:{}", source_key, target_key, rel.relation_type);
            self.vector_storage
                .upsert(&[(
                    rel_id,
                    embedding.clone(),
                    serde_json::json!({
                        "type": "relationship",  // Mark as relationship for retrieval filtering
                        "src_id": source_key,
                        "tgt_id": target_key,
                        "keywords": rel.keywords.join(", "),
                        "relation_type": rel.relation_type,
                        "description": rel.description
                    }),
                )])
                .await?;
        }

        // Check if edge exists
        let existing = self.graph_storage.get_edge(&source_key, &target_key).await?;

        match existing {
            Some(mut edge) => {
                // Update existing relationship
                self.update_relationship_edge(&mut edge, &rel)?;
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

    /// Update an existing entity node with new information.
    fn update_entity_node(
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

        let merged_desc = merge_descriptions(
            existing_desc,
            &entity.description,
            self.config.max_description_length,
        );

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

        node.properties.insert(
            "sources".to_string(),
            serde_json::json!(sources),
        );

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

        Ok(GraphNode {
            id: entity_key,
            properties,
        })
    }

    /// Update an existing relationship edge.
    fn update_relationship_edge(
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

        let merged_desc = merge_descriptions(
            existing_desc,
            &rel.description,
            self.config.max_description_length,
        );

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

        edge.properties.insert(
            "keywords".to_string(),
            serde_json::json!(keywords),
        );

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
        properties.insert(
            "keywords".to_string(),
            serde_json::json!(rel.keywords),
        );
        properties.insert(
            "relation_type".to_string(),
            serde_json::Value::String(rel.relation_type.clone()),
        );

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

            self.graph_storage.upsert_node(key, properties).await?;
        }
        Ok(())
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

/// Normalize an entity name to a consistent key format.
pub fn normalize_entity_name(name: &str) -> String {
    name.trim()
        .to_uppercase()
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
}

/// Merge two descriptions, avoiding duplication.
fn merge_descriptions(existing: &str, new: &str, max_length: usize) -> String {
    if existing.is_empty() {
        return truncate_description(new, max_length);
    }

    if new.is_empty() || existing.contains(new) {
        return existing.to_string();
    }

    // Check if new content adds meaningful information
    let new_sentences: Vec<&str> = new.split(|c| c == '.' || c == '!' || c == '?').collect();
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

    #[test]
    fn test_normalize_entity_name() {
        assert_eq!(normalize_entity_name("John Doe"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("  Hello  World  "), "HELLO_WORLD");
        assert_eq!(normalize_entity_name("O'Brien"), "OBRIEN");
        assert_eq!(normalize_entity_name("AI/ML"), "AIML");
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
        };

        assert_eq!(stats.total_entities(), 8);
        assert_eq!(stats.total_relationships(), 12);
    }
}
