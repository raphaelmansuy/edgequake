//! SPEC-146 classification / share-mode catalog (SSOT for admit + OpenAPI).
//!
//! LAW-146: one catalog — handlers and FE must not invent parallel enums.

/// Clearance / classification lattice values (default template).
pub const CLASSIFICATIONS: &[&str] = &["public", "internal", "confidential", "secret"];

/// Document share modes (CHECK constraint + AllowSet decision tree).
pub const SHARE_MODES: &[&str] = &["workspace", "acl", "classified", "owner_only"];

/// Default classification when unset.
pub const DEFAULT_CLASSIFICATION: &str = "internal";

/// Default share mode (legacy workspace-visible).
pub const DEFAULT_SHARE_MODE: &str = "workspace";

/// Post-query answer when allow-set ∩ retrieval is empty (G-146-52 / existence-hiding).
pub const ZERO_AUTHZ_ANSWER: &str = "No matching results.";

/// Normalize classification to a known catalog value (fail-open → internal).
pub fn normalize_classification(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    if CLASSIFICATIONS.iter().any(|c| *c == lower) {
        lower
    } else if lower.is_empty() {
        DEFAULT_CLASSIFICATION.into()
    } else {
        // Preserve custom workspace values but prefer catalog when matching.
        lower
    }
}

/// Normalize share_mode to a known catalog value (unknown → workspace).
pub fn normalize_share_mode(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    if SHARE_MODES.iter().any(|m| *m == lower) {
        lower
    } else {
        DEFAULT_SHARE_MODE.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_mode_unknown_becomes_workspace() {
        assert_eq!(normalize_share_mode("nope"), "workspace");
        assert_eq!(normalize_share_mode("ACL"), "acl");
    }

    #[test]
    fn classification_empty_is_internal() {
        assert_eq!(normalize_classification(""), "internal");
        assert_eq!(normalize_classification("Secret"), "secret");
    }
}
