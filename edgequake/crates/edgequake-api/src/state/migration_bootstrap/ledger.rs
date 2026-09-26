//! Embedded migrator ledger and pending-version helpers (SPEC-150).

use std::collections::HashSet;

use sqlx::PgPool;

use super::helpers;
use super::readiness::BOOT_GATE_REFUSAL_PREFIX;

/// Embedded sqlx migrator (SSOT for versions + checksums).
pub(crate) static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// SPEC-091 irreversible drop versions (LD-07) — human-gated behind `--confirm-drop`.
/// SSOT: `edgequake/migrations/manifest.toml` (`irreversible_drop`).
pub fn irreversible_drop_versions() -> &'static [i64] {
    edgequake_migrate_manifest::irreversible_drop_versions()
}

/// SPEC-105 LAW-L5 — post-drop empty-residue assert (expandable, but deferred
/// while durable `eq_*` rows remain so ≤0.22 mid-upgrade `migrate` / boot stay unblocked).
/// SSOT: `edgequake/migrations/manifest.toml` (`legacy_cutover_assert`).
pub fn legacy_cutover_assert_version() -> i64 {
    edgequake_migrate_manifest::legacy_cutover_assert_version()
}

/// True when `version` is an irreversible SPEC-091 drop migration.
pub fn is_irreversible_drop(version: i64) -> bool {
    edgequake_migrate_manifest::is_irreversible_drop(version)
}

/// True when `version` is the SPEC-105 legacy cutover assert migration.
pub fn is_legacy_cutover_assert(version: i64) -> bool {
    edgequake_migrate_manifest::is_legacy_cutover_assert(version)
}

/// True when every pending version is an irreversible drop (no expandable drift).
pub fn pending_only_irreversible_drops(pending: &[i64]) -> bool {
    !pending.is_empty() && pending.iter().copied().all(is_irreversible_drop)
}

/// Serve / soft-exit migrate when only DROP OLD (and optionally deferred 142) remain.
///
/// LAW-L5: while legacy rows exist, 142 must not block expandable migrate or boot.
pub fn pending_ok_to_serve(pending: &[i64], defer_legacy_cutover_assert: bool) -> bool {
    !pending.is_empty()
        && pending.iter().copied().all(|v| {
            is_irreversible_drop(v) || (defer_legacy_cutover_assert && is_legacy_cutover_assert(v))
        })
}

/// Expandable apply set: omit irreversible drops; omit 142 when deferred by residue.
pub fn expandable_apply_versions(pending: &[i64], defer_legacy_cutover_assert: bool) -> Vec<i64> {
    pending
        .iter()
        .copied()
        .filter(|v| include_in_expandable_apply(*v, defer_legacy_cutover_assert))
        .collect()
}

/// True when an embedded migration should run under ExpandableOnly.
pub fn include_in_expandable_apply(version: i64, defer_legacy_cutover_assert: bool) -> bool {
    !(is_irreversible_drop(version)
        || (defer_legacy_cutover_assert && is_legacy_cutover_assert(version)))
}

/// Highest pending expandable version strictly below the lowest pending irreversible.
///
/// Used by `edgequake migrate` to apply safe schema first when a drop gate is closed
/// (first-principles: consent gates destroy-data steps, not expandable DDL).
pub fn max_expandable_target(pending: &[(i64, String)]) -> Option<i64> {
    let lowest_irreversible = pending
        .iter()
        .map(|(v, _)| *v)
        .filter(|v| is_irreversible_drop(*v))
        .min()?;
    pending
        .iter()
        .map(|(v, _)| *v)
        .filter(|v| !is_irreversible_drop(*v) && *v < lowest_irreversible)
        .max()
}

/// Pending expandable versions (SAFE SCHEMA), including those that sit *after*
/// a pending irreversible drop (sqlx cannot skip the drop in a contiguous train,
/// so the CLI applies these via a filtered migrator that omits drop versions).
pub fn pending_expandable_versions(pending: &[(i64, String)]) -> Vec<i64> {
    pending
        .iter()
        .map(|(v, _)| *v)
        .filter(|v| !is_irreversible_drop(*v))
        .collect()
}

/// Single refusal-message builder (LAW-B3) — contract-pinned by
/// `contract_spec091_boot_gate`. Every element is load-bearing: pending count +
/// versions, the dry-run preview, the apply command, the runbook path.
pub fn boot_gate_pending_message(pending: &[i64]) -> String {
    let irreversible: Vec<i64> = pending
        .iter()
        .copied()
        .filter(|v| is_irreversible_drop(*v))
        .collect();
    let expandable: Vec<i64> = pending
        .iter()
        .copied()
        .filter(|v| !is_irreversible_drop(*v))
        .collect();
    format!(
        "{BOOT_GATE_REFUSAL_PREFIX} STOP — database schema is behind this binary: \
         {} pending migration(s): {pending:?}.\n\
         \n\
         First principles: the server will not start until SAFE SCHEMA migrations \
         are applied. DROP OLD (destroy-data) steps stay human-gated.\n\
         \n\
         Breakdown:\n\
         \x20 SAFE SCHEMA still missing: {expandable:?}\n\
         \x20 DROP OLD (optional, needs --confirm-drop): {irreversible:?}\n\
         \n\
         Next steps:\n\
         \x20 1. edgequake migrate dry-run     # preview (zero writes)\n\
         \x20 2. edgequake migrate             # apply SAFE SCHEMA (DROP OLD still needs --confirm-drop)\n\
         \n\
         Runbook: docs/operations/spec091-upgrade-from-v0.22.0.md",
        pending.len(),
    )
}

