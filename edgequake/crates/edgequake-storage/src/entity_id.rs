//! Entity identity newtype and the single normalization entry point.
//!
//! # WHY THIS EXISTS (RC-6 / P-G1)
//!
//! Before this module, entity identity was a *convention* enforced nowhere:
//! - The orchestrator merger wrote graph nodes as `JOHN_DOE` (normalized) and
//!   entity vectors as the bare `JOHN_DOE`.
//! - The async processor wrote graph nodes as `John Doe` (raw) and entity
//!   vectors as `entity:John Doe` (raw, prefixed).
//! - The sync upload path wrote graph nodes as `JOHN_DOE` and vectors as
//!   `entity:JOHN_DOE`.
//!
//! Three writers, three conventions. The result was a silently fragmented
//! knowledge graph: the same real entity became multiple nodes, and the entity
//! vectors written by the processor were invisible to the query layer (which
//! looked them up by the graph node id `JOHN_DOE`).
//!
//! First principle: **identity is a value, not a convention.** An `EntityId` is
//! a normalized newtype. The graph node id and the entity vector id are both
//! *derived* from it, so they can never diverge by construction. No writer can
//! build an un-normalized entity id.
//!
//! # Canonical convention
//!
//! - Graph node id  = `EntityId::as_graph_node_id()`  → bare `JOHN_DOE`
//! - Entity vector id = `EntityId::as_vector_id()`    → `entity:JOHN_DOE`
//!
//! The `entity:` prefix on the vector id is what [`VectorId`] decodes back into
//! an [`EntityId`]; keeping the prefix makes the storage id self-describing
//! even when metadata is absent.
//!
//! [`VectorId`]: crate::vector_id::VectorId

use crate::vector_id::VectorId;

/// A normalized entity identity.
///
/// Constructed exclusively via [`EntityId::new`], which runs the single
/// canonical normalizer. The wrapped string is always normalized
/// (UPPERCASE_UNDERSCORE, prefixes/possessives stripped). The empty string is
/// a valid interior value only when the input was empty/whitespace; callers
/// should check [`EntityId::is_empty`] and skip the write in that case (E1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityId(String);

impl EntityId {
    /// Construct a normalized entity id from any raw name.
    ///
    /// Defensively strips a leading `entity:` prefix if a caller accidentally
    /// passes an already-prefixed value (E2), so `EntityId::new("entity:Foo")`
    /// and `EntityId::new("Foo")` produce the same identity.
    pub fn new(raw: &str) -> Self {
        let stripped = raw.strip_prefix("entity:").unwrap_or(raw);
        Self(normalize_entity_name(stripped))
    }

    /// Construct from an already-normalized string, bypassing normalization.
    ///
    /// This is the mirror of [`as_str`](EntityId::as_str) and exists so trusted
    /// readers (e.g. reconstructing an id from a graph node) can avoid
    /// re-normalizing. The caller guarantees the input is already normalized.
    pub fn from_normalized(normalized: impl Into<String>) -> Self {
        Self(normalized.into())
    }

    /// The bare normalized name, used as the graph node id.
    pub fn as_graph_node_id(&self) -> &str {
        &self.0
    }

    /// The prefixed vector storage id (`entity:NAME`), used as the entity
    /// vector id.
    pub fn as_vector_id(&self) -> String {
        format!("entity:{}", self.0)
    }

