//! PostgreSQL migration bootstrap — SPEC-006 / SPEC-017 (SRP).
//!
//! First principle: sqlx records schema versions; **blocking DDL** runs only
//! size-aware in post-hooks (never in sqlx migrate for migration 038).

use std::collections::HashSet;

use sqlx::PgPool;
use tracing::{info, warn};

/// Size-aware index DDL — SSOT: `migrations/support/038/apply.sql`
pub(super) const SQL_038_APPLY: &str =
    include_str!("../../../../../migrations/support/038/apply.sql");

/// Entity backfill — SSOT: `migrations/support/040/apply.sql`
pub(super) const SQL_040_APPLY: &str =
    include_str!("../../../../../migrations/support/040/apply.sql");

/// Document stats columns — SSOT: `migrations/041_document_stats_columns.sql`
pub(super) const SQL_041_APPLY: &str =
    include_str!("../../../../../migrations/041_document_stats_columns.sql");

/// pgvector upgrade + ANN reindex — SSOT: `migrations/support/042/apply.sql`
pub(super) const SQL_042_APPLY: &str =
    include_str!("../../../../../migrations/support/042/apply.sql");

/// Apache AGE extension upgrade — SSOT: `migrations/support/043/apply.sql`
pub(super) const SQL_043_APPLY: &str =
    include_str!("../../../../../migrations/support/043/apply.sql");

/// Community labels marker — SSOT: `migrations/support/044/apply.sql`
pub(super) const SQL_044_APPLY: &str =
    include_str!("../../../../../migrations/support/044/apply.sql");

/// Vector content FTS — SSOT: `migrations/support/045/apply.sql`
pub(super) const SQL_045_APPLY: &str =
    include_str!("../../../../../migrations/support/045/apply.sql");

/// Graph tenant isolation perf indexes — SSOT: `migrations/support/046/apply.sql`
pub(super) const SQL_046_APPLY: &str =
    include_str!("../../../../../migrations/support/046/apply.sql");

/// Workspace document KV index backfill — SSOT: `migrations/support/047/apply.sql`
pub(super) const SQL_047_APPLY: &str =
    include_str!("../../../../../migrations/support/047/apply.sql");

/// Auth user lockout columns — SSOT: `migrations/support/048/apply.sql`
pub(super) const SQL_048_APPLY: &str =
    include_str!("../../../../../migrations/support/048/apply.sql");

/// Membership identity SSOT backfill — SSOT: `migrations/support/049/apply.sql`
pub(super) const SQL_049_APPLY: &str =
    include_str!("../../../../../migrations/support/049/apply.sql");

/// PostgreSQL RLS context verification — SSOT: `migrations/support/050/apply.sql`
pub(super) const SQL_050_APPLY: &str =
    include_str!("../../../../../migrations/support/050/apply.sql");

/// PostgreSQL identity SSOT primary — SSOT: `migrations/support/051/apply.sql`
pub(super) const SQL_051_APPLY: &str =
    include_str!("../../../../../migrations/support/051/apply.sql");

/// PostgreSQL session artifacts SSOT — SSOT: `migrations/support/052/apply.sql`
pub(super) const SQL_052_APPLY: &str =
    include_str!("../../../../../migrations/support/052/apply.sql");

/// PG-only auth reads — SSOT: `migrations/support/053/apply.sql`
pub(super) const SQL_053_APPLY: &str =
    include_str!("../../../../../migrations/support/053/apply.sql");

/// Identity PG RLS envelope — SSOT: `migrations/support/054/apply.sql`
pub(super) const SQL_054_APPLY: &str =
    include_str!("../../../../../migrations/support/054/apply.sql");

/// Auth secure by default — SSOT: `migrations/support/055/apply.sql`
pub(super) const SQL_055_APPLY: &str =
    include_str!("../../../../../migrations/support/055/apply.sql");

/// KV auth consolidated — SSOT: `migrations/support/056/apply.sql`
pub(super) const SQL_056_APPLY: &str =
    include_str!("../../../../../migrations/support/056/apply.sql");

/// KV identity mirror deprecated — SSOT: `migrations/support/057/apply.sql`
pub(super) const SQL_057_APPLY: &str =
    include_str!("../../../../../migrations/support/057/apply.sql");

/// KV mirror ignored when PG pool — SSOT: `migrations/support/058/apply.sql`
pub(super) const SQL_058_APPLY: &str =
    include_str!("../../../../../migrations/support/058/apply.sql");

/// PG-only auth branch SSOT — SSOT: `migrations/support/059/apply.sql`
pub(super) const SQL_059_APPLY: &str =
    include_str!("../../../../../migrations/support/059/apply.sql");

