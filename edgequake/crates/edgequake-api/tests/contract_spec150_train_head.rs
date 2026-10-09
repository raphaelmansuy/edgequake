//! SPEC-150: HEAD schema train contracts (migrations 160–169 + fossils).
//!
//! These are compile-time / source / manifest contracts — no live Postgres
//! required. Runtime epoch proof remains `make spec150-matrix`.

use std::fs;
use std::path::PathBuf;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

fn numbered_sql_versions() -> Vec<i64> {
    let mut versions = Vec::new();
    for ent in fs::read_dir(migrations_dir()).expect("migrations dir") {
        let ent = ent.expect("dirent");
        let name = ent.file_name().to_string_lossy().into_owned();
        if let Some((num, rest)) = name.split_once('_') {
            if rest.ends_with(".sql") {
                if let Ok(v) = num.parse::<i64>() {
                    versions.push(v);
                }
            }
        }
    }
    versions.sort_unstable();
    versions
}

#[test]
fn head_train_includes_160_through_169() {
    let versions = numbered_sql_versions();
    let max = *versions.last().expect("at least one migration");
    assert_eq!(max, 169, "HEAD schema train must end at 169");
    for v in 160..=169 {
        assert!(
            versions.contains(&v),
            "missing migration {v} in edgequake/migrations/"
        );
    }
}

#[test]
fn manifest_compat_serve_max_matches_head() {
    let raw = include_str!("../../../migrations/manifest.toml");
    let max_line = raw
        .lines()
        .find(|l| l.starts_with("compat_serve_max"))
        .expect("compat_serve_max");
    let digits: String = max_line.chars().filter(|c| c.is_ascii_digit()).collect();
    assert_eq!(digits, "169");
}

#[test]
fn fossils_001_and_019_are_registered() {
    let m = edgequake_migrate_manifest::load();
    let m001 = m.migration.iter().find(|e| e.version == 1).expect("m001");
    let m019 = m.migration.iter().find(|e| e.version == 19).expect("m019");
    assert!(
        !m001.fossils.is_empty(),
        "migration 001 must list the v0.11.0 fossil (issue #195)"
    );
    assert!(
        m001.fossils.iter().any(|f| !f.dev_only),
        "001 fossil must be production-accept"
    );
    assert!(
        !m019.fossils.is_empty(),
        "migration 019 must list the v0.10.6–v0.10.12 fossil"
    );
    assert!(
        m019.fossils.iter().any(|f| !f.dev_only),
        "019 fossil must be production-accept"
    );
}

#[test]
fn apply_uses_sequential_progress_path() {
    let src = include_str!("../src/state/migration_bootstrap/apply.rs");
    assert!(
        src.contains("apply_pending_sequentially"),
        "CLI must apply migrations one-by-one for operator progress"
    );
    assert!(
        src.contains("print_upgrade_path"),
        "CLI must print UPGRADE PATH before apply"
    );
    assert!(
        src.contains("record_migration_step"),
        "each step must record migration_run_step telemetry when table exists"
    );
}

#[test]
fn progress_module_release_map_covers_head() {
    // Source contract (no `postgres` feature required for this integration test).
    let src = include_str!("../src/state/migration_bootstrap/progress.rs");
    assert!(
        src.contains("167..=168 => \"v0.32.0–v0.32.2\""),
        "release_for_schema must map 167–168 to v0.32.x"
    );
    assert!(
        src.contains("169 => \"v0.33.0\""),
        "release_for_schema must map 169 to v0.33.0"
    );
    assert!(
        src.contains("149 => \"v0.26.0–v0.26.10\""),
        "release_for_schema must map 149 to v0.26.x"
    );
    assert!(
        src.contains("150..=159 => \"v0.27.0\""),
        "release_for_schema must map 150–159 to v0.27.0"
    );
}

#[test]
fn sequential_apply_is_restartable_by_design() {
    // Idempotent re-run / interrupted-retry: each step is its own sqlx transaction
    // + ledger row. Re-running `edgequake migrate` after a crash resumes at the
    // first unapplied version (sqlx skips applied checksums).
    let src = include_str!("../src/state/migration_bootstrap/apply.rs");
    assert!(src.contains("one.run(pool)"));
    assert!(src.contains("locking: false"));
    assert!(
        src.contains("finish_migration_run(pool, run_id, \"error\""),
        "failed step must close migration_run as error so operators can resume"
    );
}
