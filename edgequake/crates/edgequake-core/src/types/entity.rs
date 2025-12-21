//! GraphEntity type definition.
//!
//! A GraphEntity represents a named entity extracted from text,
//! stored as a node in the knowledge graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An entity extracted from text.
///
/// Entities are the nodes in the knowledge graph. They represent
/// people, organizations, locations, concepts, and other named
/// items extracted from documents.
///
/// # Example
///
/// ```rust
/// use edgequake_core::types::GraphEntity;
///
/// let entity = GraphEntity::new(
///     "OpenAI".to_string(),
///     "ORGANIZATION".to_string(),
///     "An artificial intelligence research company.".to_string(),
///     "chunk-123".to_string(),
///     None,
/// );
/// assert_eq!(entity.entity_name, "OPENAI");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEntity {
    /// Entity name (uppercase normalized) - primary key
    pub id: String,
    /// Display name (normalized to uppercase)
    pub entity_name: String,
    /// Entity type (person, organization, location, etc.)
    pub entity_type: String,
    /// Aggregated description from all mentions
    pub description: String,
    /// Pipe-separated chunk IDs (sources)
    pub source_id: String,
    /// Source file paths (pipe-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl GraphEntity {
    /// Maximum number of source IDs to retain per entity.
    pub const MAX_SOURCE_IDS: usize = 300;

    /// Normalize entity name for consistent storage.
    ///
    /// Names are converted to uppercase and trimmed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgequake_core::types::GraphEntity;
    ///
    /// assert_eq!(GraphEntity::normalize_name("  openai  "), "OPENAI");
    /// ```
    pub fn normalize_name(name: &str) -> String {
        name.trim().to_uppercase()
    }

    /// Generate entity ID from name.
    ///
    /// The ID is the normalized (uppercase, trimmed) entity name.
    pub fn generate_id(name: &str) -> String {
        Self::normalize_name(name)
    }

    /// Create a new entity.
    ///
    /// # Arguments
    ///
    /// * `entity_name` - The name of the entity (will be normalized)
    /// * `entity_type` - The type of entity (e.g., "PERSON", "ORGANIZATION")
    /// * `description` - Description of the entity
    /// * `source_id` - ID of the source chunk
    /// * `file_path` - Optional source file path
    pub fn new(
        entity_name: String,
        entity_type: String,
        description: String,
        source_id: String,
        file_path: Option<String>,
    ) -> Self {
        let normalized_name = Self::normalize_name(&entity_name);
        Self {
            id: normalized_name.clone(),
            entity_name: normalized_name,
            entity_type: entity_type.to_uppercase(),
            description,
            source_id,
            file_path,
            created_at: Utc::now(),
        }
    }

    /// Add a new source ID to the entity.
    ///
    /// Source IDs are appended with a pipe separator. If the maximum
    /// number of sources is exceeded, older sources may be removed
    /// based on the retention strategy.
    pub fn add_source(&mut self, source_id: &str) {
        if self.source_id.is_empty() {
            self.source_id = source_id.to_string();
        } else {
            let sources: Vec<&str> = self.source_id.split('|').collect();
            if !sources.contains(&source_id) {
                self.source_id = format!("{}|{}", self.source_id, source_id);
            }
        }
    }

    /// Get the number of source chunks.
    pub fn source_count(&self) -> usize {
        if self.source_id.is_empty() {
            0
        } else {
            self.source_id.split('|').count()
        }
    }

    /// Get source IDs as a vector.
    pub fn get_sources(&self) -> Vec<&str> {
        if self.source_id.is_empty() {
            Vec::new()
        } else {
            self.source_id.split('|').collect()
        }
    }

    /// Merge another entity's information into this one.
    ///
    /// This is used when the same entity is found in multiple chunks.
    /// The descriptions are concatenated and sources are merged.
    pub fn merge(&mut self, other: &GraphEntity) {
        // Append description with separator
        if !other.description.is_empty() {
            if self.description.is_empty() {
                self.description = other.description.clone();
            } else {
                self.description = format!("{}\n{}", self.description, other.description);
            }
        }

        // Merge sources
        for source in other.get_sources() {
            self.add_source(source);
        }

        // Merge file paths
        if let Some(ref other_path) = other.file_path {
            match &mut self.file_path {
                Some(existing) => {
                    if !existing.contains(other_path) {
                        *existing = format!("{}|{}", existing, other_path);
                    }
                }
                None => {
                    self.file_path = Some(other_path.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_name_normalization() {
        assert_eq!(GraphEntity::normalize_name("openai"), "OPENAI");
        assert_eq!(GraphEntity::normalize_name("  OpenAI  "), "OPENAI");
        assert_eq!(GraphEntity::normalize_name("Open AI"), "OPEN AI");
    }

    #[test]
    fn test_entity_creation() {
        let entity = GraphEntity::new(
            "openai".to_string(),
            "organization".to_string(),
            "An AI company".to_string(),
            "chunk-1".to_string(),
            Some("/doc.txt".to_string()),
        );

        assert_eq!(entity.id, "OPENAI");
        assert_eq!(entity.entity_name, "OPENAI");
        assert_eq!(entity.entity_type, "ORGANIZATION");
        assert_eq!(entity.source_id, "chunk-1");
    }

    #[test]
    fn test_entity_source_management() {
        let mut entity = GraphEntity::new(
            "Test".to_string(),
            "CONCEPT".to_string(),
            "Description".to_string(),
            "chunk-1".to_string(),
            None,
        );

        assert_eq!(entity.source_count(), 1);

        entity.add_source("chunk-2");
        assert_eq!(entity.source_count(), 2);
        assert_eq!(entity.source_id, "chunk-1|chunk-2");

        // Adding duplicate should not increase count
        entity.add_source("chunk-1");
        assert_eq!(entity.source_count(), 2);
    }

    #[test]
    fn test_entity_merge() {
        let mut entity1 = GraphEntity::new(
            "Entity".to_string(),
            "TYPE".to_string(),
            "Description 1".to_string(),
            "chunk-1".to_string(),
            Some("/file1.txt".to_string()),
        );

        let entity2 = GraphEntity::new(
            "Entity".to_string(),
            "TYPE".to_string(),
            "Description 2".to_string(),
            "chunk-2".to_string(),
            Some("/file2.txt".to_string()),
        );

        entity1.merge(&entity2);

        assert!(entity1.description.contains("Description 1"));
        assert!(entity1.description.contains("Description 2"));
        assert_eq!(entity1.source_count(), 2);
        assert!(entity1.file_path.as_ref().unwrap().contains("/file1.txt"));
        assert!(entity1.file_path.as_ref().unwrap().contains("/file2.txt"));
    }
}
