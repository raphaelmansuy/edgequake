//! GraphRelationship type definition.
//!
//! A GraphRelationship represents a connection between two entities
//! in the knowledge graph.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Separator for relationship IDs.
pub const RELATIONSHIP_SEP: &str = "<SEP>";

/// A relationship between two entities.
///
/// Relationships are the edges in the knowledge graph. They connect
/// entities and describe the nature of their connection.
///
/// # ID Format
///
/// Relationship IDs are formatted as `ENTITY1<SEP>ENTITY2` where the
/// entities are sorted alphabetically to ensure consistency regardless
/// of which direction the relationship was discovered.
///
/// # Example
///
/// ```rust
/// use edgequake_core::types::GraphRelationship;
///
/// let rel = GraphRelationship::new(
///     "OpenAI".to_string(),
///     "Sam Altman".to_string(),
///     "Sam Altman is the CEO of OpenAI".to_string(),
///     Some("leadership, management".to_string()),
///     1.0,
///     "chunk-123".to_string(),
///     None,
/// );
/// assert!(rel.id.contains("<SEP>"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelationship {
    /// Composite key: `entity1<SEP>entity2` (alphabetically sorted)
    pub id: String,
    /// Source entity name (normalized)
    pub source_entity: String,
    /// Target entity name (normalized)
    pub target_entity: String,
    /// Relationship description
    pub description: String,
    /// Keywords describing relationship (pipe-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// Relationship weight/strength
    pub weight: f32,
    /// Pipe-separated chunk IDs (sources)
    pub source_id: String,
    /// Source file path (pipe-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl GraphRelationship {
    /// Maximum number of source IDs to retain per relationship.
    pub const MAX_SOURCE_IDS: usize = 300;

    /// Generate relationship ID from source and target entities.
    ///
    /// The entities are sorted alphabetically to ensure the same
    /// relationship always has the same ID regardless of direction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use edgequake_core::types::GraphRelationship;
    ///
    /// let id1 = GraphRelationship::generate_id("A", "B");
    /// let id2 = GraphRelationship::generate_id("B", "A");
    /// assert_eq!(id1, id2);
    /// ```
    pub fn generate_id(source: &str, target: &str) -> String {
        let normalized_source = source.trim().to_uppercase();
        let normalized_target = target.trim().to_uppercase();

        // Sort alphabetically for consistent key regardless of direction
        if normalized_source <= normalized_target {
            format!(
                "{}{}{}",
                normalized_source, RELATIONSHIP_SEP, normalized_target
            )
        } else {
            format!(
                "{}{}{}",
                normalized_target, RELATIONSHIP_SEP, normalized_source
            )
        }
    }

    /// Parse a relationship ID into its component entity names.
    ///
    /// Returns None if the ID is not properly formatted.
    pub fn parse_id(id: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = id.split(RELATIONSHIP_SEP).collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    /// Create a new relationship.
    ///
    /// # Arguments
    ///
    /// * `source_entity` - Source entity name
    /// * `target_entity` - Target entity name
    /// * `description` - Description of the relationship
    /// * `keywords` - Optional keywords describing the relationship
    /// * `weight` - Relationship strength/weight
    /// * `source_id` - ID of the source chunk
    /// * `file_path` - Optional source file path
    pub fn new(
        source_entity: String,
        target_entity: String,
        description: String,
        keywords: Option<String>,
        weight: f32,
        source_id: String,
        file_path: Option<String>,
    ) -> Self {
        let normalized_source = source_entity.trim().to_uppercase();
        let normalized_target = target_entity.trim().to_uppercase();
        let id = Self::generate_id(&normalized_source, &normalized_target);

        Self {
            id,
            source_entity: normalized_source,
            target_entity: normalized_target,
            description,
            keywords,
            weight,
            source_id,
            file_path,
            created_at: Utc::now(),
        }
    }

    /// Add a new source ID to the relationship.
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

    /// Increment the relationship weight.
    pub fn increment_weight(&mut self, delta: f32) {
        self.weight += delta;
    }

    /// Merge another relationship's information into this one.
    ///
    /// This is used when the same relationship is found in multiple chunks.
    pub fn merge(&mut self, other: &GraphRelationship) {
        // Append description
        if !other.description.is_empty() {
            if self.description.is_empty() {
                self.description = other.description.clone();
            } else {
                self.description = format!("{}\n{}", self.description, other.description);
            }
        }

        // Merge keywords
        if let Some(ref other_keywords) = other.keywords {
            match &mut self.keywords {
                Some(existing) => {
                    for kw in other_keywords.split('|') {
                        if !existing.contains(kw) {
                            *existing = format!("{}|{}", existing, kw);
                        }
                    }
                }
                None => {
                    self.keywords = Some(other_keywords.clone());
                }
            }
        }

        // Increment weight
        self.weight += other.weight;

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

    /// Check if this relationship involves a specific entity.
    pub fn involves_entity(&self, entity_name: &str) -> bool {
        let normalized = entity_name.trim().to_uppercase();
        self.source_entity == normalized || self.target_entity == normalized
    }

    /// Get the other entity in the relationship given one entity.
    ///
    /// Returns None if the given entity is not part of this relationship.
    pub fn get_other_entity(&self, entity_name: &str) -> Option<&str> {
        let normalized = entity_name.trim().to_uppercase();
        if self.source_entity == normalized {
            Some(&self.target_entity)
        } else if self.target_entity == normalized {
            Some(&self.source_entity)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_id_generation() {
        let id1 = GraphRelationship::generate_id("Alice", "Bob");
        let id2 = GraphRelationship::generate_id("Bob", "Alice");
        let id3 = GraphRelationship::generate_id("alice", "bob");

        // Same regardless of order
        assert_eq!(id1, id2);
        // Same regardless of case
        assert_eq!(id1, id3);
        // Contains separator
        assert!(id1.contains(RELATIONSHIP_SEP));
    }

    #[test]
    fn test_relationship_id_parsing() {
        let id = GraphRelationship::generate_id("Alice", "Bob");
        let (e1, e2) = GraphRelationship::parse_id(&id).unwrap();

        assert_eq!(e1, "ALICE");
        assert_eq!(e2, "BOB");

        // Invalid ID
        assert!(GraphRelationship::parse_id("invalid").is_none());
    }

    #[test]
    fn test_relationship_creation() {
        let rel = GraphRelationship::new(
            "Alice".to_string(),
            "Bob".to_string(),
            "Alice knows Bob".to_string(),
            Some("friendship".to_string()),
            1.0,
            "chunk-1".to_string(),
            None,
        );

        assert_eq!(rel.source_entity, "ALICE");
        assert_eq!(rel.target_entity, "BOB");
        assert_eq!(rel.weight, 1.0);
    }

    #[test]
    fn test_relationship_merge() {
        let mut rel1 = GraphRelationship::new(
            "A".to_string(),
            "B".to_string(),
            "Description 1".to_string(),
            Some("kw1".to_string()),
            1.0,
            "chunk-1".to_string(),
            None,
        );

        let rel2 = GraphRelationship::new(
            "A".to_string(),
            "B".to_string(),
            "Description 2".to_string(),
            Some("kw2".to_string()),
            0.5,
            "chunk-2".to_string(),
            None,
        );

        rel1.merge(&rel2);

        assert!(rel1.description.contains("Description 1"));
        assert!(rel1.description.contains("Description 2"));
        assert_eq!(rel1.weight, 1.5);
        assert_eq!(rel1.source_count(), 2);
        assert!(rel1.keywords.as_ref().unwrap().contains("kw1"));
        assert!(rel1.keywords.as_ref().unwrap().contains("kw2"));
    }

    #[test]
    fn test_relationship_entity_checks() {
        let rel = GraphRelationship::new(
            "Alice".to_string(),
            "Bob".to_string(),
            "Knows".to_string(),
            None,
            1.0,
            "chunk-1".to_string(),
            None,
        );

        assert!(rel.involves_entity("alice"));
        assert!(rel.involves_entity("BOB"));
        assert!(!rel.involves_entity("Charlie"));

        assert_eq!(rel.get_other_entity("alice"), Some("BOB"));
        assert_eq!(rel.get_other_entity("bob"), Some("ALICE"));
        assert_eq!(rel.get_other_entity("charlie"), None);
    }
}
