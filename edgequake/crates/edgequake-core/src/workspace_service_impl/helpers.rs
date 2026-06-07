#[cfg(feature = "postgres")]
use std::collections::HashMap;

// ============ Helper Functions ============

/// Normalize entity types for storage.
///
/// WHY: Consistent normalization ensures that types like "machine" and "MACHINE"
/// map to the same entity type, preventing duplicate type entries in the graph.
///
/// Rules (per SPEC-085):
/// - Trim whitespace
/// - Convert to UPPERCASE
/// - Replace spaces/hyphens with underscores
/// - Skip empty strings
/// - Deduplicate (preserving first occurrence order)
/// - Cap at 50 types to avoid prompt bloat
///
/// Apply `entity_types` list to workspace metadata (create/update).
pub(crate) fn apply_entity_types_metadata(
    metadata: &mut HashMap<String, serde_json::Value>,
    entity_types: Option<Vec<String>>,
) {
    if let Some(entity_types) = entity_types {
        let normalized = normalize_entity_types(&entity_types);
        if normalized.is_empty() {
            metadata.remove("entity_types");
        } else {
            metadata.insert("entity_types".to_string(), serde_json::json!(normalized));
        }
    }
}

/// Apply strict entity-type enforcement flag (default true when key absent).
pub(crate) fn apply_entity_types_strict_metadata(
    metadata: &mut HashMap<String, serde_json::Value>,
    strict: Option<bool>,
) {
    if let Some(strict) = strict {
        if strict {
            metadata.remove("entity_types_strict");
        } else {
            metadata.insert("entity_types_strict".to_string(), serde_json::json!(false));
        }
    }
}

/// @implements SPEC-085: Custom entity configuration normalization
pub(crate) fn normalize_entity_types(types: &[String]) -> Vec<String> {
    const MAX_ENTITY_TYPES: usize = 50;

    let mut seen = std::collections::HashSet::new();
    types
        .iter()
        .filter_map(|t| {
            let normalized = t.trim().to_uppercase().replace([' ', '-'], "_");
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
        .filter(|t| seen.insert(t.clone()))
        .take(MAX_ENTITY_TYPES)
        .collect()
}