/// OAuth/OIDC honesty + KV quarantine — SSOT: `migrations/support/060/apply.sql`
pub(super) const SQL_060_APPLY: &str =
    include_str!("../../../../../migrations/support/060/apply.sql");

/// Handler isolation from auth_kv_store — SSOT: `migrations/support/061/apply.sql`
pub(super) const SQL_061_APPLY: &str =
    include_str!("../../../../../migrations/support/061/apply.sql");

/// auth/mod identity SSOT — SSOT: `migrations/support/062/apply.sql`
pub(super) const SQL_062_APPLY: &str =
    include_str!("../../../../../migrations/support/062/apply.sql");

/// Service-layer auth SSOT — SSOT: `migrations/support/063/apply.sql`
pub(super) const SQL_063_APPLY: &str =
    include_str!("../../../../../migrations/support/063/apply.sql");

/// Builtin OIDC authorization-code flow — SSOT: `migrations/support/064/apply.sql`
pub(super) const SQL_064_APPLY: &str =
    include_str!("../../../../../migrations/support/064/apply.sql");

/// Auth KV eliminated — SSOT: `migrations/support/065/apply.sql`
pub(super) const SQL_065_APPLY: &str =
    include_str!("../../../../../migrations/support/065/apply.sql");

/// AGE child Node workspace indexes — SSOT: `migrations/support/078/apply.sql`
pub(super) const SQL_078_APPLY: &str =
    include_str!("../../../../../migrations/support/078/apply.sql");

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

/// sqlx migration version marker for graph tenant isolation perf indexes.
pub const MIGRATION_046_VERSION: i64 = 46;

/// sqlx migration version marker for workspace document KV index backfill.
pub const MIGRATION_047_VERSION: i64 = 47;

/// sqlx migration version marker for auth identity SSOT (user lockout columns).
pub const MIGRATION_048_VERSION: i64 = 48;

/// sqlx migration version marker for membership identity SSOT backfill.
pub const MIGRATION_049_VERSION: i64 = 49;

/// sqlx migration version marker for PostgreSQL RLS context SSOT verification.
pub const MIGRATION_050_VERSION: i64 = 50;

/// sqlx migration version marker for PostgreSQL identity SSOT primary.
pub const MIGRATION_051_VERSION: i64 = 51;

/// sqlx migration version marker for PostgreSQL session artifacts SSOT.
pub const MIGRATION_052_VERSION: i64 = 52;

/// sqlx migration version marker for PG-only auth reads (KV not SSOT when pool available).
pub const MIGRATION_053_VERSION: i64 = 53;

/// sqlx migration version marker for identity/session PG RLS envelope.
pub const MIGRATION_054_VERSION: i64 = 54;

/// sqlx migration version marker for auth secure by default (AC-4).
pub const MIGRATION_055_VERSION: i64 = 55;

/// sqlx migration version marker for KV auth consolidation (IMP-026).
pub const MIGRATION_056_VERSION: i64 = 56;

/// sqlx migration version marker for KV identity mirror deprecated.
pub const MIGRATION_057_VERSION: i64 = 57;

/// sqlx migration version marker for KV mirror ignored when PG pool (phase 47).
pub const MIGRATION_058_VERSION: i64 = 58;

/// sqlx migration version marker for PG-only auth branch SSOT (phase 48).
pub const MIGRATION_059_VERSION: i64 = 59;

/// sqlx migration version marker for OAuth/OIDC honesty + KV quarantine (phase 49).
pub const MIGRATION_060_VERSION: i64 = 60;

/// sqlx migration version marker for auth_kv handler isolation (phase 50).
pub const MIGRATION_061_VERSION: i64 = 61;

/// sqlx migration version marker for auth/mod identity SSOT (phase 51).
pub const MIGRATION_062_VERSION: i64 = 62;

/// sqlx migration version marker for auth service-layer SSOT (phase 52).
pub const MIGRATION_063_VERSION: i64 = 63;

/// sqlx migration version marker for builtin OIDC flow (phase 54).
pub const MIGRATION_064_VERSION: i64 = 64;

/// sqlx migration version marker for auth KV eliminated (phase 55).
pub const MIGRATION_065_VERSION: i64 = 65;

/// sqlx migration version for AGE child Node workspace indexes (SPEC-040 / #262).
pub const MIGRATION_078_VERSION: i64 = 78;

/// sqlx migration version for AGE child Node index reconcile (SPEC-041 / #273).
pub const MIGRATION_079_VERSION: i64 = 79;

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
    pub migration_046: Migration046Report,
    pub migration_047: Migration047Report,
    pub migration_048: Migration048Report,
    pub migration_049: Migration049Report,
    pub migration_050: Migration050Report,
    pub migration_051: Migration051Report,
    pub migration_052: Migration052Report,
    pub migration_053: Migration053Report,
    pub migration_054: Migration054Report,
    pub migration_055: Migration055Report,
    pub migration_056: Migration056Report,
    pub migration_057: Migration057Report,
    pub migration_058: Migration058Report,
    pub migration_059: Migration059Report,
    pub migration_060: Migration060Report,
    pub migration_061: Migration061Report,
    pub migration_062: Migration062Report,
    pub migration_063: Migration063Report,
    pub migration_064: Migration064Report,
    pub migration_065: Migration065Report,
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

