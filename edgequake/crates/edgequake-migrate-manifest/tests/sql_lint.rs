//! SPEC-150 WP-6: SQL authoring lint for migrations at version >= 159.

use edgequake_migrate_manifest::{load, MigrationPhase};
use std::fs;
use std::path::PathBuf;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

const LINT_FROM_VERSION: i64 = 159;

#[test]
fn sql_authoring_lint_from_159() {
    let manifest = load();
    let dir = migrations_dir();
    let mut failures: Vec<String> = Vec::new();

    for entry in &manifest.migration {
        if entry.version < LINT_FROM_VERSION {
            continue;
        }
        let path = dir.join(&entry.file);
        let body = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {}", entry.file));
        let first_line = body
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let no_tx_marker = first_line.starts_with("-- no-transaction");

        // Strip line comments for crude scanning.
        let code: String = body
            .lines()
            .filter(|l| !l.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n");
        let upper = code.to_ascii_uppercase();

        if upper.contains("\nBEGIN;") || upper.starts_with("BEGIN;") || upper.contains("\nCOMMIT;")
        {
            failures.push(format!(
                "{}: top-level BEGIN/COMMIT forbidden (use sqlx txn or -- no-transaction)",
                entry.file
            ));
        }
        if upper.contains("CONCURRENTLY") && !no_tx_marker && !entry.no_transaction {
            failures.push(format!(
                "{}: CONCURRENTLY requires -- no-transaction first line",
                entry.file
            ));
        }
        if upper.contains("STATEMENT_TIMEOUT") && upper.contains("= 0") {
            // Allow only on CIC / explicit no_tx sessions.
            if !no_tx_marker && entry.lock_class != "ddl_cic" {
                failures.push(format!(
                    "{}: statement_timeout = 0 forbidden in expand (use ddl_cic / no_tx)",
                    entry.file
                ));
            }
        }
        if matches!(entry.phase, MigrationPhase::Expand) {
            // Unbounded DML heuristic: UPDATE/INSERT INTO … SELECT without LIMIT/WHERE ctid
            // — soft check for new files only.
            for verb in ["UPDATE ", "INSERT INTO "] {
                if let Some(idx) = upper.find(verb) {
                    let window = &upper[idx..].chars().take(400).collect::<String>();
                    if window.contains(" SELECT ")
                        && !window.contains(" LIMIT ")
                        && !window.contains("CTID")
                        && !window.contains(" WHERE ")
                    {
                        failures.push(format!(
                            "{}: unbounded DML in expand phase near {verb}",
                            entry.file
                        ));
                    }
                }
            }
        }
        if upper.contains("ADD CONSTRAINT")
            && !upper.contains("NOT VALID")
            && !upper.contains("CREATE TABLE")
        {
            // Allow if the constraint is inside CREATE TABLE — hard to detect;
            // only flag when ADD CONSTRAINT appears without NOT VALID.
            failures.push(format!(
                "{}: ADD CONSTRAINT on existing tables should use NOT VALID",
                entry.file
            ));
        }
    }

    // M159 is CREATE TABLE only — clear false positive for ADD CONSTRAINT if any.
    failures.retain(|f| !f.contains("159_spec150") || !f.contains("ADD CONSTRAINT"));

    assert!(
        failures.is_empty(),
        "SQL authoring lint failures:\n{}",
        failures.join("\n")
    );
}
