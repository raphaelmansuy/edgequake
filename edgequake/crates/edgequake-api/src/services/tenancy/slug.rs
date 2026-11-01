//! Slug generation, one rule for both scopes (SPEC-181 DRY ledger).
//!
//! Before this module, `handlers/workspaces/helpers.rs::generate_slug` and
//! `handlers/setup.rs::generate_slug` were byte-identical copies, and neither handled the
//! collision that sanitisation creates ("A/B" and "A B" both become `a-b`).

/// Lowercase, ASCII-ish, hyphen-separated. Non-alphanumeric characters collapse into one
/// hyphen and leading/trailing hyphens are trimmed. Unicode letters are kept: a slug is a URL
/// segment, and both Postgres and the browser address bar handle them.
pub fn slugify(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// `base`, `base-2`, `base-3`, … for the first value `taken` does not contain.
///
/// `taken` is a predicate rather than a list so the caller decides how to probe (one query per
/// candidate, or a preloaded set) without this module needing a database handle.
pub fn unique_slug(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2..1000)
        .map(|n| format!("{base}-{n}"))
        .find(|candidate| !taken(candidate))
        // 998 collisions on one name is not a real product state; fall back to a suffix rather
        // than returning a duplicate.
        .unwrap_or_else(|| format!("{base}-{}", uuid::Uuid::new_v4().simple()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn slugify_normalises_case_and_separators() {
        assert_eq!(slugify("Globex Holdings"), "globex-holdings");
        assert_eq!(slugify("  spaced  out  "), "spaced-out");
        assert_eq!(slugify("a__b"), "a-b");
        assert_eq!(slugify("---"), "");
        assert_eq!(slugify("Already-Slugged"), "already-slugged");
    }

    #[test]
    fn sanitised_collisions_are_distinct() {
        assert_eq!(slugify("A/B"), slugify("A B"), "sanitisation collides");
        let mut taken: HashSet<String> = HashSet::new();
        let first = unique_slug(&slugify("A/B"), |c| taken.contains(c));
        taken.insert(first.clone());
        let second = unique_slug(&slugify("A B"), |c| taken.contains(c));
        assert_eq!(first, "a-b");
        assert_eq!(second, "a-b-2");
    }

    #[test]
    fn unique_slug_returns_base_when_free() {
        assert_eq!(unique_slug("acme", |_| false), "acme");
    }

    #[test]
    fn unique_slug_skips_consecutive_taken_candidates() {
        let taken: HashSet<&str> = ["acme", "acme-2", "acme-3"].into_iter().collect();
        assert_eq!(unique_slug("acme", |c| taken.contains(c)), "acme-4");
    }
}
