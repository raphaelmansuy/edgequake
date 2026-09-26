//! SPEC-150 WP-4: serving boot must not emit DDL / checksum UPDATE.

#[test]
fn bootstrap_for_serving_does_not_spawn_support_without_escape() {
    let src = include_str!("../src/state/migration_bootstrap/mod.rs");
    assert!(
        src.contains("EDGEQUAKE_SERVE_RECONCILE"),
        "serve reconcile escape hatch must exist"
    );
    assert!(
        src.contains("migrate_cli_mode() || serve_reconcile"),
        "040/139/140/141 spawns must be gated on CLI or SERVE_RECONCILE"
    );
}

#[test]
fn fossil_repair_is_cli_gated() {
    let src = include_str!("../src/state/migration_bootstrap/mod.rs");
    assert!(
        src.contains("let fossil_repairs = if migrate_cli_mode()"),
        "fossil checksum rewrite must be CLI-only"
    );
}

#[test]
fn spawn_for_serving_requires_automatic() {
    let src = include_str!("../../edgequake-storage/src/migration_engine/runner.rs");
    assert!(
        src.contains("!matches!(mode, MigrationMode::Automatic)"),
        "spawn_for_serving must no-op unless EDGEQUAKE_MIGRATION_MODE=automatic"
    );
}