/// Post-sqlx status for migration 046 graph isolation perf indexes.
#[derive(Debug, Clone)]
pub struct Migration046Report {
    pub marker_present: bool,
    pub apply_executed: bool,
    pub graphs_checked: usize,
    pub missing_indexes: Vec<String>,
}

impl Migration046Report {
    /// Index gaps are logged but do not block traffic (graphs may not exist yet).
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 047 workspace document KV index.
#[derive(Debug, Clone)]
pub struct Migration047Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration047Report {
    /// wsdoc backfill is best-effort — never blocks traffic.
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 048 auth user lockout columns.
#[derive(Debug, Clone)]
pub struct Migration048Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration048Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 049 membership SSOT backfill.
#[derive(Debug, Clone)]
pub struct Migration049Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration049Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 050 RLS context verification.
#[derive(Debug, Clone)]
pub struct Migration050Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration050Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 051 PG identity SSOT primary.
#[derive(Debug, Clone)]
pub struct Migration051Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration051Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 052 PG session artifacts SSOT.
#[derive(Debug, Clone)]
pub struct Migration052Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration052Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 053 PG-only auth reads.
#[derive(Debug, Clone)]
pub struct Migration053Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration053Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 054 identity PG RLS envelope.
#[derive(Debug, Clone)]
pub struct Migration054Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration054Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 055 auth secure by default.
#[derive(Debug, Clone)]
pub struct Migration055Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration055Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 056 KV auth consolidation.
#[derive(Debug, Clone)]
pub struct Migration056Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration056Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 057 KV mirror deprecated.
#[derive(Debug, Clone)]
pub struct Migration057Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration057Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 058 KV mirror ignored with PG pool.
#[derive(Debug, Clone)]
pub struct Migration058Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration058Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 059 PG-only auth branch SSOT.
#[derive(Debug, Clone)]
pub struct Migration059Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration059Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 060 OAuth/OIDC honesty marker.
#[derive(Debug, Clone)]
pub struct Migration060Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration060Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 061 auth_kv handler isolation.
#[derive(Debug, Clone)]
pub struct Migration061Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration061Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 062 auth/mod identity SSOT.
#[derive(Debug, Clone)]
pub struct Migration062Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration062Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 063 auth service-layer SSOT.
#[derive(Debug, Clone)]
pub struct Migration063Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration063Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 064 builtin OIDC flow.
#[derive(Debug, Clone)]
pub struct Migration064Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration064Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 065 auth KV eliminated.
#[derive(Debug, Clone)]
pub struct Migration065Report {
    pub marker_present: bool,
    pub apply_executed: bool,
}

impl Migration065Report {
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
                && !r.migration_046.is_degraded()
                && !r.migration_047.is_degraded()
                && !r.migration_048.is_degraded()
                && !r.migration_049.is_degraded()
                && !r.migration_050.is_degraded()
                && !r.migration_051.is_degraded()
                && !r.migration_052.is_degraded()
                && !r.migration_053.is_degraded()
                && !r.migration_054.is_degraded()
                && !r.migration_055.is_degraded()
                && !r.migration_056.is_degraded()
                && !r.migration_057.is_degraded()
                && !r.migration_058.is_degraded()
                && !r.migration_059.is_degraded()
                && !r.migration_060.is_degraded()
                && !r.migration_061.is_degraded()
                && !r.migration_062.is_degraded()
                && !r.migration_063.is_degraded()
                && !r.migration_064.is_degraded()
                && !r.migration_065.is_degraded()
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

