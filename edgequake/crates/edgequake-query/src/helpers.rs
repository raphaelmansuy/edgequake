//! Helper functions for query processing.
//!
//! This module provides common utilities for extracting metadata,
//! building context objects, and processing search results.
//!
//! # WHY This Module Exists
//!
//! Before this extraction, the same patterns were repeated 5-7 times
//! across `sota_engine.rs`. This caused:
//! - Inconsistent handling of edge cases
//! - Hard-to-maintain code (changes required 7 edits)
//! - Risk of divergent implementations
//!
//! By centralizing these patterns, we ensure consistent behavior and
//! reduce the surface area for bugs.

use std::collections::HashMap;

use serde_json::Value;

use crate::context::{RetrievedChunk, RetrievedEntity, RetrievedRelationship};
use edgequake_storage::traits::VectorSearchResult;

/// Source tracking information extracted from entity nodes.
///
/// WHY: Entities in the knowledge graph track their provenance
/// (which chunks/documents they came from) for citation purposes.
#[derive(Debug, Default, Clone)]
pub struct EntitySourceTracking {
    /// Chunk IDs that contributed to this entity.
    pub source_chunk_ids: Vec<String>,
    /// Primary source document ID.
    pub source_document_id: Option<String>,
    /// File path of the source document.
    pub source_file_path: Option<String>,
}

/// Source tracking information extracted from relationship edges.
///
/// WHY: Relationships also track provenance, but typically have
/// a single source chunk (relationships are extracted from one place).
#[derive(Debug, Default, Clone)]
pub struct RelationshipSourceTracking {
    /// Single chunk ID that contributed this relationship.
    pub source_chunk_id: Option<String>,
    /// Primary source document ID.
    pub source_document_id: Option<String>,
    /// File path of the source document.
    pub source_file_path: Option<String>,
}

/// Extract document UUID from chunk ID.
///
/// # WHY: Chunk ID Format
///
/// Chunk IDs follow the format: `{document-uuid}-chunk-{N}`
/// Example: `f0291a69-8b63-46d5-b44b-24095b3a8283-chunk-0`
///
/// This function extracts the UUID portion for document linking,
/// enabling the UI to link citations back to source documents.
pub fn extract_document_id(chunk_id: &str) -> Option<String> {
    if let Some(suffix_idx) = chunk_id.rfind("-chunk-") {
        if suffix_idx > 0 {
            return Some(chunk_id[..suffix_idx].to_string());
        }
    }
    None
}

