//! sqlx migration apply + post-hooks (CLI / bootstrap).

use sqlx::PgPool;
use tracing::{info, warn};

use super::ledger::{
    boot_gate_pending_message, fetch_applied_versions, include_in_expandable_apply,
    migrate_cli_mode, pending_ok_to_serve, MIGRATOR,
};
use super::reconcile;
use super::repair::repair_known_production_fossils;
use super::reports::*;
use super::runner;
use super::support_sql::{
    MIGRATION_040_VERSION, MIGRATION_139_VERSION, MIGRATION_140_VERSION, MIGRATION_141_VERSION,
};

/// Run sqlx migrations plus size-aware 038 apply with structured progression logs.
pub async fn run_postgres_migrations(
    pool: &PgPool,
) -> Result<MigrationBootstrapReport, sqlx::Error> {
    run_postgres_migrations_inner(pool, MigrationApplyMode::All).await
}

/// Apply only SAFE SCHEMA (expandable) migrations, skipping irreversible drop
/// versions so later expandables (e.g. 132 behind gated 131) can land without
/// `--confirm-drop`. CLI-only.
pub async fn run_postgres_expandable_migrations(
    pool: &PgPool,
) -> Result<MigrationBootstrapReport, sqlx::Error> {
    if !migrate_cli_mode() {
        return Err(sqlx::Error::Protocol(
            "run_postgres_expandable_migrations is CLI-only (EDGEQUAKE_MIGRATE_CLI=1)".into(),
        ));
    }
    run_postgres_migrations_inner(pool, MigrationApplyMode::ExpandableOnly).await
}

/// Apply embedded migrations **through** `max_version` inclusive (CLI only).
///
/// Used when an irreversible drop is pending without `--confirm-drop`: apply
/// expandable migrations that precede the drop, leave the drop pending.
pub async fn run_postgres_migrations_through(
    pool: &PgPool,
    max_version: i64,
) -> Result<MigrationBootstrapReport, sqlx::Error> {
    if !migrate_cli_mode() {
        return Err(sqlx::Error::Protocol(
            "run_postgres_migrations_through is CLI-only (EDGEQUAKE_MIGRATE_CLI=1)".into(),
        ));
    }
    run_postgres_migrations_inner(pool, MigrationApplyMode::Through(max_version)).await
}

#[derive(Debug, Clone, Copy)]
enum MigrationApplyMode {
    /// Apply every pending migration (confirm-drop / fresh install).
    All,
    /// Apply through `max_version` inclusive (legacy partial train).
    Through(i64),
    /// Apply every pending expandable; omit irreversible drop versions.
    ExpandableOnly,
}

async fn run_postgres_migrations_inner(
    pool: &PgPool,
    mode: MigrationApplyMode,
) -> Result<MigrationBootstrapReport, sqlx::Error> {
    let runner_cfg = runner::RunnerConfig::from_env();
    // SPEC-150: advisory lock only on migrate CLI — serving must not block on it.
    if migrate_cli_mode() {
        let lock_guard = runner::acquire_migrate_run_lock(pool, &runner_cfg).await?;
        let result = run_postgres_migrations_inner_locked(pool, mode, &runner_cfg).await;
        lock_guard.release().await;
        result.map_err(runner::enrich_migrate_error)
    } else {
        run_postgres_migrations_inner_locked(pool, mode, &runner_cfg)
            .await
            .map_err(runner::enrich_migrate_error)
    }
}

