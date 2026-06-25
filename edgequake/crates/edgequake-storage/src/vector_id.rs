//! Typed vector record identifiers (SPEC-021 R-SOLID-04).
//!
//! # WHY THIS MODULE EXISTS
//!
//! The vector store uses string IDs with an implicit naming convention:
//! - Chunk vectors:        `"{doc_id}-chunk-{index}"`
//! - Entity vectors:       `"{entity_name}"` (normalized, UPPERCASE_UNDERSCORE)
//! - Relationship vectors: `"{source}::{target}"`
//!
//! This convention was scattered as string literals across the pipeline (writer)
//! and query strategies (reader). A mismatch silently returns empty results with
//! no compile-time or runtime error.
//!
//! This module provides typed constructors and parsers so the convention is
//! defined exactly once.
//!
//! # Example
//!
//! ```rust
//! use edgequake_storage::vector_id::VectorId;
//!
//! let id = VectorId::chunk("doc-123", 0);
//! assert_eq!(id.to_storage_id(), "doc-123-chunk-0");
//!
//! let id = VectorId::entity("APPLE_INC");
//! assert_eq!(id.to_storage_id(), "APPLE_INC");
//!
//! let id = VectorId::relationship("APPLE_INC", "TIM_COOK");
//! assert_eq!(id.to_storage_id(), "APPLE_INC::TIM_COOK");
//! ```

/// Typed vector record identifier.
///
/// Encodes the three types of vectors EdgeQuake stores:
/// chunk embeddings, entity embeddings, and relationship embeddings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VectorId {
    /// A chunk of text from a document.
    Chunk { doc_id: String, index: usize },
    /// A named entity extracted from text.
    Entity {
        /// Normalized entity name (UPPERCASE_UNDERSCORE convention).
        name: String,
    },
    /// A relationship between two entities.
    Relationship { source: String, target: String },
}

impl VectorId {
    /// Construct a chunk vector ID.
    pub fn chunk(doc_id: impl Into<String>, index: usize) -> Self {
        Self::Chunk {
            doc_id: doc_id.into(),
            index,
        }
    }

    /// Construct an entity vector ID.
    pub fn entity(name: impl Into<String>) -> Self {
        Self::Entity { name: name.into() }
    }

    /// Construct a relationship vector ID.
    pub fn relationship(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self::Relationship {
            source: source.into(),
            target: target.into(),
        }
    }

    /// Convert to the string ID used in the vector store.
    pub fn to_storage_id(&self) -> String {
        match self {
            Self::Chunk { doc_id, index } => format!("{doc_id}-chunk-{index}"),
            Self::Entity { name } => name.clone(),
            Self::Relationship { source, target } => format!("{source}::{target}"),
        }
    }

    /// Parse a storage ID string back into a typed `VectorId`.
    ///
    /// Returns `None` if the format cannot be determined.
    /// Disambiguation rules:
    /// - Contains `"-chunk-"` with numeric suffix → Chunk
    /// - Contains `"::"` → Relationship  
    /// - Otherwise → Entity (names don't contain `"-chunk-"` or `"::"`)
    pub fn from_storage_id(id: &str) -> Option<Self> {
        // Try Relationship first (contains "::")
        if let Some(sep) = id.find("::") {
            let source = id[..sep].to_string();
            let target = id[sep + 2..].to_string();
            if !source.is_empty() && !target.is_empty() {
                return Some(Self::Relationship { source, target });
            }
        }

        // Try Chunk: must have "-chunk-{numeric}"
        if let Some(chunk_pos) = id.rfind("-chunk-") {
            let index_str = &id[chunk_pos + 7..];
            if let Ok(index) = index_str.parse::<usize>() {
                let doc_id = id[..chunk_pos].to_string();
                if !doc_id.is_empty() {
                    return Some(Self::Chunk { doc_id, index });
                }
            }
        }

        // Fallback: Entity (any non-empty string that isn't chunk/relationship)
        if !id.is_empty() {
            return Some(Self::Entity {
                name: id.to_string(),
            });
        }

        None
    }

    /// Parse a vector ID from metadata JSON.
    ///
    /// Reads `metadata.type` and the appropriate fields.
    pub fn from_metadata(metadata: &serde_json::Value) -> Option<Self> {
        let vtype = metadata.get("type")?.as_str()?;
        match vtype {
            "chunk" => {
                let doc_id = metadata.get("document_id")?.as_str()?.to_string();
                let index = metadata
                    .get("chunk_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                Some(Self::Chunk { doc_id, index })
            }
            "entity" => {
                let name = metadata
                    .get("entity_name")
                    .or_else(|| metadata.get("name"))
                    .and_then(|v| v.as_str())?
                    .to_string();
                Some(Self::Entity { name })
            }
            "relationship" => {
                let source = metadata.get("source")?.as_str()?.to_string();
                let target = metadata.get("target")?.as_str()?.to_string();
                Some(Self::Relationship { source, target })
            }
            _ => None,
        }
    }

    /// Returns the vector type string used in metadata (`"chunk"`, `"entity"`, `"relationship"`).
    pub fn type_str(&self) -> &'static str {
        match self {
            Self::Chunk { .. } => "chunk",
            Self::Entity { .. } => "entity",
            Self::Relationship { .. } => "relationship",
        }
    }

