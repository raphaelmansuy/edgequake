//! Entity type schema, prompt fragments, and enforcement for LLM extraction output.
//!
//! @implements SPEC-013 entity_extraction / strict limit checkbox

use std::collections::HashMap;

use serde_json::Value;

use super::default_entity_types;

/// Workspace-scoped entity type allow-list and strict/permissive mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityExtractionSchema {
    /// Normalized UPPER_SNAKE types used in prompts and matching.
    pub types: Vec<String>,
    /// When true, unknown types remap to OTHER/CONCEPT/first (GitHub #217).
    /// When false, unknown types pass through (normalized casing only).
    pub strict: bool,
}

impl EntityExtractionSchema {
    /// Default schema: server entity types + strict enforcement (backward compatible).
    pub fn server_default() -> Self {
        Self {
            types: default_entity_types(),
            strict: true,
        }
    }

    /// Read from workspace `metadata` JSONB (`entity_types`, `entity_types_strict`).
    pub fn from_workspace_metadata(metadata: &HashMap<String, Value>) -> Self {
        let types = metadata
            .get("entity_types")
            .and_then(|v| serde_json::from_value::<Vec<String>>(v.clone()).ok())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(default_entity_types);

        let strict = metadata
            .get("entity_types_strict")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Self { types, strict }
    }

    /// Build with explicit types; strict defaults to true.
    pub fn with_types(types: Vec<String>) -> Self {
        Self {
            types,
            strict: true,
        }
    }
}

/// Metadata key for strict mode (default true when absent).
pub const METADATA_ENTITY_TYPES_STRICT: &str = "entity_types_strict";

/// Normalize a raw entity type token to `UPPER_SNAKE_CASE`.
pub fn normalize_type_token(raw: &str) -> String {
    raw.trim()
        .to_uppercase()
        .replace([' ', '-', '/'], "_")
        .trim_matches('_')
        .to_string()
}

/// Enforce entity type against [`EntityExtractionSchema`].
///
/// Returns `(enforced_type, was_remapped)`.
pub fn enforce_entity_type(raw_type: &str, schema: &EntityExtractionSchema) -> (String, bool) {
    let allowed_types = &schema.types;
    if allowed_types.is_empty() {
        return (normalize_type_token(raw_type), false);
    }

    let normalized = normalize_type_token(raw_type);
    if normalized.is_empty() {
        if schema.strict {
            let fallback = pick_strict_fallback(allowed_types);
            return (fallback, true);
        }
        return (normalized, false);
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

    if !schema.strict {
        return (normalized, false);
    }

    let fallback = pick_strict_fallback(allowed_types);
    (fallback, true)
}

fn pick_strict_fallback(allowed_types: &[String]) -> String {
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

/// JSON extractor prompt section for entity types (LLMExtractor).
pub fn json_entity_types_prompt_section(schema: &EntityExtractionSchema) -> String {
    let entity_types_str = schema.types.join(", ");
    if schema.strict {
        format!(
            "## Entity Types (STRICT)\n\
             Use ONLY these types exactly as written — never invent new types: {entity_types_str}\n\
             If nothing fits, use OTHER when listed, otherwise CONCEPT."
        )
    } else {
        format!(
            "## Entity Types (GUIDANCE)\n\
             Prefer these types when they clearly apply: {entity_types_str}\n\
             You may use additional specific type labels when they describe the entity better.\n\
             Do not use OTHER as a catch-all for unrelated entities."
        )
    }
}

/// Strict-mode instruction for SOTA system prompt entity_type field.
pub fn sota_entity_type_instruction(
    schema: &EntityExtractionSchema,
    entity_types_str: &str,
) -> String {
    if schema.strict {
        format!(
            "You MUST use ONLY one of these types exactly as written (same spelling): `{entity_types_str}`. \
             Never invent new types. If nothing fits, use `OTHER` when listed, otherwise use `CONCEPT`."
        )
    } else {
        format!(
            "Prefer one of these types when applicable: `{entity_types_str}`. \
             You may use other specific type labels when they fit better. \
             Do not use `OTHER` as a generic catch-all."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strict_types() -> EntityExtractionSchema {
        EntityExtractionSchema {
            types: vec![
                "PERSON".into(),
                "ORGANIZATION".into(),
                "LOCATION".into(),
                "CONCEPT".into(),
                "OTHER".into(),
            ],
            strict: true,
        }
    }

    fn permissive_types() -> EntityExtractionSchema {
        EntityExtractionSchema {
            types: strict_types().types,
            strict: false,
        }
    }

    #[test]
    fn exact_match_unchanged() {
        let schema = strict_types();
        let (t, remapped) = enforce_entity_type("person", &schema);
        assert_eq!(t, "PERSON");
        assert!(!remapped);
    }

    #[test]
    fn unknown_maps_to_other_when_strict() {
        let schema = strict_types();
        let (t, remapped) = enforce_entity_type("TELEPHONE_NUMBER", &schema);
        assert_eq!(t, "OTHER");
        assert!(remapped);
    }

    #[test]
    fn unknown_passes_through_when_permissive() {
        let schema = permissive_types();
        let (t, remapped) = enforce_entity_type("TELEPHONE_NUMBER", &schema);
        assert_eq!(t, "TELEPHONE_NUMBER");
        assert!(!remapped);
    }

    #[test]
    fn empty_maps_to_fallback_only_when_strict() {
        let schema = strict_types();
        let (t, remapped) = enforce_entity_type("  ", &schema);
        assert_eq!(t, "OTHER");
        assert!(remapped);

        let permissive = permissive_types();
        let (t2, remapped2) = enforce_entity_type("  ", &permissive);
        assert_eq!(t2, "");
        assert!(!remapped2);
    }

    #[test]
    fn metadata_defaults_strict_true() {
        let schema = EntityExtractionSchema::from_workspace_metadata(&HashMap::new());
        assert!(schema.strict);
        assert_eq!(schema.types.len(), default_entity_types().len());
    }

    #[test]
    fn metadata_reads_strict_false() {
        let mut md = HashMap::new();
        md.insert(METADATA_ENTITY_TYPES_STRICT.to_string(), Value::Bool(false));
        let schema = EntityExtractionSchema::from_workspace_metadata(&md);
        assert!(!schema.strict);
    }

    #[test]
    fn json_prompt_strict_mentions_only() {
        let section = json_entity_types_prompt_section(&strict_types());
        assert!(section.contains("STRICT"));
        assert!(section.contains("Use ONLY"));
    }

    #[test]
    fn json_prompt_permissive_no_other_catchall() {
        let section = json_entity_types_prompt_section(&permissive_types());
        assert!(section.contains("GUIDANCE"));
        assert!(section.contains("Do not use OTHER"));
    }
}
