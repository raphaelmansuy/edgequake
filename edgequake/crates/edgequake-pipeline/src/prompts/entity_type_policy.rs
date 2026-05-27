//! Entity type enforcement for LLM extraction output.
//!
//! @implements SPEC-013 / GitHub #217

use std::collections::HashMap;

/// Normalize a raw entity type token to `UPPER_SNAKE_CASE`.
pub fn normalize_type_token(raw: &str) -> String {
    raw.trim()
        .to_uppercase()
        .replace([' ', '-', '/'], "_")
        .trim_matches('_')
        .to_string()
}

/// Enforce that `raw_type` is one of `allowed_types`.
///
/// Returns `(enforced_type, was_remapped)`.
/// - Exact match (case-insensitive) → allowed type as stored
/// - Substring / alias match → closest allowed type
/// - No match → `OTHER` if present, else `CONCEPT` if present, else first allowed type
pub fn enforce_entity_type(raw_type: &str, allowed_types: &[String]) -> (String, bool) {
    if allowed_types.is_empty() {
        return (normalize_type_token(raw_type), false);
    }

    let normalized = normalize_type_token(raw_type);
    if normalized.is_empty() {
        let fallback = pick_fallback(allowed_types);
        return (fallback, true);
    }

    let allowed_map: HashMap<String, String> = allowed_types
        .iter()
        .map(|t| (normalize_type_token(t), t.clone()))
        .collect();

    if let Some(canonical) = allowed_map.get(&normalized) {
        return (canonical.clone(), false);
    }

    // Alias: TELEPHONE_NUMBER → PHONE when PHONE is allowed, etc.
    for (key, canonical) in &allowed_map {
        if normalized.contains(key) || key.contains(&normalized) {
            return (canonical.clone(), true);
        }
    }

    let fallback = pick_fallback(allowed_types);
    (fallback, true)
}

fn pick_fallback(allowed_types: &[String]) -> String {
    let allowed_map: HashMap<String, String> = allowed_types
        .iter()
        .map(|t| (normalize_type_token(t), t.clone()))
        .collect();

    if let Some(other) = allowed_map.get("OTHER") {
        return other.clone();
    }
    if let Some(concept) = allowed_map.get("CONCEPT") {
        return concept.clone();
    }
    allowed_types[0].clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types() -> Vec<String> {
        vec![
            "PERSON".into(),
            "ORGANIZATION".into(),
            "LOCATION".into(),
            "CONCEPT".into(),
            "OTHER".into(),
        ]
    }

    #[test]
    fn exact_match_unchanged() {
        let (t, remapped) = enforce_entity_type("person", &types());
        assert_eq!(t, "PERSON");
        assert!(!remapped);
    }

    #[test]
    fn unknown_maps_to_other() {
        let (t, remapped) = enforce_entity_type("TELEPHONE_NUMBER", &types());
        assert_eq!(t, "OTHER");
        assert!(remapped);
    }

    #[test]
    fn empty_maps_to_fallback() {
        let (t, remapped) = enforce_entity_type("  ", &types());
        assert_eq!(t, "OTHER");
        assert!(remapped);
    }
}