/// Extract source tracking from entity node properties.
///
/// # WHY: Centralized Extraction
///
/// Entity nodes store source tracking in a specific schema:
/// - `source_chunk_ids`: JSON array of chunk IDs
/// - `source_document_id`: String document UUID
/// - `source_file_path`: String file path
///
/// This function handles JSON parsing and null checks consistently.
pub fn extract_entity_source_tracking(props: &HashMap<String, Value>) -> EntitySourceTracking {
    let source_chunk_ids: Vec<String> = props
        .get("source_chunk_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let source_document_id = props
        .get("source_document_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source_file_path = props
        .get("source_file_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    EntitySourceTracking {
        source_chunk_ids,
        source_document_id,
        source_file_path,
    }
}

/// Extract source tracking from relationship edge properties.
///
/// # WHY: Relationships Have Single Source
///
/// Unlike entities (which may appear in multiple chunks), relationships
/// typically originate from a single chunk where the connection was stated.
pub fn extract_relationship_source_tracking(
    props: &HashMap<String, Value>,
) -> RelationshipSourceTracking {
    let source_chunk_id = props
        .get("source_chunk_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source_document_id = props
        .get("source_document_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source_file_path = props
        .get("source_file_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    RelationshipSourceTracking {
        source_chunk_id,
        source_document_id,
        source_file_path,
    }
}

/// Build a RetrievedChunk from a vector search result.
///
/// # WHY: Consistent Chunk Construction
///
/// When building chunks from vector results, we need to:
/// 1. Extract content from metadata
/// 2. Set the score
/// 3. Extract document ID from chunk ID
/// 4. Extract line numbers if available
/// 5. Extract chunk index if available
///
/// This function ensures all chunks are built consistently.
pub fn build_chunk_from_result(result: &VectorSearchResult) -> RetrievedChunk {
    let content = result
        .metadata
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut chunk = RetrievedChunk::new(&result.id, content, result.score);

    // Extract document_id from chunk_id (format: "uuid-chunk-N")
    if let Some(doc_id) = extract_document_id(&result.id) {
        chunk = chunk.with_document_id(doc_id);
    }

    // Extract line number information if available
    if let Some(start) = result.metadata.get("start_line").and_then(|v| v.as_u64()) {
        if let Some(end) = result.metadata.get("end_line").and_then(|v| v.as_u64()) {
            chunk = chunk.with_lines(start as usize, end as usize);
        }
    }

    if let Some(idx) = result.metadata.get("chunk_index").and_then(|v| v.as_u64()) {
        chunk = chunk.with_chunk_index(idx as usize);
    }

    chunk
}

/// Build a RetrievedEntity from a graph node.
///
/// # Arguments
/// * `node_id` - The entity identifier
/// * `props` - Node properties from the graph
/// * `degree` - Number of connections (for ranking)
/// * `score` - Similarity score from vector search
pub fn build_entity_from_node(
    node_id: &str,
    props: &HashMap<String, Value>,
    degree: usize,
    score: f32,
) -> RetrievedEntity {
    let entity_type = props
        .get("entity_type")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN")
        .to_string();

    let description = props
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let source_tracking = extract_entity_source_tracking(props);

    let mut entity = RetrievedEntity::new(node_id, entity_type, description)
        .with_degree(degree)
        .with_score(score);

    if !source_tracking.source_chunk_ids.is_empty() {
        entity = entity.with_source_chunk_ids(source_tracking.source_chunk_ids);
    }
    if let Some(doc_id) = source_tracking.source_document_id {
        entity = entity.with_source_document_id(doc_id);
    }
    if let Some(file_path) = source_tracking.source_file_path {
        entity = entity.with_source_file_path(file_path);
    }

    entity
}

/// Build a RetrievedRelationship from a graph edge.
///
/// # Arguments
/// * `source` - Source entity ID
/// * `target` - Target entity ID
/// * `props` - Edge properties from the graph
pub fn build_relationship_from_edge(
    source: &str,
    target: &str,
    props: &HashMap<String, Value>,
) -> RetrievedRelationship {
    let rel_type = props
        .get("relation_type")
        .and_then(|v| v.as_str())
        .unwrap_or("RELATED_TO")
        .to_string();

    let description = props
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let source_tracking = extract_relationship_source_tracking(props);

    let mut rel = RetrievedRelationship::new(source, target, rel_type);

    if let Some(desc) = description {
        rel = rel.with_description(desc);
    }
    if let Some(chunk_id) = source_tracking.source_chunk_id {
        rel = rel.with_source_chunk_id(chunk_id);
    }
    if let Some(doc_id) = source_tracking.source_document_id {
        rel = rel.with_source_document_id(doc_id);
    }
    if let Some(file_path) = source_tracking.source_file_path {
        rel = rel.with_source_file_path(file_path);
    }

    rel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_document_id() {
        // Standard format
        assert_eq!(
            extract_document_id("f0291a69-8b63-46d5-b44b-24095b3a8283-chunk-0"),
            Some("f0291a69-8b63-46d5-b44b-24095b3a8283".to_string())
        );

        // Multi-digit chunk index
        assert_eq!(
            extract_document_id("abc123-chunk-42"),
            Some("abc123".to_string())
        );

        // No chunk suffix
        assert_eq!(extract_document_id("just-a-uuid"), None);

        // Empty string
        assert_eq!(extract_document_id(""), None);

        // Malformed (chunk at start)
        assert_eq!(extract_document_id("-chunk-0"), None);
    }

    #[test]
    fn test_extract_entity_source_tracking() {
        let mut props = HashMap::new();
        props.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(["chunk-1", "chunk-2"]),
        );
        props.insert(
            "source_document_id".to_string(),
            serde_json::json!("doc-123"),
        );
        props.insert(
            "source_file_path".to_string(),
            serde_json::json!("/path/to/file.pdf"),
        );

        let tracking = extract_entity_source_tracking(&props);

        assert_eq!(tracking.source_chunk_ids, vec!["chunk-1", "chunk-2"]);
        assert_eq!(tracking.source_document_id, Some("doc-123".to_string()));
        assert_eq!(
            tracking.source_file_path,
            Some("/path/to/file.pdf".to_string())
        );
    }

    #[test]
    fn test_extract_entity_source_tracking_empty() {
        let props = HashMap::new();
        let tracking = extract_entity_source_tracking(&props);

        assert!(tracking.source_chunk_ids.is_empty());
        assert!(tracking.source_document_id.is_none());
        assert!(tracking.source_file_path.is_none());
    }

    #[test]
    fn test_extract_relationship_source_tracking() {
        let mut props = HashMap::new();
        props.insert("source_chunk_id".to_string(), serde_json::json!("chunk-1"));
        props.insert(
            "source_document_id".to_string(),
            serde_json::json!("doc-123"),
        );

        let tracking = extract_relationship_source_tracking(&props);

        assert_eq!(tracking.source_chunk_id, Some("chunk-1".to_string()));
        assert_eq!(tracking.source_document_id, Some("doc-123".to_string()));
    }

    #[test]
    fn test_build_entity_from_node() {
        let mut props = HashMap::new();
        props.insert("entity_type".to_string(), serde_json::json!("PERSON"));
        props.insert(
            "description".to_string(),
            serde_json::json!("A famous person"),
        );
        props.insert(
            "source_chunk_ids".to_string(),
            serde_json::json!(["chunk-1"]),
        );

        let entity = build_entity_from_node("JOHN_DOE", &props, 5, 0.85);

        assert_eq!(entity.name, "JOHN_DOE");
        assert_eq!(entity.entity_type, "PERSON");
        assert_eq!(entity.description, "A famous person");
        assert_eq!(entity.degree, 5);
        assert_eq!(entity.score, 0.85);
        assert_eq!(entity.source_chunk_ids, vec!["chunk-1"]);
    }

    #[test]
    fn test_build_relationship_from_edge() {
        let mut props = HashMap::new();
        props.insert("relation_type".to_string(), serde_json::json!("WORKS_FOR"));
        props.insert(
            "description".to_string(),
            serde_json::json!("Employment relationship"),
        );
        props.insert("source_chunk_id".to_string(), serde_json::json!("chunk-1"));

        let rel = build_relationship_from_edge("JOHN_DOE", "ACME_CORP", &props);

        assert_eq!(rel.source, "JOHN_DOE");
        assert_eq!(rel.target, "ACME_CORP");
        assert_eq!(rel.relation_type, "WORKS_FOR");
        assert_eq!(rel.source_chunk_id, Some("chunk-1".to_string()));
    }
}
