//! Entity name normalization SSOT (SPEC-027 IMP-026 / ARCH-002).
//!
//! Single canonical normalizer for graph entities, relationships, and lineage.

/// Normalize user-supplied entity names to graph storage keys.
pub fn normalize_entity_name(name: &str) -> String {
    edgequake_storage::normalize_entity_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_to_uppercase_underscores() {
        assert_eq!(
            normalize_entity_name("Machine Learning"),
            "MACHINE_LEARNING"
        );
    }
}