async fn run_postgres_migrations_inner_locked(
    pool: &PgPool,
    mode: MigrationApplyMode,
    runner_cfg: &runner::RunnerConfig,
) -> Result<MigrationBootstrapReport, sqlx::Error> {
    let run_id = if migrate_cli_mode() {
        runner::apply_session_timeouts(pool, runner_cfg, "ddl_access_exclusive").await?;
        runner::begin_migration_run(pool, env!("CARGO_PKG_VERSION")).await?
    } else {
        None
    };
    let max_version = match mode {
        MigrationApplyMode::Through(v) => Some(v),
        MigrationApplyMode::All | MigrationApplyMode::ExpandableOnly => None,
    };
    info!(
        target: "edgequake.migration",
        step = "bootstrap_start",
        total_embedded = MIGRATOR.migrations.len(),
        migrate_cli = migrate_cli_mode(),
        mode = ?mode,
        max_version = ?max_version,
        "Database migration bootstrap starting"
    );

    let applied_before = fetch_applied_versions(pool).await?;
    let defer_legacy_cutover_assert = edgequake_storage::any_legacy_rows(pool)
        .await
        .map_err(|e| sqlx::Error::Protocol(format!("legacy census for migrate filter: {e}")))?;
    let pending: Vec<_> = MIGRATOR
        .migrations
        .iter()
        .filter(|m| !applied_before.contains(&m.version))
        .filter(|m| match mode {
            MigrationApplyMode::All => true,
            MigrationApplyMode::Through(cap) => m.version <= cap,
            MigrationApplyMode::ExpandableOnly => {
                include_in_expandable_apply(m.version, defer_legacy_cutover_assert)
            }
        })
        .collect();

    // Defense-in-depth (LAW-B1): serving never applies versioned SQL.
    // Expandable pending ⇒ refuse. Irreversible-only (and deferred 142) pending
    // ⇒ soft-allow (reconcile-only; drop/assert stay operator-gated).
    if !migrate_cli_mode() {
        let all_pending: Vec<i64> = MIGRATOR
            .migrations
            .iter()
            .filter(|m| !applied_before.contains(&m.version))
            .map(|m| m.version)
            .collect();
        if !all_pending.is_empty()
            && !pending_ok_to_serve(&all_pending, defer_legacy_cutover_assert)
        {
            let _ = runner::finish_migration_run(pool, run_id, "refused", 0).await;
            return Err(sqlx::Error::Protocol(boot_gate_pending_message(
                &all_pending,
            )));
        }
    }

    info!(
        target: "edgequake.migration",
        step = "preflight",
        applied = applied_before.len(),
        pending = pending.len(),
        latest_applied = applied_before.iter().max().copied(),
        "Migration preflight complete"
    );

    for (idx, migration) in pending.iter().enumerate() {
        info!(
            target: "edgequake.migration",
            step = "pending",
            progress = format!("{}/{}", idx + 1, pending.len()),
            version = migration.version,
            description = %migration.description,
            "Pending migration queued"
        );
    }

    // SPEC-150 WP-4: fossil checksum rewrite is CLI-only (serving never writes).
    let fossil_repairs = if migrate_cli_mode() {
        repair_known_production_fossils(pool).await?
    } else {
        0
    };
    if fossil_repairs > 0 {
        info!(
            target: "edgequake.migration",
            step = "manifest_fossil_checksum_repaired",
            count = fossil_repairs,
            "Rewrote known production fossil checksums from manifest.toml"
        );
    }

    // Checksum repair modules are CLI-only (SPEC-150 WP-4).
    if migrate_cli_mode() {
        if reconcile::repair_migration_071_checksum_if_needed(pool).await? {
            info!(
                target: "edgequake.migration",
                step = "migration_071_checksum_repaired",
                "v0.13.3 → #275 M071 checksum reconciled before sqlx run"
            );
        }

        if reconcile::repair_migration_078_checksum_if_needed(pool).await? {
            info!(
                target: "edgequake.migration",
                step = "migration_078_checksum_repaired",
                "v0.13.2 → v0.13.3 M078 checksum reconciled before sqlx run"
            );
        }

        if reconcile::repair_migration_118_checksum_if_needed(pool).await? {
            info!(
                target: "edgequake.migration",
                step = "migration_118_checksum_repaired",
                "v0.24.1 → SPEC-110 M118 checksum reconciled before sqlx run"
            );
        }

        if reconcile::repair_migration_121_checksum_if_needed(pool).await? {
            info!(
                target: "edgequake.migration",
                step = "migration_121_checksum_repaired",
                "v0.24.1 → SPEC-110 M121 checksum reconciled before sqlx run"
            );
        }

        if reconcile::repair_migration_125_checksum_if_needed(pool).await? {
            info!(
                target: "edgequake.migration",
                step = "migration_125_checksum_repaired",
                "SPEC-111 M125 cast-direction checksum reconciled before sqlx run"
            );
        }

        if reconcile::repair_migration_131_checksum_if_needed(pool).await? {
            info!(
                target: "edgequake.migration",
                step = "migration_131_checksum_repaired",
                "SPEC-111 M131 provenance-guard checksum reconciled before sqlx run"
            );
        }
    } // end migrate_cli_mode checksum repairs

    let apply_sqlx = migrate_cli_mode() && !pending.is_empty();
    if !apply_sqlx {
        info!(
            target: "edgequake.migration",
            step = "sqlx_run",
            "Schema apply skipped (up to date, serving soft-allow, or empty pending set)"
        );
    } else {
        match mode {
            MigrationApplyMode::All => {
                info!(
                    target: "edgequake.migration",
                    step = "sqlx_run",
                    count = pending.len(),
                    "Applying sqlx migrations (advisory lock held)"
                );
                MIGRATOR.run(pool).await?;
                info!(
                    target: "edgequake.migration",
                    step = "sqlx_complete",
                    count = pending.len(),
                    "sqlx migrations applied successfully"
                );
            }
            MigrationApplyMode::Through(cap) => {
                let filtered: Vec<_> = MIGRATOR
                    .migrations
                    .iter()
                    .filter(|m| m.version <= cap)
                    .cloned()
                    .collect();
                info!(
                    target: "edgequake.migration",
                    step = "sqlx_run",
                    count = pending.len(),
                    max_version = cap,
                    "Applying sqlx migrations through max_version (advisory lock held)"
                );
                let partial = sqlx::migrate::Migrator {
                    migrations: std::borrow::Cow::Owned(filtered),
                    ignore_missing: MIGRATOR.ignore_missing,
                    locking: MIGRATOR.locking,
                    no_tx: MIGRATOR.no_tx,
                };
                partial.run(pool).await?;
                info!(
                    target: "edgequake.migration",
                    step = "sqlx_complete",
                    count = pending.len(),
                    max_version = cap,
                    "sqlx migrations applied successfully (partial train)"
                );
            }
            MigrationApplyMode::ExpandableOnly => {
                // Omit irreversible drop versions so expandables that sit *after*
                // a gated DROP (e.g. 132 behind 131) still apply without confirm.
                // SPEC-105: omit 142 while durable legacy rows remain (LAW-L5).
                let filtered: Vec<_> = MIGRATOR
                    .migrations
                    .iter()
                    .filter(|m| include_in_expandable_apply(m.version, defer_legacy_cutover_assert))
                    .cloned()
                    .collect();
                info!(
                    target: "edgequake.migration",
                    step = "sqlx_run",
                    count = pending.len(),
                    defer_legacy_cutover_assert,
                    "Applying expandable sqlx migrations (irreversible drops omitted; 142 deferred if residue)"
                );
                let partial = sqlx::migrate::Migrator {
                    migrations: std::borrow::Cow::Owned(filtered),
                    ignore_missing: true, // applied drop versions may be absent from this filter
                    locking: MIGRATOR.locking,
                    no_tx: MIGRATOR.no_tx,
                };
                partial.run(pool).await?;
                info!(
                    target: "edgequake.migration",
                    step = "sqlx_complete",
                    count = pending.len(),
                    "expandable sqlx migrations applied successfully"
                );
            }
        }
    }

    let applied_after = fetch_applied_versions(pool).await?;
    let applied_this_run: Vec<i64> = applied_after
        .iter()
        .filter(|v| !applied_before.contains(v))
        .copied()
        .collect();

    for version in &applied_this_run {
        if let Some(m) = MIGRATOR.migrations.iter().find(|m| m.version == *version) {
            info!(
                target: "edgequake.migration",
                step = "applied",
                version = m.version,
                description = %m.description,
                "Migration applied in this bootstrap"
            );
        }
    }

    if reconcile::reconcile_migration_041(pool).await? {
        info!(
            target: "edgequake.migration",
            step = "migration_041_ok",
            "Migration 041 document stats columns reconciled"
        );
    }

    if reconcile::reconcile_migration_078(pool, &applied_after).await? {
        info!(
            target: "edgequake.migration",
            step = "migration_078_ok",
            "Migration 078/079 child Node indexes reconciled"
        );
    }

    // M083: every boot — graphs created after sqlx migrate still need UNIQUE indexes.
    if reconcile::reconcile_migration_083(pool).await? {
        info!(
            target: "edgequake.migration",
            step = "migration_083_ok",
            "Migration 083 native UNIQUE indexes + stats backfill reconciled"
        );
    }

    // M086 / SPEC-070: every boot — EDGE BFS indexes for incident-edge / degrees.
    if reconcile::reconcile_migration_086(pool).await? {
        info!(
            target: "edgequake.migration",
            step = "migration_086_ok",
            "Migration 086 EDGE BFS indexes reconciled (DDL boot-owned)"
        );
    }

    // M092 / SPEC-069 / SPEC-083: every boot — eq_* columns/indexes/triggers off the delete hot path.
    let migration_092 = reconcile::reconcile_migration_092(pool).await?;
    if migration_092.apply_executed && !migration_092.is_degraded() {
        info!(
            target: "edgequake.migration",
            step = "migration_092_ok",
            graphs_ready = migration_092.graphs_ready,
            graphs_checked = migration_092.graphs_checked,
            "Migration 092 eq_* denorm schema reconciled (DDL boot-owned)"
        );
    }

    let migration_080_applied =
        reconcile::reconcile_migration_080(pool, &applied_after, &applied_this_run).await?;
    if migration_080_applied {
        info!(
            target: "edgequake.migration",
            step = "migration_080_ok",
            operator_action = "verify_embeddings_after_halfvec_conversion",
            "Migration 080 halfvec conversion reconciled — vector registry cache should be cleared"
        );
    }

    let migration_081_applied =
        reconcile::reconcile_migration_081(pool, &applied_after, &applied_this_run).await?;
    if migration_081_applied {
        info!(
            target: "edgequake.migration",
            step = "migration_081_ok",
            "Migration 081 AGE graph RLS reconciled"
        );
    }

    let migration_038 = reconcile::reconcile_migration_038(pool, &applied_this_run).await?;
    let migration_042 =
        reconcile::reconcile_migration_042(pool, &applied_after, &applied_this_run).await?;
    let migration_043 =
        reconcile::reconcile_migration_043(pool, &applied_after, &applied_this_run).await?;
    let migration_044 =
        reconcile::reconcile_migration_044(pool, &applied_after, &applied_this_run).await?;
    let migration_045 =
        reconcile::reconcile_migration_045(pool, &applied_after, &applied_this_run).await?;
    let migration_046 =
        reconcile::reconcile_migration_046(pool, &applied_after, &applied_this_run).await?;
    let migration_047 =
        reconcile::reconcile_migration_047(pool, &applied_after, &applied_this_run).await?;
    let migration_048 =
        reconcile::reconcile_migration_048(pool, &applied_after, &applied_this_run).await?;
    let migration_049 =
        reconcile::reconcile_migration_049(pool, &applied_after, &applied_this_run).await?;
    let migration_050 =
        reconcile::reconcile_migration_050(pool, &applied_after, &applied_this_run).await?;
    let migration_051 =
        reconcile::reconcile_migration_051(pool, &applied_after, &applied_this_run).await?;
    let migration_052 =
        reconcile::reconcile_migration_052(pool, &applied_after, &applied_this_run).await?;
    let migration_053 =
        reconcile::reconcile_migration_053(pool, &applied_after, &applied_this_run).await?;
    let migration_054 =
        reconcile::reconcile_migration_054(pool, &applied_after, &applied_this_run).await?;
    let migration_055 =
        reconcile::reconcile_migration_055(pool, &applied_after, &applied_this_run).await?;
    let migration_056 =
        reconcile::reconcile_migration_056(pool, &applied_after, &applied_this_run).await?;
    let migration_057 =
        reconcile::reconcile_migration_057(pool, &applied_after, &applied_this_run).await?;
    let migration_058 =
        reconcile::reconcile_migration_058(pool, &applied_after, &applied_this_run).await?;
    let migration_059 =
        reconcile::reconcile_migration_059(pool, &applied_after, &applied_this_run).await?;
    let migration_060 =
        reconcile::reconcile_migration_060(pool, &applied_after, &applied_this_run).await?;
    let migration_061 =
        reconcile::reconcile_migration_061(pool, &applied_after, &applied_this_run).await?;
    let migration_062 =
        reconcile::reconcile_migration_062(pool, &applied_after, &applied_this_run).await?;
    let migration_063 =
        reconcile::reconcile_migration_063(pool, &applied_after, &applied_this_run).await?;
    let migration_064 =
        reconcile::reconcile_migration_064(pool, &applied_after, &applied_this_run).await?;
    let migration_065 =
        reconcile::reconcile_migration_065(pool, &applied_after, &applied_this_run).await?;

    // SPEC-021 P2-02c / SPEC-098: support reconcile DDL.
    // SPEC-150 WP-4: serving never writes — run only on migrate CLI, or when
    // the one-release escape hatch EDGEQUAKE_SERVE_RECONCILE=1 is set.
    let serve_reconcile = matches!(
        std::env::var("EDGEQUAKE_SERVE_RECONCILE").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("on") | Ok("yes")
    );
    if migrate_cli_mode() || serve_reconcile {
        let should_backfill = applied_after.contains(&MIGRATION_040_VERSION);
        if should_backfill {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                reconcile::reconcile_migration_040_background(&pool_clone).await;
            });
        }

        if applied_after.contains(&MIGRATION_139_VERSION) {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                reconcile::reconcile_migration_139_background(&pool_clone).await;
            });
        }

        if applied_after.contains(&MIGRATION_140_VERSION) {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                reconcile::reconcile_migration_140_background(&pool_clone).await;
            });
        }

        if applied_after.contains(&MIGRATION_141_VERSION) {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                reconcile::reconcile_migration_141_background(&pool_clone).await;
            });
        }
    }

    if migration_038.is_degraded() {
        warn!(
            target: "edgequake.migration",
            step = "migration_038_degraded",
            missing = ?migration_038.missing_indexes,
            deferred = ?migration_038.deferred_large_graphs,
            action = migration_038.operator_action.as_deref().unwrap_or("none"),
            "Migration 038 indexes incomplete — /ready will fail until ops completes CONCURRENTLY apply"
        );
    } else if migration_038.indexes_repaired_inline {
        info!(
            target: "edgequake.migration",
            step = "migration_038_repaired",
            graphs = migration_038.graphs_checked,
            "Migration 038 indexes verified/repaired at bootstrap"
        );
    } else {
        info!(
            target: "edgequake.migration",
            step = "migration_038_ok",
            graphs = migration_038.graphs_checked,
            "Migration 038 indexes verified"
        );
    }

    if migration_042.pgvector_available {
        info!(
            target: "edgequake.migration",
            step = "migration_042_ok",
            extversion = ?migration_042.extversion_after,
            iterative_scan = migration_042.iterative_scan_capable,
            indexes_rebuilt = migration_042.indexes_rebuilt,
            tables = migration_042.vector_tables_checked,
            "Migration 042 pgvector upgrade/index rebuild complete"
        );
        if migration_042.is_degraded() {
            warn!(
                target: "edgequake.migration",
                step = "migration_042_degraded",
                extversion = ?migration_042.extversion_after,
                shipped = ?migration_042.shipped_extversion,
                "pgvector catalog is below 0.8 — /ready returns 503 until upgraded; rebuild postgres: make db-start (or docker compose up -d --build --force-recreate postgres) then restart backend"
            );
        }
    }

    if migration_043.age_available {
        info!(
            target: "edgequake.migration",
            step = "migration_043_ok",
            extversion = ?migration_043.extversion_after,
            updated = migration_043.extension_updated,
            "Migration 043 AGE extension upgrade complete"
        );
    }

    if migration_044.marker_present {
        info!(
            target: "edgequake.migration",
            step = "migration_044_ok",
            apply_executed = migration_044.apply_executed,
            "Migration 044 community labels marker recorded (backfill at graph startup)"
        );
    }

    if migration_045.marker_present {
        info!(
            target: "edgequake.migration",
            step = "migration_045_ok",
            apply_executed = migration_045.apply_executed,
            "Migration 045 vector content_tsv FTS indexes ready"
        );
    }

    if migration_046.marker_present || migration_046.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_046_ok",
            graphs = migration_046.graphs_checked,
            missing = migration_046.missing_indexes.len(),
            apply_executed = migration_046.apply_executed,
            "Migration 046 graph isolation perf indexes verified"
        );
    }

    if migration_047.marker_present || migration_047.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_047_ok",
            apply_executed = migration_047.apply_executed,
            "Migration 047 workspace document KV index backfill complete"
        );
    }

    if migration_048.marker_present || migration_048.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_048_ok",
            apply_executed = migration_048.apply_executed,
            "Migration 048 auth user lockout columns ready"
        );
    }

    if migration_049.marker_present || migration_049.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_049_ok",
            apply_executed = migration_049.apply_executed,
            "Migration 049 membership SSOT backfill complete"
        );
    }

    if migration_050.marker_present || migration_050.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_050_ok",
            apply_executed = migration_050.apply_executed,
            "Migration 050 PostgreSQL RLS context functions verified"
        );
    }

    if migration_051.marker_present || migration_051.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_051_ok",
            apply_executed = migration_051.apply_executed,
            "Migration 051 PG identity SSOT primary verified"
        );
    }

    if migration_052.marker_present || migration_052.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_052_ok",
            apply_executed = migration_052.apply_executed,
            "Migration 052 PG session artifacts SSOT verified"
        );
    }

    if migration_053.marker_present || migration_053.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_053_ok",
            apply_executed = migration_053.apply_executed,
            "Migration 053 PG-only auth reads verified"
        );
    }

    if migration_054.marker_present || migration_054.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_054_ok",
            apply_executed = migration_054.apply_executed,
            "Migration 054 identity PG RLS envelope verified"
        );
    }

    if migration_055.marker_present || migration_055.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_055_ok",
            apply_executed = migration_055.apply_executed,
            "Migration 055 auth secure-by-default marker verified"
        );
    }

    if migration_056.marker_present || migration_056.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_056_ok",
            apply_executed = migration_056.apply_executed,
            "Migration 056 KV auth consolidation marker verified"
        );
    }

    if migration_057.marker_present || migration_057.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_057_ok",
            apply_executed = migration_057.apply_executed,
            "Migration 057 KV identity mirror deprecated marker verified"
        );
    }

    if migration_058.marker_present || migration_058.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_058_ok",
            apply_executed = migration_058.apply_executed,
            "Migration 058 KV mirror ignored-with-pool marker verified"
        );
    }

    if migration_059.marker_present || migration_059.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_059_ok",
            apply_executed = migration_059.apply_executed,
            "Migration 059 PG-only auth branch SSOT verified"
        );
    }

    if migration_060.marker_present || migration_060.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_060_ok",
            apply_executed = migration_060.apply_executed,
            "Migration 060 OAuth/OIDC honesty marker verified"
        );
    }

    if migration_061.marker_present || migration_061.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_061_ok",
            apply_executed = migration_061.apply_executed,
            "Migration 061 auth_kv handler isolation verified"
        );
    }

    if migration_062.marker_present || migration_062.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_062_ok",
            apply_executed = migration_062.apply_executed,
            "Migration 062 auth/mod identity SSOT verified"
        );
    }

    if migration_063.marker_present || migration_063.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_063_ok",
            apply_executed = migration_063.apply_executed,
            "Migration 063 auth service-layer SSOT verified"
        );
    }

    if migration_064.marker_present || migration_064.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_064_ok",
            apply_executed = migration_064.apply_executed,
            "Migration 064 builtin OIDC marker verified"
        );
    }

    if migration_065.marker_present || migration_065.apply_executed {
        info!(
            target: "edgequake.migration",
            step = "migration_065_ok",
            apply_executed = migration_065.apply_executed,
            "Migration 065 auth KV eliminated marker verified"
        );
    }

    // SPEC-083 D-45: ensure next-month audit partition exists so inserts past the
    // initial window do not fail. Function is SSOT in 001_init_database.sql.
    match sqlx::query_scalar::<_, String>("SELECT create_next_audit_log_partition()")
        .fetch_optional(pool)
        .await
    {
        Ok(Some(msg)) => {
            info!(
                target: "edgequake.migration",
                step = "audit_next_month_partition",
                result = %msg,
                "Ensured next-month audit_logs partition"
            );
        }
        Ok(None) => {}
        Err(e) => {
            // Non-fatal: older DBs may lack the function until 001/012 applied.
            tracing::warn!(
                target: "edgequake.migration",
                step = "audit_next_month_partition",
                error = %e,
                "Could not ensure next-month audit partition (will retry next boot)"
            );
        }
    }

    info!(
        target: "edgequake.migration",
        step = "bootstrap_complete",
        latest_version = applied_after.iter().max().copied(),
        ready_for_traffic = !migration_038.is_degraded()
            && !migration_042.is_degraded()
            && !migration_043.is_degraded()
            && !migration_044.is_degraded()
            && !migration_045.is_degraded()
            && !migration_046.is_degraded()
            && !migration_047.is_degraded()
            && !migration_048.is_degraded()
            && !migration_049.is_degraded()
            && !migration_050.is_degraded()
            && !migration_051.is_degraded()
            && !migration_052.is_degraded()
            && !migration_053.is_degraded()
            && !migration_054.is_degraded()
            && !migration_055.is_degraded()
            && !migration_056.is_degraded()
            && !migration_057.is_degraded()
            && !migration_058.is_degraded()
            && !migration_059.is_degraded()
            && !migration_060.is_degraded()
            && !migration_061.is_degraded()
            && !migration_062.is_degraded()
            && !migration_063.is_degraded()
            && !migration_064.is_degraded()
            && !migration_065.is_degraded()
            && !migration_092.is_degraded(),
        "Database migration bootstrap complete"
    );

    let report = MigrationBootstrapReport {
        pending_before: pending.len(),
        applied_versions: applied_this_run,
        latest_version: applied_after.iter().max().copied(),
        migration_038,
        migration_042,
        migration_043,
        migration_044,
        migration_045,
        migration_046,
        migration_047,
        migration_048,
        migration_049,
        migration_050,
        migration_051,
        migration_052,
        migration_053,
        migration_054,
        migration_055,
        migration_056,
        migration_057,
        migration_058,
        migration_059,
        migration_060,
        migration_061,
        migration_062,
        migration_063,
        migration_064,
        migration_065,
        migration_080: Migration080Report {
            halfvec_conversion_applied: migration_080_applied,
            apply_executed: migration_080_applied,
        },
        migration_081: Migration081Report {
            age_rls_applied: migration_081_applied,
            apply_executed: migration_081_applied,
            skipped_age_version: false,
        },
        migration_092,
    };
    let applied_count = report.applied_versions.len() as i32;
    let _ = runner::finish_migration_run(pool, run_id, "success", applied_count).await;
    if let Some(max_v) = report.latest_version {
        let floor = edgequake_migrate_manifest::load()
            .migration
            .iter()
            .filter(|e| e.version <= max_v)
            .filter_map(|e| e.serving_floor)
            .max()
            .unwrap_or_else(|| max_v.saturating_sub(9));
        let _ = runner::bump_schema_compat(pool, floor).await;
    }
    Ok(report)
}