/// Downgrade refusal (LAW-B5): database applied a newer schema than this
/// binary embeds. Silent downgrade-serve is schema drift by omission.
pub fn boot_gate_downgrade_message(applied_max: i64, embedded_max: i64) -> String {
    format!(
        "{BOOT_GATE_REFUSAL_PREFIX} database is NEWER than this binary \
         (applied v{applied_max} > embedded v{embedded_max}). Run the binary that \
         matches the schema, or restore a compatible backup. \
         (SPEC-091 LAW-B5 downgrade protection.)"
    )
}

/// SPEC-091 Doc 17 (LD-15): `EDGEQUAKE_ALLOW_BOOT_MIGRATE` was removed as a
/// behavior input. One-release warn-and-ignore shim — the gate is fail-closed
/// regardless. Warns only on a TRUTHY value (someone relying on the old
/// escape); an explicit `=0` already states the new behavior, so it stays quiet.
pub fn warn_if_removed_boot_flag_set() {
    let truthy = matches!(
        std::env::var("EDGEQUAKE_ALLOW_BOOT_MIGRATE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    );
    if truthy {
        tracing::warn!(
            target: "edgequake.migration",
            "EDGEQUAKE_ALLOW_BOOT_MIGRATE was removed (SPEC-091 LD-15) and is ignored — \
             schema apply is `edgequake migrate` only; serving boot is fail-closed verify-only"
        );
    }
}

/// Set by `edgequake migrate` so support DDL apply runs without the boot escape.
pub fn migrate_cli_mode() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_MIGRATE_CLI")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Schema-vs-binary disagreement (LAW-B2/B3) — one derivation, shared by the
/// boot gate and `/health` (DRY). `None` when the ledger is unreadable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaDrift {
    /// Embedded migrations not yet applied.
    pub pending_count: usize,
    /// Database applied a version beyond this binary's embedded latest (LAW-B5).
    pub db_newer_than_binary: bool,
}

impl SchemaDrift {
    /// True when serving requires operator action (`edgequake migrate` or a
    /// matching binary) before the schema agrees with this binary.
    pub fn migration_required(&self) -> bool {
        self.pending_count > 0 || self.db_newer_than_binary
    }
}

/// Live drift read for `/health` and the boot gate (LAW-B3: same derivation,
/// no second computation).
pub async fn schema_drift(pool: &PgPool) -> Option<SchemaDrift> {
    let applied = fetch_applied_versions(pool).await.ok()?;
    let embedded_max = MIGRATOR
        .migrations
        .iter()
        .map(|m| m.version)
        .max()
        .unwrap_or(0);
    let applied_max = applied.iter().copied().max().unwrap_or(0);
    Some(SchemaDrift {
        pending_count: MIGRATOR
            .migrations
            .iter()
            .filter(|m| !applied.contains(&m.version))
            .count(),
        db_newer_than_binary: applied_max > embedded_max,
    })
}

/// True when the database has never been migrated (zero successful
/// `_sqlx_migrations` rows). Used by the CLI to scope irreversible-op consent:
/// on a fresh install the drop migrations cannot destroy anything (LAW-C5 —
/// consent is required only when there is something to lose).
pub async fn is_fresh_database(pool: &PgPool) -> Result<bool, sqlx::Error> {
    Ok(fetch_applied_versions(pool).await?.is_empty())
}

pub(crate) async fn fetch_applied_versions(pool: &PgPool) -> Result<HashSet<i64>, sqlx::Error> {
    if !helpers::sqlx_migrations_table_exists(pool).await? {
        return Ok(HashSet::new());
    }

    let rows: Vec<i64> = sqlx::query_scalar(
        "SELECT version FROM _sqlx_migrations WHERE success = true ORDER BY version",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Pending sqlx migrations as `(version, description)` for operator console (`edgequake migrate`).
pub async fn list_pending_migrations(pool: &PgPool) -> Result<Vec<(i64, String)>, sqlx::Error> {
    let applied = fetch_applied_versions(pool).await?;
    let mut pending: Vec<(i64, String)> = MIGRATOR
        .migrations
        .iter()
        .filter(|m| !applied.contains(&m.version))
        .map(|m| (m.version, m.description.to_string()))
        .collect();
    pending.sort_by_key(|(v, _)| *v);
    Ok(pending)
}

/// Description for an embedded migration version (empty string if unknown).
pub fn migration_description(version: i64) -> String {
    MIGRATOR
        .migrations
        .iter()
        .find(|m| m.version == version)
        .map(|m| m.description.to_string())
        .unwrap_or_default()
}