    /// Returns true if this is a chunk vector.
    pub fn is_chunk(&self) -> bool {
        matches!(self, Self::Chunk { .. })
    }

    /// Returns true if this is an entity vector.
    pub fn is_entity(&self) -> bool {
        matches!(self, Self::Entity { .. })
    }

    /// Returns true if this is a relationship vector.
    pub fn is_relationship(&self) -> bool {
        matches!(self, Self::Relationship { .. })
    }
}

impl std::fmt::Display for VectorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_storage_id())
    }
}

#[cfg(test)]
mod tests {
    use super::VectorId;

    #[test]
    fn chunk_roundtrip() {
        let id = VectorId::chunk("doc-123", 7);
        let s = id.to_storage_id();
        assert_eq!(s, "doc-123-chunk-7");
        assert_eq!(VectorId::from_storage_id(&s), Some(id));
    }

    #[test]
    fn entity_roundtrip() {
        let id = VectorId::entity("APPLE_INC");
        let s = id.to_storage_id();
        assert_eq!(s, "APPLE_INC");
        assert_eq!(VectorId::from_storage_id(&s), Some(id));
    }

    #[test]
    fn relationship_roundtrip() {
        let id = VectorId::relationship("APPLE_INC", "TIM_COOK");
        let s = id.to_storage_id();
        assert_eq!(s, "APPLE_INC::TIM_COOK");
        assert_eq!(VectorId::from_storage_id(&s), Some(id));
    }

    #[test]
    fn type_str_values() {
        assert_eq!(VectorId::chunk("d", 0).type_str(), "chunk");
        assert_eq!(VectorId::entity("E").type_str(), "entity");
        assert_eq!(VectorId::relationship("A", "B").type_str(), "relationship");
    }

    #[test]
    fn from_metadata_chunk() {
        let meta = serde_json::json!({
            "type": "chunk",
            "document_id": "doc-abc",
            "chunk_index": 3
        });
        let id = VectorId::from_metadata(&meta).unwrap();
        assert_eq!(id, VectorId::chunk("doc-abc", 3));
    }

    #[test]
    fn from_metadata_entity() {
        let meta = serde_json::json!({
            "type": "entity",
            "entity_name": "APPLE_INC"
        });
        let id = VectorId::from_metadata(&meta).unwrap();
        assert_eq!(id, VectorId::entity("APPLE_INC"));
    }

    #[test]
    fn from_metadata_relationship() {
        let meta = serde_json::json!({
            "type": "relationship",
            "source": "APPLE_INC",
            "target": "TIM_COOK"
        });
        let id = VectorId::from_metadata(&meta).unwrap();
        assert_eq!(id, VectorId::relationship("APPLE_INC", "TIM_COOK"));
    }

    #[test]
    fn disambiguation_chunk_vs_entity() {
        // An entity name containing "-chunk-" would be pathological but let's test
        // that the chunk-specific suffix (numeric) is required
        let s = "some-entity-with-chunk-in-name-chunk-notanumber";
        // This has "-chunk-" but no numeric suffix → should be Entity
        let id = VectorId::from_storage_id(s);
        // rfind finds the LAST occurrence; "notanumber" is not numeric → Entity
        assert!(matches!(id, Some(VectorId::Entity { .. })));
    }

    #[test]
    fn storage_id_no_collisions_between_types() {
        // A chunk vector and an entity that happens to have a chunk-like name
        // parse differently only if the numeric suffix is present.
        let chunk = VectorId::chunk("ABC", 0).to_storage_id();
        assert_eq!(chunk, "ABC-chunk-0");

        // An entity named "ABC-chunk-0" would have the same storage ID as above.
        // This documents the known ambiguity: from_storage_id cannot distinguish
        // a chunk ID from an entity with the same string. Use from_metadata when
        // the type field is available (preferred in all query paths).
        let parsed = VectorId::from_storage_id(&chunk);
        // "ABC-chunk-0" → parsed as Chunk (numeric suffix present)
        assert_eq!(parsed, Some(VectorId::chunk("ABC", 0)));
    }
}