    /// The bare normalized name as a borrowed string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True if the normalized name is empty (E1). Callers should skip writes
    /// for empty ids and log a warning.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Decode an entity vector storage id (e.g. `entity:JOHN_DOE`) back into an
    /// `EntityId`. Returns `None` for non-entity vector ids (chunk/relationship)
    /// or for an empty entity name.
    pub fn from_vector_storage_id(storage_id: &str) -> Option<Self> {
        VectorId::from_storage_id(storage_id).and_then(|vid| match vid {
            VectorId::Entity { name } => {
                let name = name.strip_prefix("entity:").unwrap_or(&name);
                if name.is_empty() {
                    None
                } else {
                    Some(Self::from_normalized(name))
                }
            }
            _ => None,
        })
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&EntityId> for String {
    fn from(id: &EntityId) -> Self {
        id.0.clone()
    }
}

/// The single canonical entity-name normalizer.
///
/// This is the **only** place entity names are normalized in the codebase.
/// `edgequake-pipeline` re-exports this function as
/// `edgequake_pipeline::prompts::normalize_entity_name` for backwards
/// compatibility; do not duplicate the logic elsewhere (DRY).
///
/// Transformations:
/// - Trims surrounding whitespace.
/// - Strips common leading articles ("The", "A", "An" in any case).
/// - Strips possessive suffixes (`'s`) per word.
/// - Title-cases each word, joins with `_`, uppercases the result.
///
/// Empty / whitespace-only input yields the empty string (E1).
pub fn normalize_entity_name(raw_name: &str) -> String {
    let trimmed = raw_name.trim();

    let without_prefix = trimmed
        .strip_prefix("The ")
        .or_else(|| trimmed.strip_prefix("the "))
        .or_else(|| trimmed.strip_prefix("A "))
        .or_else(|| trimmed.strip_prefix("a "))
        .or_else(|| trimmed.strip_prefix("An "))
        .or_else(|| trimmed.strip_prefix("an "))
        .unwrap_or(trimmed);

    without_prefix
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|word| {
            let without_possessive = word
                .strip_suffix("'s")
                .or_else(|| word.strip_suffix("'s"))
                .unwrap_or(word);
            to_title_case(without_possessive)
        })
        .collect::<Vec<_>>()
        .join("_")
        .to_uppercase()
}

/// Convert a word to title case (first letter uppercase, rest lowercase).
fn to_title_case(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(|c| c.to_lowercase()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_normalizes_casing_variants_to_one_identity() {
        // P-G1 acceptance (unit level): three casing variants → one EntityId.
        let a = EntityId::new("John Doe");
        let b = EntityId::new("john doe");
        let c = EntityId::new("JOHN DOE");
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a.as_graph_node_id(), "JOHN_DOE");
    }

    #[test]
    fn graph_node_id_and_vector_id_are_derived_consistently() {
        let id = EntityId::new("Sarah Chen");
        assert_eq!(id.as_graph_node_id(), "SARAH_CHEN");
        assert_eq!(id.as_vector_id(), "entity:SARAH_CHEN");
    }

    #[test]
    fn vector_id_round_trips_through_storage_id() {
        // P-G1: EntityId → vector id → from_storage_id → EntityId.
        let id = EntityId::new("Apple Inc");
        let vid = id.as_vector_id();
        let decoded = EntityId::from_vector_storage_id(&vid).unwrap();
        assert_eq!(decoded, id);
    }

    #[test]
    fn strips_accidental_entity_prefix() {
        // E2: a caller passing an already-prefixed value must not double-prefix.
        assert_eq!(EntityId::new("entity:Foo Bar"), EntityId::new("Foo Bar"));
        assert_eq!(
            EntityId::new("entity:Foo Bar").as_vector_id(),
            "entity:FOO_BAR"
        );
    }

    #[test]
    fn empty_name_is_empty_identity() {
        // E1.
        assert!(EntityId::new("").is_empty());
        assert!(EntityId::new("   ").is_empty());
        assert_eq!(EntityId::new("").as_graph_node_id(), "");
    }

    #[test]
    fn non_entity_storage_id_decodes_to_none() {
        assert!(EntityId::from_vector_storage_id("doc-123-chunk-0").is_none());
        assert!(EntityId::from_vector_storage_id("A::B").is_none());
    }

    #[test]
    fn prefixes_and_possessives_stripped() {
        assert_eq!(EntityId::new("The Company").as_str(), "COMPANY");
        assert_eq!(EntityId::new("John's").as_str(), "JOHN");
    }

    #[test]
    fn non_ascii_preserved_and_normalized() {
        // E3: non-ASCII names are handled by the existing title-case logic.
        assert_eq!(EntityId::new("René Descartes").as_str(), "RENÉ_DESCARTES");
    }

    #[test]
    fn hyphens_and_special_chars_preserved() {
        assert_eq!(EntityId::new("New-York").as_str(), "NEW-YORK");
        assert_eq!(EntityId::new("C++").as_str(), "C++");
    }
}
