//! Serving-boot verify path — never applies versioned migrations (SPEC-150 WP-4).

use sqlx::PgPool;

use super::apply::run_postgres_migrations;
use super::ledger::{
    boot_gate_downgrade_message, boot_gate_pending_message, fetch_applied_versions,
    migrate_cli_mode, pending_ok_to_serve, warn_if_removed_boot_flag_set, MIGRATOR,
};
use super::reports::MigrationBootstrapReport;

/// Serving-boot entry (LAW-B1/B2/B5): never applies versioned migrations.
/// Fail-closed verify: expandable pending ⇒ refuse (exit 78 contract), database
/// newer than the binary ⇒ refuse (downgrade protection). Pending **only**
/// irreversible drops (125/126/131) soft-allow with WARN so local upgrade DBs
/// can serve on typed defaults while the human-gated drop stays operator-owned.
/// SPEC-105: pending 142 is soft-allowed while durable legacy rows remain
/// (deferred assert — LAW-L5 ladder). Only `edgequake migrate` (CLI mode) may
/// proceed to apply.
pub async fn bootstrap_for_serving(pool: &PgPool) -> Result<MigrationBootstrapReport, sqlx::Error> {
    warn_if_removed_boot_flag_set();
    if !migrate_cli_mode() {
        let applied_before = fetch_applied_versions(pool).await?;
        let embedded_max = MIGRATOR
            .migrations
            .iter()
            .map(|m| m.version)
            .max()
            .unwrap_or(0);
        let applied_max = applied_before.iter().copied().max().unwrap_or(0);
        if applied_max > embedded_max {
            return Err(sqlx::Error::Protocol(boot_gate_downgrade_message(
                applied_max,
                embedded_max,
            )));
        }
        let pending: Vec<i64> = MIGRATOR
            .migrations
            .iter()
            .filter(|m| !applied_before.contains(&m.version))
            .map(|m| m.version)
            .collect();
        if !pending.is_empty() {
            let defer_142 = edgequake_storage::any_legacy_rows(pool)
                .await
                .map_err(|e| sqlx::Error::Protocol(format!("legacy census for boot gate: {e}")))?;
            if pending_ok_to_serve(&pending, defer_142) {
                tracing::warn!(
                    target: "edgequake.migration",
                    pending = ?pending,
                    defer_legacy_cutover_assert = defer_142,
                    "OK TO SERVE — SAFE SCHEMA is complete; only optional DROP OLD \
                     and/or deferred SPEC-105 assert (142) remain. They delete or \
                     assert legacy tables after data copy is verified. Do NOT \
                     --confirm-drop while readiness is RED. Preview: edgequake \
                     migrate dry-run. Apply drops when GREEN: edgequake migrate \
                     --confirm-drop (then 142 on next expandable migrate)."
                );
            } else {
                return Err(sqlx::Error::Protocol(boot_gate_pending_message(&pending)));
            }
        }
    }
    run_postgres_migrations(pool).await
}
