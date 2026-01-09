//! Vector search result filtering utilities.
//!
//! This module provides utilities for filtering vector search results by type,
//! supporting the distinction between chunks, entities, and relationships.
//!
//! @implements FEAT0110 (Vector Filtering)

use edgequake_storage::VectorSearchResult;

/// Vector type identifier in metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorType {
    /// Chunk vector (text content).
    Chunk,
    /// Entity vector (entity name + description).
    Entity,
    /// Relationship vector (keywords + src + tgt + description).
    Relationship,
}

impl VectorType {
    /// Get the string representation used in metadata.
    pub fn as_str(&self) -> &'static str {
        match self {
            VectorType::Chunk => "chunk",
            VectorType::Entity => "entity",
            VectorType::Relationship => "relationship",
        }
    }
}

/// Filter vector search results by type.
///
/// # Arguments
///
/// * `results` - Vector search results from storage
/// * `vector_type` - Type to filter for
///
/// # Returns
///
/// Filtered results containing only the specified type.
pub fn filter_by_type(
    results: Vec<VectorSearchResult>,
    vector_type: VectorType,
) -> Vec<VectorSearchResult> {
    let type_str = vector_type.as_str();

    results
        .into_iter()
        .filter(|result| {
            result
                .metadata
                .get("type")
                .and_then(|v| v.as_str())
                .map(|t| t == type_str)
                .unwrap_or(false)
        })
        .collect()
}

/// Get vector results of a specific type with limit.
///
/// Filters results by type and takes up to `limit` items.
///
/// # Arguments
///
/// * `results` - Vector search results from storage
/// * `vector_type` - Type to filter for
/// * `limit` - Maximum number of results to return
///
/// # Returns
///
/// Filtered and limited results.
pub fn get_typed_vectors(
    results: Vec<VectorSearchResult>,
    vector_type: VectorType,
    limit: usize,
) -> Vec<VectorSearchResult> {
    filter_by_type(results, vector_type)
        .into_iter()
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_result(id: &str, score: f32, vector_type: &str) -> VectorSearchResult {
        VectorSearchResult {
            id: id.to_string(),
            score,
            metadata: json!({
                "type": vector_type,
                "content": "test content"
            }),
        }
    }

    #[test]
    fn test_filter_by_type_chunks() {
        let results = vec![
            create_test_result("chunk-1", 0.9, "chunk"),
            create_test_result("entity-1", 0.8, "entity"),
            create_test_result("chunk-2", 0.7, "chunk"),
            create_test_result("rel-1", 0.6, "relationship"),
        ];

        let chunks = filter_by_type(results, VectorType::Chunk);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].id.starts_with("chunk"));
        assert!(chunks[1].id.starts_with("chunk"));
    }

    #[test]
    fn test_filter_by_type_entities() {
        let results = vec![
            create_test_result("chunk-1", 0.9, "chunk"),
            create_test_result("entity-1", 0.8, "entity"),
            create_test_result("entity-2", 0.7, "entity"),
            create_test_result("rel-1", 0.6, "relationship"),
        ];

        let entities = filter_by_type(results, VectorType::Entity);
        assert_eq!(entities.len(), 2);
        assert!(entities[0].id.starts_with("entity"));
    }

    #[test]
    fn test_filter_by_type_relationships() {
        let results = vec![
            create_test_result("chunk-1", 0.9, "chunk"),
            create_test_result("entity-1", 0.8, "entity"),
            create_test_result("rel-1", 0.7, "relationship"),
            create_test_result("rel-2", 0.6, "relationship"),
        ];

        let rels = filter_by_type(results, VectorType::Relationship);
        assert_eq!(rels.len(), 2);
        assert!(rels[0].id.starts_with("rel"));
    }

    #[test]
    fn test_get_typed_vectors_with_limit() {
        let results = vec![
            create_test_result("entity-1", 0.9, "entity"),
            create_test_result("entity-2", 0.8, "entity"),
            create_test_result("entity-3", 0.7, "entity"),
            create_test_result("chunk-1", 0.6, "chunk"),
        ];

        let entities = get_typed_vectors(results, VectorType::Entity, 2);
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].id, "entity-1");
        assert_eq!(entities[1].id, "entity-2");
    }

    #[test]
    fn test_filter_empty_results() {
        let results = vec![];
        let chunks = filter_by_type(results, VectorType::Chunk);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_filter_missing_type_field() {
        let result = VectorSearchResult {
            id: "test-1".to_string(),
            score: 0.9,
            metadata: json!({"content": "no type field"}),
        };

        let results = vec![result];
        let chunks = filter_by_type(results, VectorType::Chunk);
        assert_eq!(chunks.len(), 0); // Should filter out results without type
    }
}