    if reconcile::repair_migration_078_checksum_if_needed(pool).await? {
        info!(
            target: "edgequake.migration",
            step = "migration_078_checksum_repaired",
            "v0.13.2 → v0.13.3 M078 checksum reconciled before sqlx run"
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
            && !migration_065.is_degraded(),
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

    fn noop_migration_046() -> Migration046Report {
        Migration046Report {
            marker_present: true,
            apply_executed: false,
            graphs_checked: 0,
            missing_indexes: vec![],
        }
    }

    fn noop_migration_047() -> Migration047Report {
        Migration047Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_048() -> Migration048Report {
        Migration048Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_049() -> Migration049Report {
        Migration049Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_050() -> Migration050Report {
        Migration050Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_051() -> Migration051Report {
        Migration051Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_052() -> Migration052Report {
        Migration052Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_053() -> Migration053Report {
        Migration053Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_054() -> Migration054Report {
        Migration054Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_055() -> Migration055Report {
        Migration055Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_056() -> Migration056Report {
        Migration056Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_057() -> Migration057Report {
        Migration057Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_058() -> Migration058Report {
        Migration058Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_059() -> Migration059Report {
        Migration059Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_060() -> Migration060Report {
        Migration060Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_061() -> Migration061Report {
        Migration061Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_062() -> Migration062Report {
        Migration062Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_063() -> Migration063Report {
        Migration063Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_064() -> Migration064Report {
        Migration064Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    fn noop_migration_065() -> Migration065Report {
        Migration065Report {
            marker_present: true,
            apply_executed: false,
        }
    }

    #[test]
    fn migration_041_apply_sql_embedded() {
        assert!(SQL_041_APPLY.contains("cost_usd"));
        assert!(SQL_041_APPLY.contains("relationship_count"));
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
            migration_046: noop_migration_046(),
            migration_047: noop_migration_047(),
            migration_048: noop_migration_048(),
            migration_049: noop_migration_049(),
            migration_050: noop_migration_050(),
            migration_051: noop_migration_051(),
            migration_052: noop_migration_052(),
            migration_053: noop_migration_053(),
            migration_054: noop_migration_054(),
            migration_055: noop_migration_055(),
            migration_056: noop_migration_056(),
            migration_057: noop_migration_057(),
            migration_058: noop_migration_058(),
            migration_059: noop_migration_059(),
            migration_060: noop_migration_060(),
            migration_061: noop_migration_061(),
            migration_062: noop_migration_062(),
            migration_063: noop_migration_063(),
            migration_064: noop_migration_064(),
            migration_065: noop_migration_065(),
        })));
    }

    #[test]
    fn migration_046_apply_sql_embedded() {
        assert!(SQL_046_APPLY.contains("tenant_workspace"));
        assert!(SQL_046_APPLY.contains("_ag_edge_start_id"));
        assert!(SQL_046_APPLY.contains("CREATE INDEX IF NOT EXISTS"));
    }

    #[test]
    fn migration_047_apply_sql_embedded() {
        assert!(SQL_047_APPLY.contains("wsdoc:"));
        assert!(SQL_047_APPLY.contains("-metadata"));
        assert!(SQL_047_APPLY.contains("ON CONFLICT"));
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
            migration_046: noop_migration_046(),
            migration_047: noop_migration_047(),
            migration_048: noop_migration_048(),
            migration_049: noop_migration_049(),
            migration_050: noop_migration_050(),
            migration_051: noop_migration_051(),
            migration_052: noop_migration_052(),
            migration_053: noop_migration_053(),
            migration_054: noop_migration_054(),
            migration_055: noop_migration_055(),
            migration_056: noop_migration_056(),
            migration_057: noop_migration_057(),
            migration_058: noop_migration_058(),
            migration_059: noop_migration_059(),
            migration_060: noop_migration_060(),
            migration_061: noop_migration_061(),
            migration_062: noop_migration_062(),
            migration_063: noop_migration_063(),
            migration_064: noop_migration_064(),
            migration_065: noop_migration_065(),
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
            migration_046: noop_migration_046(),
            migration_047: noop_migration_047(),
            migration_048: noop_migration_048(),
            migration_049: noop_migration_049(),
            migration_050: noop_migration_050(),
            migration_051: noop_migration_051(),
            migration_052: noop_migration_052(),
            migration_053: noop_migration_053(),
            migration_054: noop_migration_054(),
            migration_055: noop_migration_055(),
            migration_056: noop_migration_056(),
            migration_057: noop_migration_057(),
            migration_058: noop_migration_058(),
            migration_059: noop_migration_059(),
            migration_060: noop_migration_060(),
            migration_061: noop_migration_061(),
            migration_062: noop_migration_062(),
            migration_063: noop_migration_063(),
            migration_064: noop_migration_064(),
            migration_065: noop_migration_065(),
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
            migration_046: noop_migration_046(),
            migration_047: noop_migration_047(),
            migration_048: noop_migration_048(),
            migration_049: noop_migration_049(),
            migration_050: noop_migration_050(),
            migration_051: noop_migration_051(),
            migration_052: noop_migration_052(),
            migration_053: noop_migration_053(),
            migration_054: noop_migration_054(),
            migration_055: noop_migration_055(),
            migration_056: noop_migration_056(),
            migration_057: noop_migration_057(),
            migration_058: noop_migration_058(),
            migration_059: noop_migration_059(),
            migration_060: noop_migration_060(),
            migration_061: noop_migration_061(),
            migration_062: noop_migration_062(),
            migration_063: noop_migration_063(),
            migration_064: noop_migration_064(),
            migration_065: noop_migration_065(),
        })));
    }
}
