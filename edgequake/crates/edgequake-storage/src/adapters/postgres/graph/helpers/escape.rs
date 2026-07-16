//! Typed string escaping for AGE Cypher and SQL literals (SPEC-054 DRY).
//!
//! SSOT — do not add parallel escapers. Call sites should use:
//! - [`escape_cypher_string`] for values embedded in Cypher string literals
//! - [`escape_sql_literal`] for values embedded in SQL single-quoted literals
//!
//! Prefer **bound** parameters (`$1` / `PgAgtype`) over any escaping on hot paths.

/// Escape a string for safe inclusion in Cypher single-quoted string literals.
pub(in crate::adapters::postgres::graph) fn escape_cypher_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape a string for safe inclusion in SQL single-quoted literals.
pub(in crate::adapters::postgres::graph) fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cypher_escapes_quotes_and_newlines() {
        assert_eq!(escape_cypher_string("hello'world"), "hello\\'world");
        assert_eq!(escape_cypher_string("line\nnew"), "line\\nnew");
    }

    #[test]
    fn sql_doubles_single_quotes() {
        assert_eq!(escape_sql_literal("O'Reilly"), "O''Reilly");
    }
}
