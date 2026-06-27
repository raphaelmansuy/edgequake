//! PostgreSQL migration bootstrap — SPEC-006 / SPEC-017 (SRP).
//!
//! First principle: sqlx records schema versions; **blocking DDL** runs only
//! size-aware in post-hooks (never in sqlx migrate for migration 038).

use std::collections::HashSet;

use sqlx::PgPool;
use tracing::{info, warn};

/// Size-aware index DDL — SSOT: `migrations/support/038/apply.sql`
pub(super) const SQL_038_APPLY: &str = include_str!("../../../../../migrations/support/038/apply.sql");

/// Entity backfill — SSOT: `migrations/support/040/apply.sql`
pub(super) const SQL_040_APPLY: &str = include_str!("../../../../../migrations/support/040/apply.sql");

/// pgvector upgrade + ANN reindex — SSOT: `migrations/support/042/apply.sql`
pub(super) const SQL_042_APPLY: &str = include_str!("../../../../../migrations/support/042/apply.sql");

/// Apache AGE extension upgrade — SSOT: `migrations/support/043/apply.sql`
pub(super) const SQL_043_APPLY: &str = include_str!("../../../../../migrations/support/043/apply.sql");

/// Community labels marker — SSOT: `migrations/support/044/apply.sql`
pub(super) const SQL_044_APPLY: &str = include_str!("../../../../../migrations/support/044/apply.sql");

/// Vector content FTS — SSOT: `migrations/support/045/apply.sql`
pub(super) const SQL_045_APPLY: &str = include_str!("../../../../../migrations/support/045/apply.sql");

/// sqlx migration version marker (no blocking DDL in sqlx file).
pub const MIGRATION_038_VERSION: i64 = 38;

/// sqlx migration version marker for CQRS backfill.
pub const MIGRATION_040_VERSION: i64 = 40;

/// sqlx migration version marker for pgvector upgrade + index rebuild.
pub const MIGRATION_042_VERSION: i64 = 42;

/// sqlx migration version marker for Apache AGE extension upgrade.
pub const MIGRATION_043_VERSION: i64 = 43;

/// sqlx migration version marker for community labels backfill hook.
pub const MIGRATION_044_VERSION: i64 = 44;

/// sqlx migration version marker for vector content native FTS.
pub const MIGRATION_045_VERSION: i64 = 45;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Outcome of bootstrap migration run (surfaced in `/health` and `/ready`).
#[derive(Debug, Clone)]
pub struct MigrationBootstrapReport {
    pub pending_before: usize,
    pub applied_versions: Vec<i64>,
    pub latest_version: Option<i64>,
    pub migration_038: Migration038Report,
    pub migration_042: Migration042Report,
    pub migration_043: Migration043Report,
    pub migration_044: Migration044Report,
    pub migration_045: Migration045Report,
}

/// Post-sqlx status for migration 038 indexes.
#[derive(Debug, Clone)]
pub struct Migration038Report {
    pub age_available: bool,
    pub graphs_checked: usize,
    pub indexes_ready: bool,
    pub indexes_repaired_inline: bool,
    pub deferred_large_graphs: Vec<String>,
    pub missing_indexes: Vec<String>,
    pub operator_action: Option<String>,
}

impl Migration038Report {
    pub fn is_degraded(&self) -> bool {
        !self.indexes_ready && self.age_available
    }
}

/// Post-sqlx status for migration 042 pgvector upgrade + index rebuild.
#[derive(Debug, Clone)]
pub struct Migration042Report {
    pub pgvector_available: bool,
    pub extversion_before: Option<String>,
    pub extversion_after: Option<String>,
    pub shipped_extversion: Option<String>,
    pub iterative_scan_capable: bool,
    pub indexes_rebuilt: bool,
    pub vector_tables_checked: usize,
}

impl Migration042Report {
    pub fn is_degraded(&self) -> bool {
        self.pgvector_available && !self.iterative_scan_capable
    }
}

/// Post-sqlx status for migration 043 AGE extension upgrade.
#[derive(Debug, Clone)]
pub struct Migration043Report {
    pub age_available: bool,
    pub extversion_before: Option<String>,
    pub extversion_after: Option<String>,
    pub extension_updated: bool,
}

