//! Entity name normalization — thin re-export of the canonical implementation.
//!
//! # WHY THIS DELEGATES
//!
//! The single source of truth for entity-name normalization lives in
//! `edgequake-storage::entity_id` (RC-6 / P-G1), because entity identity is a
//! storage concept and `edgequake-storage` is the lowest layer (the pipeline
//! depends on storage, not the reverse). Duplicating the algorithm here would
//! reintroduce the three-convention divergence this module was created to fix.
//!
//! This module re-exports the canonical function so existing imports
//! (`edgequake_pipeline::prompts::normalize_entity_name`) keep working with no
//! behavioral change.

pub use edgequake_storage::entity_id::normalize_entity_name;

/// Normalize for comparison (more lenient than storage normalization).
///
/// This is a *display/comparison* helper only — it is never used to construct
/// storage ids. Storage ids are built via [`edgequake_storage::EntityId`].
#[allow(dead_code)]
pub fn normalize_for_comparison(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Check if two entity names are equivalent after normalization.
#[allow(dead_code)]
pub fn entities_match(name1: &str, name2: &str) -> bool {
    normalize_entity_name(name1) == normalize_entity_name(name2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_normalization() {
        assert_eq!(normalize_entity_name("John Doe"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("john doe"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("JOHN DOE"), "JOHN_DOE");
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(normalize_entity_name("  John  Doe  "), "JOHN_DOE");
        assert_eq!(normalize_entity_name("\tJohn\nDoe\r"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("John   Doe"), "JOHN_DOE");
    }

    #[test]
    fn test_prefix_removal() {
        assert_eq!(normalize_entity_name("The Company"), "COMPANY");
        assert_eq!(normalize_entity_name("the company"), "COMPANY");
        assert_eq!(normalize_entity_name("A Person"), "PERSON");
        assert_eq!(normalize_entity_name("An Event"), "EVENT");
    }

    #[test]
    fn test_possessive_removal() {
        assert_eq!(normalize_entity_name("John's"), "JOHN");
        assert_eq!(
            normalize_entity_name("Company's Products"),
            "COMPANY_PRODUCTS"
        );
    }

    #[test]
    fn test_title_case_conversion() {
        assert_eq!(normalize_entity_name("jOHN dOE"), "JOHN_DOE");
        assert_eq!(normalize_entity_name("mCdonald"), "MCDONALD");
    }

    #[test]
    fn test_empty_and_edge_cases() {
        assert_eq!(normalize_entity_name(""), "");
        assert_eq!(normalize_entity_name("   "), "");
        assert_eq!(normalize_entity_name("A"), "A");
        assert_eq!(normalize_entity_name("I"), "I");
    }

    #[test]
    fn test_entities_match() {
        assert!(entities_match("John Doe", "john doe"));
        assert!(entities_match("The Company", "Company"));
        assert!(entities_match("  Sarah  ", "Sarah"));
        assert!(!entities_match("John", "Jane"));
    }

    #[test]
    fn test_normalize_for_comparison() {
        assert_eq!(normalize_for_comparison("  John  Doe  "), "john doe");
        assert_eq!(normalize_for_comparison("JOHN DOE"), "john doe");
    }

    #[test]
    fn test_special_characters_preserved() {
        assert_eq!(normalize_entity_name("New-York"), "NEW-YORK");
        assert_eq!(normalize_entity_name("C++"), "C++");
    }
}
