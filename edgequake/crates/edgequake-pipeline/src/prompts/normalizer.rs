//! Entity name normalization utilities.
//!
//! Provides consistent entity naming across extractions to ensure
//! proper graph node merging.
//!
//! # WHY Normalization Matters
//!
//! Without normalization, the same entity extracted from different chunks might
//! be stored as separate nodes in the knowledge graph:
//!
//! - "John Doe" (from chunk 1)
//! - "john doe" (from chunk 2)  
//! - "JOHN DOE" (from chunk 3)
//! - "The John Doe" (from chunk 4)
//!
//! This leads to:
//! 1. **Graph fragmentation**: Same entity exists as multiple disconnected nodes
//! 2. **Lost relationships**: Edges only connect to one variant
//! 3. **Query failures**: Search for "John Doe" misses "JOHN DOE" nodes
//! 4. **Inflated entity counts**: 4 nodes instead of 1
//!
//! By normalizing to `JOHN_DOE`, all references merge into a single node,
//! preserving the complete relationship graph.

/// Normalize entity name to consistent format.
///
/// Applies the following transformations:
/// - Trims whitespace
/// - Removes common prefixes (The, A, An)
/// - Removes possessive suffixes ('s)
/// - Converts to title case
/// - Replaces spaces with underscores
/// - Converts to uppercase
///
/// # Examples
///
/// ```rust
/// use edgequake_pipeline::prompts::normalize_entity_name;
///
/// assert_eq!(normalize_entity_name("John Doe"), "JOHN_DOE");
/// assert_eq!(normalize_entity_name("the company"), "COMPANY");
/// assert_eq!(normalize_entity_name("  Sarah  Chen  "), "SARAH_CHEN");
/// ```
pub fn normalize_entity_name(raw_name: &str) -> String {
    let trimmed = raw_name.trim();

    // Remove common prefixes that don't add identity
    let without_prefix = trimmed
        .strip_prefix("The ")
        .or_else(|| trimmed.strip_prefix("the "))
        .or_else(|| trimmed.strip_prefix("A "))
        .or_else(|| trimmed.strip_prefix("a "))
        .or_else(|| trimmed.strip_prefix("An "))
        .or_else(|| trimmed.strip_prefix("an "))
        .unwrap_or(trimmed);

    // Split by whitespace, normalize each word (removing possessives), and rejoin
    without_prefix
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|word| {
            // Remove possessive suffix from each word
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

/// Normalize for comparison (more lenient than storage normalization).
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
        // Hyphens and other meaningful characters should be preserved
        assert_eq!(normalize_entity_name("New-York"), "NEW-YORK");
        assert_eq!(normalize_entity_name("C++"), "C++");
    }
}
