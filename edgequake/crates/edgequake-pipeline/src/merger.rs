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
//! This module provides functionality for merging extracted entities
//! and relationships into the knowledge graph, handling deduplication
//! and description aggregation.

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_storage::{GraphEdge, GraphNode, GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::{ExtractedEntity, ExtractedRelationship, ExtractionResult};
use crate::summarizer::LLMSummarizer;

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
}

impl Default for MergerConfig {
    fn default() -> Self {
        Self {
            max_description_length: 4096,
            description_decay: 0.9,
            min_importance: 0.1,
            max_sources: 10,
            use_llm_summarization: true, // Enable by default for SOTA quality
        }
    }
}

/// Merges extracted entities and relationships into the knowledge graph.
/// @implements FEAT0005
pub struct KnowledgeGraphMerger<G: GraphStorage + ?Sized, V: VectorStorage + ?Sized> {
    config: MergerConfig,
    graph_storage: Arc<G>,
    vector_storage: Arc<V>,
    tenant_id: Option<String>,
    workspace_id: Option<String>,
    /// Optional LLM summarizer for intelligent description merging.
    summarizer: Option<Arc<LLMSummarizer>>,
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
        }
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
            let mut metadata = serde_json::json!({
                "type": "entity",  // Mark as entity for retrieval filtering
                "entity_name": entity.name,
                "entity_type": entity.entity_type,
                "description": entity.description,
                // Source tracking for citations (LightRAG parity)
                "source_chunk_ids": entity.source_chunk_ids,
                "source_document_id": entity.source_document_id,
                "source_file_path": entity.source_file_path
            });

            if let Some(tenant_id) = &self.tenant_id {
                metadata["tenant_id"] = serde_json::json!(tenant_id);
            }
            if let Some(workspace_id) = &self.workspace_id {
                metadata["workspace_id"] = serde_json::json!(workspace_id);
            }

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

    /// Merge a single relationship, returning true if it was newly created.
    async fn merge_relationship(&self, rel: ExtractedRelationship) -> Result<bool> {
        let source_key = normalize_entity_name(&rel.source);
        let target_key = normalize_entity_name(&rel.target);

        // Store relationship embedding with type metadata (for Global query mode)
        if let Some(embedding) = &rel.embedding {
            let rel_id = format!("{}->{}:{}", source_key, target_key, rel.relation_type);
            let mut metadata = serde_json::json!({
                "type": "relationship",  // Mark as relationship for retrieval filtering
                "src_id": source_key,
                "tgt_id": target_key,
                "keywords": rel.keywords.join(", "),
                "relation_type": rel.relation_type,
                "description": rel.description,
                // Source tracking for citations (LightRAG parity)
                "source_chunk_id": rel.source_chunk_id,
                "source_document_id": rel.source_document_id,
                "source_file_path": rel.source_file_path
            });

            if let Some(tenant_id) = &self.tenant_id {
                metadata["tenant_id"] = serde_json::json!(tenant_id);
            }
            if let Some(workspace_id) = &self.workspace_id {
                metadata["workspace_id"] = serde_json::json!(workspace_id);
            }

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
}