impl Migration043Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 044 community labels marker.
#[derive(Debug, Clone)]
pub struct Migration044Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration044Report {
    /// Community backfill is best-effort at graph startup — never blocks traffic.
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 045 vector content FTS.
#[derive(Debug, Clone)]
pub struct Migration045Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration045Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// True when the process may receive traffic (readiness probe).
pub fn is_ready_for_traffic(report: &Option<MigrationBootstrapReport>) -> bool {
    match report {
        None => true,
        Some(r) => {
            !r.migration_038.is_degraded()
                && !r.migration_042.is_degraded()
                && !r.migration_043.is_degraded()
                && !r.migration_044.is_degraded()
                && !r.migration_045.is_degraded()
        }
    }
}

/// Run sqlx migrations plus size-aware 038 apply with structured progression logs.
pub async fn run_postgres_migrations(
    pool: &PgPool,
) -> Result<MigrationBootstrapReport, sqlx::Error> {
    info!(
        target: "edgequake.migration",
        step = "bootstrap_start",
        total_embedded = MIGRATOR.migrations.len(),
        "Database migration bootstrap starting"
    );

    let applied_before = fetch_applied_versions(pool).await?;
    let pending: Vec<_> = MIGRATOR
        .migrations
        .iter()
        .filter(|m| !applied_before.contains(&m.version))
        .collect();

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

    if pending.is_empty() {
        info!(
            target: "edgequake.migration",
            step = "sqlx_run",
            "Schema up to date — no sqlx migrations to apply"
        );
    } else {
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

    let migration_038 = reconcile::reconcile_migration_038(pool, &applied_this_run).await?;
    let migration_042 = reconcile::reconcile_migration_042(pool, &applied_after, &applied_this_run).await?;
    let migration_043 = reconcile::reconcile_migration_043(pool, &applied_after, &applied_this_run).await?;
    let migration_044 = reconcile::reconcile_migration_044(pool, &applied_after, &applied_this_run).await?;
    let migration_045 = reconcile::reconcile_migration_045(pool, &applied_after, &applied_this_run).await?;

    // SPEC-021 P2-02c: Kick off the CQRS entity backfill in the background
    // if migration 040 has been applied but the backfill hasn't completed yet.
    // WHY background: The apply.sql can take minutes on large corpora.
    // Running it at startup would delay the server health check unacceptably.
    let should_backfill = applied_after.contains(&MIGRATION_040_VERSION);
    if should_backfill {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            reconcile::reconcile_migration_040_background(&pool_clone).await;
        });
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

    info!(
        target: "edgequake.migration",
        step = "bootstrap_complete",
        latest_version = applied_after.iter().max().copied(),
        ready_for_traffic = !migration_038.is_degraded()
            && !migration_042.is_degraded()
            && !migration_043.is_degraded()
            && !migration_044.is_degraded()
            && !migration_045.is_degraded(),
        "Database migration bootstrap complete"
    );

    Ok(MigrationBootstrapReport {
        pending_before: pending.len(),
        applied_versions: applied_this_run,
        latest_version: applied_after.iter().max().copied(),
        migration_038,
        migration_042,
        migration_043,
        migration_044,
        migration_045,
    })
}

async fn fetch_applied_versions(pool: &PgPool) -> Result<HashSet<i64>, sqlx::Error> {
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = '_sqlx_migrations'
        )",
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        return Ok(HashSet::new());
    }

    let rows: Vec<i64> = sqlx::query_scalar(
        "SELECT version FROM _sqlx_migrations WHERE success = true ORDER BY version",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

mod helpers;
mod reconcile;

pub use helpers::large_graph_threshold;

#[cfg(test)]
mod tests {
    use super::*;

    fn noop_migration_042() -> Migration042Report {
        Migration042Report {
            pgvector_available: true,
            extversion_before: Some("0.8.0".into()),
            extversion_after: Some("0.8.0".into()),
            shipped_extversion: Some("0.8.3".into()),
            iterative_scan_capable: true,
            indexes_rebuilt: false,
            vector_tables_checked: 0,
        }
    }

    fn noop_migration_043() -> Migration043Report {
        Migration043Report {
            age_available: true,
            extversion_before: Some("1.6.0".into()),
            extversion_after: Some("1.6.0".into()),
            extension_updated: false,
        }
    }

    fn noop_migration_044() -> Migration044Report {
        Migration044Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_045() -> Migration045Report {
        Migration045Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    #[test]
    fn migration_038_apply_sql_embedded() {
        assert!(SQL_038_APPLY.contains("source_ids_gin"));
        assert!(SQL_038_APPLY.contains("CREATE INDEX IF NOT EXISTS"));
        assert!(SQL_038_APPLY.contains("migration_large_graph_threshold"));
        assert!(
            SQL_038_APPLY.contains("::jsonb") && SQL_038_APPLY.contains("jsonb_ops"),
            "GIN indexes must cast agtype to jsonb (json has no GIN opclass)"
        );
    }

    #[test]
    fn degraded_when_indexes_missing_with_age() {
        let report = Migration038Report {
            age_available: true,
            graphs_checked: 1,
            indexes_ready: false,
            indexes_repaired_inline: false,
            deferred_large_graphs: vec!["g (600000 vertices)".into()],
            missing_indexes: vec!["g.idx_g_vertex_source_ids_gin".into()],
            operator_action: Some("apply concurrent".into()),
        };
        assert!(report.is_degraded());
        assert!(!is_ready_for_traffic(&Some(MigrationBootstrapReport {
            pending_before: 0,
            applied_versions: vec![],
            latest_version: Some(38),
            migration_038: report,
            migration_042: noop_migration_042(),
            migration_043: noop_migration_043(),
            migration_044: noop_migration_044(),
            migration_045: noop_migration_045(),
        })));
    }

    #[test]
    fn migration_045_apply_sql_embedded() {
        assert!(SQL_045_APPLY.contains("content_tsv"));
        assert!(SQL_045_APPLY.contains("tsvector"));
    }

    #[test]
    fn migration_044_apply_sql_embedded() {
        assert!(SQL_044_APPLY.contains("community labels"));
    }

    #[test]
    fn migration_043_apply_sql_embedded() {
        assert!(SQL_043_APPLY.contains("ALTER EXTENSION age UPDATE"));
    }

    #[test]
    fn migration_042_apply_sql_embedded() {
        assert!(SQL_042_APPLY.contains("ALTER EXTENSION vector UPDATE"));
        assert!(SQL_042_APPLY.contains("REINDEX INDEX"));
    }

    #[test]
    fn pgvector_iterative_scan_version_gate() {
        assert!(helpers::pgvector_supports_iterative_scan("0.8.0"));
        assert!(!helpers::pgvector_supports_iterative_scan("0.7.4"));
    }

    #[test]
    fn ready_when_pgvector_old_but_not_installed() {
        let report = Migration042Report {
            pgvector_available: false,
            extversion_before: None,
            extversion_after: None,
            shipped_extversion: None,
            iterative_scan_capable: false,
            indexes_rebuilt: false,
            vector_tables_checked: 0,
        };
        assert!(!report.is_degraded());
        assert!(is_ready_for_traffic(&Some(MigrationBootstrapReport {
            pending_before: 0,
            applied_versions: vec![],
            latest_version: Some(42),
            migration_038: Migration038Report {
                age_available: true,
                graphs_checked: 0,
                indexes_ready: true,
                indexes_repaired_inline: false,
                deferred_large_graphs: vec![],
                missing_indexes: vec![],
                operator_action: None,
            },
            migration_042: report,
            migration_043: noop_migration_043(),
            migration_044: noop_migration_044(),
            migration_045: noop_migration_045(),
        })));
    }

    #[test]
    fn degraded_when_pgvector_below_080() {
        let report = Migration042Report {
            pgvector_available: true,
            extversion_before: Some("0.7.4".into()),
            extversion_after: Some("0.7.4".into()),
            shipped_extversion: Some("0.8.3".into()),
            iterative_scan_capable: false,
            indexes_rebuilt: false,
            vector_tables_checked: 1,
        };
        assert!(report.is_degraded());
        assert!(!is_ready_for_traffic(&Some(MigrationBootstrapReport {
            pending_before: 0,
            applied_versions: vec![],
            latest_version: Some(42),
            migration_038: Migration038Report {
                age_available: true,
                graphs_checked: 0,
                indexes_ready: true,
                indexes_repaired_inline: false,
                deferred_large_graphs: vec![],
                missing_indexes: vec![],
                operator_action: None,
            },
            migration_042: report,
            migration_043: noop_migration_043(),
            migration_044: noop_migration_044(),
            migration_045: noop_migration_045(),
        })));
    }

    #[test]
    fn ready_when_no_age_or_indexes_ok() {
        assert!(is_ready_for_traffic(&None));
        let report = Migration038Report {
            age_available: true,
            graphs_checked: 1,
            indexes_ready: true,
            indexes_repaired_inline: false,
            deferred_large_graphs: vec![],
            missing_indexes: vec![],
            operator_action: None,
        };
        assert!(is_ready_for_traffic(&Some(MigrationBootstrapReport {
            pending_before: 0,
            applied_versions: vec![],
            latest_version: Some(38),
            migration_038: report,
            migration_042: noop_migration_042(),
            migration_043: noop_migration_043(),
            migration_044: noop_migration_044(),
            migration_045: noop_migration_045(),
        })));
    }
}
