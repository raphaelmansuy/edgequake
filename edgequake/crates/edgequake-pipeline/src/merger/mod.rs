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
mod relationship;

use std::sync::Arc;

use edgequake_storage::{GraphStorage, VectorStorage};

use crate::error::Result;
use crate::extractor::ExtractionResult;
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
    pub(super) config: MergerConfig,
    pub(super) graph_storage: Arc<G>,
    pub(super) vector_storage: Arc<V>,
    pub(super) tenant_id: Option<String>,
    pub(super) workspace_id: Option<String>,
    /// Optional LLM summarizer for intelligent description merging.
    pub(super) summarizer: Option<Arc<LLMSummarizer>>,
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
    use crate::extractor::{ExtractedEntity, ExtractedRelationship};

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
