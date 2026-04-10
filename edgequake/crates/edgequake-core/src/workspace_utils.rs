//! Pure parsing and normalization utilities for workspace/tenant data.
//!
//! WHY separate module: These functions are pure (no DB dependencies) and must
//! be testable without the `postgres` feature gate. Extracted from
//! workspace_row_types.rs in OODA-17.

use crate::types::{MembershipRole, TenantPlan};

// ============ Parsing Helpers ============

/// Parse TenantPlan from a string stored in metadata JSONB.
///
/// Unknown values default to `Free` — this is the safe fallback for
/// corrupted or migrated data.
pub(crate) fn parse_plan(s: &str) -> TenantPlan {
    match s.to_lowercase().as_str() {
        "basic" => TenantPlan::Basic,
        "pro" => TenantPlan::Pro,
        "enterprise" => TenantPlan::Enterprise,
        _ => TenantPlan::Free,
    }
}

/// Parse MembershipRole from a string stored in the role column.
///
/// Unknown values default to `Member` — least-privilege principle.
pub(crate) fn parse_role(s: &str) -> MembershipRole {
    match s.to_lowercase().as_str() {
        "readonly" => MembershipRole::Readonly,
        "admin" => MembershipRole::Admin,
        "owner" => MembershipRole::Owner,
        _ => MembershipRole::Member,
    }
}

// ============ Entity Type Normalization ============

/// Normalize entity type strings for consistent knowledge graph labeling.
///
/// Rules (per SPEC-085):
/// - Trim whitespace
/// - Convert to UPPERCASE
/// - Replace spaces/hyphens with underscores
/// - Skip empty strings
/// - Deduplicate (preserving first occurrence order)
/// - Cap at 50 types to avoid prompt bloat
///
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============ normalize_entity_types ============

    #[test]
    fn normalize_empty_input() {
        assert_eq!(normalize_entity_types(&[]), Vec::<String>::new());
    }

    #[test]
    fn normalize_whitespace_only_entries_filtered() {
        let input: Vec<String> = vec!["  ".into(), "\t".into(), "".into()];
        assert_eq!(normalize_entity_types(&input), Vec::<String>::new());
    }

    #[test]
    fn normalize_uppercases_and_replaces_separators() {
        let input: Vec<String> = vec!["key person".into(), "co-worker".into()];
        assert_eq!(
            normalize_entity_types(&input),
            vec!["KEY_PERSON", "CO_WORKER"]
        );
    }

    #[test]
    fn normalize_deduplicates_case_insensitive() {
        let input: Vec<String> = vec!["Person".into(), "PERSON".into(), "person".into()];
        // WHY: Only first occurrence kept after normalization
        assert_eq!(normalize_entity_types(&input), vec!["PERSON"]);
    }

    #[test]
    fn normalize_trims_whitespace() {
        let input: Vec<String> = vec!["  Person  ".into()];
        assert_eq!(normalize_entity_types(&input), vec!["PERSON"]);
    }

    #[test]
    fn normalize_caps_at_50_types() {
        let input: Vec<String> = (0..100).map(|i| format!("type_{}", i)).collect();
        let result = normalize_entity_types(&input);
        assert_eq!(result.len(), 50);
        assert_eq!(result[0], "TYPE_0");
        assert_eq!(result[49], "TYPE_49");
    }

    #[test]
    fn normalize_preserves_unicode() {
        let input: Vec<String> = vec!["café".into(), "naïve".into()];
        let result = normalize_entity_types(&input);
        assert_eq!(result, vec!["CAFÉ", "NAÏVE"]);
    }

    #[test]
    fn normalize_mixed_valid_and_empty() {
        let input: Vec<String> = vec!["".into(), "valid".into(), "  ".into(), "also-valid".into()];
        assert_eq!(normalize_entity_types(&input), vec!["VALID", "ALSO_VALID"]);
    }

    // ============ parse_plan ============

    #[test]
    fn parse_plan_known_values() {
        assert!(matches!(parse_plan("free"), TenantPlan::Free));
        assert!(matches!(parse_plan("basic"), TenantPlan::Basic));
        assert!(matches!(parse_plan("pro"), TenantPlan::Pro));
        assert!(matches!(parse_plan("enterprise"), TenantPlan::Enterprise));
    }

    #[test]
    fn parse_plan_case_insensitive() {
        assert!(matches!(parse_plan("PRO"), TenantPlan::Pro));
        assert!(matches!(parse_plan("Enterprise"), TenantPlan::Enterprise));
    }

    #[test]
    fn parse_plan_unknown_defaults_to_free() {
        assert!(matches!(parse_plan(""), TenantPlan::Free));
        assert!(matches!(parse_plan("gold"), TenantPlan::Free));
        assert!(matches!(parse_plan("premium"), TenantPlan::Free));
    }

    // ============ parse_role ============

    #[test]
    fn parse_role_known_values() {
        assert!(matches!(parse_role("readonly"), MembershipRole::Readonly));
        assert!(matches!(parse_role("admin"), MembershipRole::Admin));
        assert!(matches!(parse_role("owner"), MembershipRole::Owner));
        assert!(matches!(parse_role("member"), MembershipRole::Member));
    }

    #[test]
    fn parse_role_case_insensitive() {
        assert!(matches!(parse_role("ADMIN"), MembershipRole::Admin));
        assert!(matches!(parse_role("Owner"), MembershipRole::Owner));
    }

    #[test]
    fn parse_role_unknown_defaults_to_member() {
        assert!(matches!(parse_role(""), MembershipRole::Member));
        assert!(matches!(parse_role("superadmin"), MembershipRole::Member));
        assert!(matches!(parse_role("viewer"), MembershipRole::Member));
    }
}
