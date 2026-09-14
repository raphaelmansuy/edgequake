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

/// SPEC-098 entity spine ensure — SSOT: `migrations/support/139/apply.sql`
pub(super) const SQL_139_APPLY: &str =
    include_str!("../../../../../migrations/support/139/apply.sql");

/// SPEC-098 edge arbiter + relationship spine — SSOT: `migrations/support/140/apply.sql`
pub(super) const SQL_140_APPLY: &str =
    include_str!("../../../../../migrations/support/140/apply.sql");

/// SPEC-098 document lifecycle status CHECK — SSOT: `migrations/support/141/apply.sql`
pub(super) const SQL_141_APPLY: &str =
    include_str!("../../../../../migrations/support/141/apply.sql");

/// sqlx migration version marker for SPEC-098 spine ensure.
pub const MIGRATION_139_VERSION: i64 = 139;

/// sqlx migration version marker for SPEC-098 edge arbiter reconcile.
pub const MIGRATION_140_VERSION: i64 = 140;

/// sqlx migration version for SPEC-098 document lifecycle statuses.
pub const MIGRATION_141_VERSION: i64 = 141;

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

/// halfvec embedding conversion — SSOT: `migrations/support/080/apply.sql`
pub(super) const SQL_080_APPLY: &str =
    include_str!("../../../../../migrations/support/080/apply.sql");

/// AGE graph RLS policies — SSOT: `migrations/support/081/apply.sql`
pub(super) const SQL_081_APPLY: &str =
    include_str!("../../../../../migrations/support/081/apply.sql");

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

/// sqlx migration version for HNSW ef_construction optimization (SPEC-034 IMP-04).
pub const MIGRATION_071_VERSION: i64 = 71;

/// sqlx migration version for SPEC-091 wsdoc backfill (SPEC-110 checksum repair).
pub const MIGRATION_118_VERSION: i64 = 118;

/// sqlx migration version for SPEC-091 injection backfill (SPEC-110 checksum repair).
pub const MIGRATION_121_VERSION: i64 = 121;
/// sqlx migration version for SPEC-091 KV drop (SPEC-111 cast-direction checksum repair).
pub const MIGRATION_125_VERSION: i64 = 125;
/// sqlx migration version for SPEC-091 fleet vector drop (SPEC-111 provenance checksum repair).
pub const MIGRATION_131_VERSION: i64 = 131;

/// sqlx migration version for halfvec embeddings (SPEC-042-E E-01).
pub const MIGRATION_080_VERSION: i64 = 80;

/// sqlx migration version for AGE graph RLS (SPEC-042-E E-02).
pub const MIGRATION_081_VERSION: i64 = 81;

/// sqlx migration version for native UNIQUE index reconcile (all AGE graphs).
pub const MIGRATION_083_VERSION: i64 = 83;

/// Native UNIQUE index reconcile — SSOT: `migrations/support/083/apply.sql`
pub const SQL_083_APPLY: &str = include_str!("../../../../../migrations/support/083/apply.sql");

/// JSONB→column stats backfill — SSOT: `migrations/support/083/stats_backfill.sql`
pub const SQL_083_STATS_BACKFILL: &str =
    include_str!("../../../../../migrations/support/083/stats_backfill.sql");

/// sqlx migration version for EDGE BFS index reconcile (SPEC-053 / SPEC-070).
pub const MIGRATION_086_VERSION: i64 = 86;

/// EDGE BFS index reconcile — SSOT: `migrations/support/086/apply.sql`
pub const SQL_086_APPLY: &str = include_str!("../../../../../migrations/support/086/apply.sql");

/// sqlx migration version for eq_* denorm marker (SPEC-062 / SPEC-069).
pub const MIGRATION_092_VERSION: i64 = 92;

/// eq_* denorm reconcile — SSOT: `migrations/support/092/apply.sql`
pub const SQL_092_APPLY: &str = include_str!("../../../../../migrations/support/092/apply.sql");

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
    pub migration_080: Migration080Report,
    pub migration_081: Migration081Report,
    /// SPEC-083 / P0: eq_* denorm readiness after every-boot M092 reconcile.
    pub migration_092: Migration092Report,
}

/// Post-reconcile status for migration 092 eq_* denorm (SPEC-083).
#[derive(Debug, Clone)]
pub struct Migration092Report {
    pub age_available: bool,
    pub apply_executed: bool,
    pub graphs_checked: usize,
    pub graphs_ready: usize,
    pub graphs_degraded: Vec<String>,
    /// When true, traffic may proceed with property-path SQL fallback.
    pub fallback_env_enabled: bool,
}

impl Migration092Report {
    /// Degraded when AGE graphs exist but eq_* columns are still missing,
    /// unless operator opted into `EDGEQUAKE_EQ_ID_FALLBACK=1`.
    pub fn is_degraded(&self) -> bool {
        self.age_available && !self.graphs_degraded.is_empty() && !self.fallback_env_enabled
    }
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
    /// SPEC-046 OPS-P0.3: vector tables missing HNSW/IVFFlat index.
    pub missing_ann_index_tables: usize,
}

impl Migration042Report {
    pub fn is_degraded(&self) -> bool {
        self.pgvector_available
            && (!self.iterative_scan_capable || self.missing_ann_index_tables > 0)
    }
}

/// Post-sqlx status for migration 043 AGE extension upgrade.
#[derive(Debug, Clone)]
pub struct Migration043Report {
    pub age_available: bool,
    pub extversion_before: Option<String>,
    pub extversion_after: Option<String>,
    pub shipped_extversion: Option<String>,
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

/// Post-sqlx status for migration 080 halfvec conversion (SPEC-042-E).
#[derive(Debug, Clone)]
pub struct Migration080Report {
    pub halfvec_conversion_applied: bool,
    pub apply_executed: bool,
}

impl Migration080Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Post-sqlx status for migration 081 AGE graph RLS (SPEC-042-E).
#[derive(Debug, Clone)]
pub struct Migration081Report {
    pub age_rls_applied: bool,
    pub apply_executed: bool,
    pub skipped_age_version: bool,
}

impl Migration081Report {
    pub fn is_degraded(&self) -> bool {
        false
    }
}

/// Collect readiness blocker IDs for `/ready` JSON (SPEC-045 SRE-M03 / SPEC-046 OPS-P1.13).
///
/// SSOT: every migration report that participates in [`is_ready_for_traffic`]
/// must appear here so probe JSON and boolean readiness cannot diverge.
pub fn readiness_blockers(report: &Option<MigrationBootstrapReport>) -> Vec<String> {
    let Some(r) = report else {
        return Vec::new();
    };
    let mut blockers = Vec::new();

    // 038: AGE indexes
    if r.migration_038.is_degraded() {
        blockers.push("migration_038".to_string());
    }

    // 042: pgvector iterative_scan + HNSW presence (split for operator action)
    if r.migration_042.is_degraded() {
        if r.migration_042.pgvector_available && !r.migration_042.iterative_scan_capable {
            blockers.push("migration_042".to_string());
        }
        if r.migration_042.missing_ann_index_tables > 0 {
            blockers.push("missing_hnsw_index".to_string());
        }
    }
    // CVE-2026-3172: pgvector 0.8.0/0.8.1 unsafe for parallel HNSW builds.
    if r.migration_042.pgvector_available {
        if let Some(ver) = r.migration_042.extversion_after.as_deref() {
            if !helpers::pgvector_meets_cve_floor(ver) {
                blockers.push("pgvector_cve_floor".to_string());
            }
        }
    }

    // Remaining migrations: emit when degraded (most currently never degrade).
    macro_rules! push_if_degraded {
        ($field:ident, $id:expr) => {
            if r.$field.is_degraded() {
                blockers.push($id.to_string());
            }
        };
    }
    push_if_degraded!(migration_043, "migration_043");
    push_if_degraded!(migration_044, "migration_044");
    push_if_degraded!(migration_045, "migration_045");
    push_if_degraded!(migration_046, "migration_046");
    push_if_degraded!(migration_047, "migration_047");
    push_if_degraded!(migration_048, "migration_048");
    push_if_degraded!(migration_049, "migration_049");
    push_if_degraded!(migration_050, "migration_050");
    push_if_degraded!(migration_051, "migration_051");
    push_if_degraded!(migration_052, "migration_052");
    push_if_degraded!(migration_053, "migration_053");
    push_if_degraded!(migration_054, "migration_054");
    push_if_degraded!(migration_055, "migration_055");
    push_if_degraded!(migration_056, "migration_056");
    push_if_degraded!(migration_057, "migration_057");
    push_if_degraded!(migration_058, "migration_058");
    push_if_degraded!(migration_059, "migration_059");
    push_if_degraded!(migration_060, "migration_060");
    push_if_degraded!(migration_061, "migration_061");
    push_if_degraded!(migration_062, "migration_062");
    push_if_degraded!(migration_063, "migration_063");
    push_if_degraded!(migration_064, "migration_064");
    push_if_degraded!(migration_065, "migration_065");
    push_if_degraded!(migration_080, "migration_080");
    push_if_degraded!(migration_081, "migration_081");
    if r.migration_092.is_degraded() {
        blockers.push("eq_id_schema".to_string());
        blockers.push("migration_092".to_string());
    }

    blockers
}

/// Operator-facing remediation hint for the first readiness blocker.
pub fn readiness_operator_action(report: &Option<MigrationBootstrapReport>) -> Option<String> {
    let r = report.as_ref()?;
    if r.migration_092.is_degraded() {
        return Some(format!(
            "eq_* AGE columns missing on graphs {:?}; run maintenance DDL (docs/083-improvements/INCIDENT-PROD-DIAGNOSIS.md) or set EDGEQUAKE_EQ_ID_FALLBACK=1 for property-path SQL",
            r.migration_092.graphs_degraded
        ));
    }
    if r.migration_038.is_degraded() {
        return r
            .migration_038
            .operator_action
            .clone()
            .or_else(|| Some("apply_038.sh --concurrent for large graphs".to_string()));
    }
    if r.migration_042.pgvector_available {
        if let Some(ver) = r.migration_042.extversion_after.as_deref() {
            if !helpers::pgvector_meets_cve_floor(ver) {
                return Some(
                    "Upgrade pgvector to >= 0.8.2 (prefer 0.8.5; CVE-2026-3172) then restart backend (make db-start)"
                        .to_string(),
                );
            }
        }
    }
    if r.migration_042.is_degraded() {
        if r.migration_042.missing_ann_index_tables > 0 {
            return Some(format!(
                "ANN index missing on {} vector table(s); restart backend to recreate HNSW or run CREATE INDEX",
                r.migration_042.missing_ann_index_tables
            ));
        }
        return Some(
            "Upgrade pgvector to >= 0.8.2 and restart backend (make db-start)".to_string(),
        );
    }
    None
}

/// True when the process may receive traffic (readiness probe).
///
/// SPEC-046 OPS-P1.13: derived from [`readiness_blockers`] (single source of truth).
pub fn is_ready_for_traffic(report: &Option<MigrationBootstrapReport>) -> bool {
    readiness_blockers(report).is_empty()
}

/// SPEC-091 Doc 17 (LD-15, LAW-B2): distinct exit code for boot-gate refusals
/// (EX_CONFIG) so orchestrators can branch on "migrate required" vs "crash".
pub const BOOT_GATE_EXIT_CODE: i32 = 78;

/// Sentinel prefix so the binary can map refusal errors to [`BOOT_GATE_EXIT_CODE`].
pub const BOOT_GATE_REFUSAL_PREFIX: &str = "BOOT_GATE_REFUSAL:";

/// SPEC-091 irreversible drop versions (LD-07) — human-gated behind `--confirm-drop`.
pub const IRREVERSIBLE_DROP_VERSIONS: &[i64] = &[125, 126, 131];

/// SPEC-105 LAW-L5 — post-drop empty-residue assert (expandable, but deferred
/// while durable `eq_*` rows remain so ≤0.22 mid-upgrade `migrate` / boot stay unblocked).
pub const LEGACY_CUTOVER_ASSERT_VERSION: i64 = 142;

/// True when `version` is an irreversible SPEC-091 drop migration.
pub fn is_irreversible_drop(version: i64) -> bool {
    IRREVERSIBLE_DROP_VERSIONS.contains(&version)
}

/// True when `version` is the SPEC-105 legacy cutover assert migration.
pub fn is_legacy_cutover_assert(version: i64) -> bool {
    version == LEGACY_CUTOVER_ASSERT_VERSION
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
    /// Highest successful `_sqlx_migrations.version` (0 when the ledger is empty).
    pub applied_max: i64,
    /// Highest version this binary embeds (0 when the migrator is empty).
    pub embedded_max: i64,
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
        applied_max,
        embedded_max,
    })
}

/// True when the database has never been migrated (zero successful
/// `_sqlx_migrations` rows). Used by the CLI to scope irreversible-op consent:
/// on a fresh install the drop migrations cannot destroy anything (LAW-C5 —
/// consent is required only when there is something to lose).
pub async fn is_fresh_database(pool: &PgPool) -> Result<bool, sqlx::Error> {
    Ok(fetch_applied_versions(pool).await?.is_empty())
}

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

    // SPEC-098: ensure relational spine for typed fleet FK resolve (AGE → entities).
    if applied_after.contains(&MIGRATION_139_VERSION) {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            reconcile::reconcile_migration_139_background(&pool_clone).await;
        });
    }

    // SPEC-098 W6: single EDGE arbiter + AGE → relationships spine.
    if applied_after.contains(&MIGRATION_140_VERSION) {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            reconcile::reconcile_migration_140_background(&pool_clone).await;
        });
    }

    // SPEC-098 W9: documents_valid_status includes deleting / delete_failed.
    if applied_after.contains(&MIGRATION_141_VERSION) {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            reconcile::reconcile_migration_141_background(&pool_clone).await;
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
    })
}

async fn fetch_applied_versions(pool: &PgPool) -> Result<HashSet<i64>, sqlx::Error> {
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

mod checksum_repair;
mod helpers;
mod reconcile;
mod reconcile_state;

pub use checksum_repair::{
    allow_checksum_repair, parse_allow_checksum_repair_list, refuse_silent_repair_message,
    ALLOW_CHECKSUM_REPAIR_ENV, KNOWN_CHECKSUM_REPAIR_VERSIONS,
};

pub use helpers::large_graph_threshold;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irreversible_drop_helpers_and_expandable_target() {
        assert!(is_irreversible_drop(125));
        assert!(is_irreversible_drop(126));
        assert!(is_irreversible_drop(131));
        assert!(!is_irreversible_drop(130));
        assert!(pending_only_irreversible_drops(&[131]));
        assert!(pending_only_irreversible_drops(&[125, 126, 131]));
        assert!(!pending_only_irreversible_drops(&[130, 131]));
        assert!(!pending_only_irreversible_drops(&[]));

        let pending = vec![
            (128, "listing".into()),
            (129, "hnsw".into()),
            (130, "fleet".into()),
            (131, "drop".into()),
        ];
        assert_eq!(max_expandable_target(&pending), Some(130));

        let behind_drop = vec![(131, "drop".into()), (132, "halfvec".into())];
        assert_eq!(max_expandable_target(&behind_drop), None);
        assert_eq!(pending_expandable_versions(&behind_drop), vec![132]);

        let blocked_at_125 = vec![
            (125, "kv".into()),
            (126, "vec".into()),
            (128, "listing".into()),
        ];
        assert_eq!(max_expandable_target(&blocked_at_125), None);
        assert_eq!(pending_expandable_versions(&blocked_at_125), vec![128]);
    }

    #[test]
    fn e2e_105_07_defer_142_while_legacy_residue() {
        assert!(is_legacy_cutover_assert(LEGACY_CUTOVER_ASSERT_VERSION));
        assert!(!is_irreversible_drop(LEGACY_CUTOVER_ASSERT_VERSION));

        // Mid-upgrade: 131 + 142 with residue → OK to serve / soft-exit.
        assert!(pending_ok_to_serve(&[131, 142], true));
        // Without residue, 142 is hard expandable — must apply before serve.
        assert!(!pending_ok_to_serve(&[131, 142], false));
        assert!(pending_ok_to_serve(&[131], false));

        assert_eq!(expandable_apply_versions(&[131, 132, 142], true), vec![132]);
        assert_eq!(
            expandable_apply_versions(&[131, 132, 142], false),
            vec![132, 142]
        );
    }

    fn noop_migration_042() -> Migration042Report {
        Migration042Report {
            pgvector_available: true,
            extversion_before: Some("0.8.5".into()),
            extversion_after: Some("0.8.5".into()),
            shipped_extversion: Some("0.8.5".into()),
            iterative_scan_capable: true,
            indexes_rebuilt: false,
            vector_tables_checked: 0,
            missing_ann_index_tables: 0,
        }
    }

    fn noop_migration_043() -> Migration043Report {
        Migration043Report {
            age_available: true,
            extversion_before: Some("1.6.0".into()),
            extversion_after: Some("1.6.0".into()),
            shipped_extversion: Some("1.6.0".into()),
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

    fn noop_migration_080() -> Migration080Report {
        Migration080Report {
            halfvec_conversion_applied: false,
            apply_executed: false,
        }
    }

    fn noop_migration_081() -> Migration081Report {
        Migration081Report {
            age_rls_applied: false,
            apply_executed: false,
            skipped_age_version: false,
        }
    }

    fn noop_migration_092() -> Migration092Report {
        Migration092Report {
            age_available: true,
            apply_executed: true,
            graphs_checked: 0,
            graphs_ready: 0,
            graphs_degraded: Vec::new(),
            fallback_env_enabled: false,
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
            SQL_038_APPLY.contains("\"Node\"") && SQL_038_APPLY.contains("\"EDGE\""),
            "M038 must target AGE child label tables (SPEC-034), not _ag_label_* parents"
        );
        assert!(
            SQL_038_APPLY.contains("idx_node_source_ids_gin"),
            "index names must be NAMEDATALEN-safe (≤63 bytes)"
        );
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
            missing_indexes: vec!["g.idx_node_source_ids_gin".into()],
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
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
    fn pgvector_cve_floor_blocks_081() {
        assert!(!helpers::pgvector_meets_cve_floor("0.8.0"));
        assert!(!helpers::pgvector_meets_cve_floor("0.8.1"));
        assert!(helpers::pgvector_meets_cve_floor("0.8.2"));
        let mut report = Migration042Report {
            pgvector_available: true,
            extversion_before: Some("0.8.1".into()),
            extversion_after: Some("0.8.1".into()),
            shipped_extversion: Some("0.8.5".into()),
            iterative_scan_capable: true,
            indexes_rebuilt: false,
            vector_tables_checked: 0,
            missing_ann_index_tables: 0,
        };
        let blockers = readiness_blockers(&Some(MigrationBootstrapReport {
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
            migration_042: report.clone(),
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
        }));
        assert!(
            blockers.iter().any(|b| b == "pgvector_cve_floor"),
            "blockers={blockers:?}"
        );
        report.extversion_after = Some("0.8.5".into());
        let blockers_ok = readiness_blockers(&Some(MigrationBootstrapReport {
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
        }));
        assert!(!blockers_ok.iter().any(|b| b == "pgvector_cve_floor"));
    }

    #[test]
    fn extension_version_at_least_semantics() {
        assert!(helpers::extension_version_at_least("0.8.3", "0.8.3"));
        assert!(helpers::extension_version_at_least("1.6.0", "1.6.0"));
        assert!(helpers::extension_version_at_least("1.7.0", "1.6.0"));
        assert!(!helpers::extension_version_at_least("0.7.4", "0.8.0"));
        assert!(!helpers::extension_version_at_least("1.5.0", "1.6.0"));
        assert!(!helpers::extension_version_at_least("0.8.0-rc1", "0.8.0"));
        assert!(helpers::extension_version_at_least("0.8.0", "0.8.0-rc1"));
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
            missing_ann_index_tables: 0,
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
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
            missing_ann_index_tables: 0,
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
        })));
    }

    #[test]
    fn degraded_when_ann_index_missing() {
        let report = Migration042Report {
            pgvector_available: true,
            extversion_before: Some("0.8.3".into()),
            extversion_after: Some("0.8.3".into()),
            shipped_extversion: Some("0.8.3".into()),
            iterative_scan_capable: true,
            indexes_rebuilt: false,
            vector_tables_checked: 2,
            missing_ann_index_tables: 1,
        };
        assert!(report.is_degraded());
        let blockers = readiness_blockers(&Some(MigrationBootstrapReport {
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
        }));
        assert!(blockers.iter().any(|b| b == "missing_hnsw_index"));
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
            migration_042: Migration042Report {
                pgvector_available: true,
                extversion_before: Some("0.8.3".into()),
                extversion_after: Some("0.8.3".into()),
                shipped_extversion: Some("0.8.3".into()),
                iterative_scan_capable: true,
                indexes_rebuilt: false,
                vector_tables_checked: 2,
                missing_ann_index_tables: 1,
            },
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
        })));
    }

    #[test]
    fn readiness_blockers_ssot_matches_is_ready() {
        // Empty report → ready
        assert!(is_ready_for_traffic(&None));
        assert!(readiness_blockers(&None).is_empty());

        // Healthy bootstrap → ready iff blockers empty
        let healthy = MigrationBootstrapReport {
            pending_before: 0,
            applied_versions: vec![],
            latest_version: Some(65),
            migration_038: Migration038Report {
                age_available: true,
                graphs_checked: 0,
                indexes_ready: true,
                indexes_repaired_inline: false,
                deferred_large_graphs: vec![],
                missing_indexes: vec![],
                operator_action: None,
            },
            migration_042: Migration042Report {
                pgvector_available: true,
                extversion_before: Some("0.8.3".into()),
                extversion_after: Some("0.8.3".into()),
                shipped_extversion: Some("0.8.3".into()),
                iterative_scan_capable: true,
                indexes_rebuilt: true,
                vector_tables_checked: 2,
                missing_ann_index_tables: 0,
            },
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
            migration_080: noop_migration_080(),
            migration_081: noop_migration_081(),
            migration_092: noop_migration_092(),
        };
        let blockers = readiness_blockers(&Some(healthy.clone()));
        assert_eq!(is_ready_for_traffic(&Some(healthy)), blockers.is_empty());
    }

    #[test]
    fn m083_apply_sql_is_idempotent_ssot() {
        assert!(SQL_083_APPLY.contains("idx_node_prop_node_id_unique"));
        assert!(SQL_083_APPLY.contains("idx_edge_source_target_unique"));
        assert!(SQL_083_APPLY.contains("node_id"));
        // SPEC-062 / D-30: drop legacy expression UNIQUEs when eq_* arbiters exist.
        assert!(SQL_083_APPLY.contains("idx_node_eq_node_id"));
        assert!(SQL_083_APPLY.contains("idx_edge_eq_source_target_rel"));
        assert!(SQL_083_APPLY.contains("idx_edge_eq_source_target"));
        assert!(SQL_083_APPLY.contains("DROP INDEX IF EXISTS"));
        // Fast-boot guard: skip O(N) dedup when a UNIQUE index already present.
        assert!(
            SQL_083_APPLY.contains("already exists") || SQL_083_APPLY.contains("already present")
        );
        assert!(SQL_083_APPLY.contains("skip dedup") || SQL_083_APPLY.contains("skip"));
        assert!(SQL_083_STATS_BACKFILL.contains("relationship_count"));
        assert!(SQL_083_STATS_BACKFILL.contains("metadata->>'relationship_count'"));
    }

    #[test]
    fn m092_apply_sql_is_boot_owned_eq_id_ssot() {
        assert!(SQL_092_APPLY.contains("eq_node_id"));
        assert!(SQL_092_APPLY.contains("eq_source_id"));
        assert!(SQL_092_APPLY.contains("eq_target_id"));
        // D-30: every-boot SSOT must add multigraph arbiter (support/ only — not checksummed).
        assert!(SQL_092_APPLY.contains("eq_rel_type"));
        assert!(SQL_092_APPLY.contains("idx_edge_eq_source_target_rel"));
        assert!(SQL_092_APPLY.contains("DROP INDEX IF EXISTS %I.idx_edge_eq_source_target"));
        assert!(SQL_092_APPLY.contains("trg_eq_sync_node_id"));
        assert!(SQL_092_APPLY.contains("statement_timeout = 0"));
        assert!(SQL_092_APPLY.contains("lock_timeout"));
        // Never execute DROP TRIGGER — only document the rule in comments.
        assert!(
            !SQL_092_APPLY
                .lines()
                .filter(|l| !l.trim_start().starts_with("--"))
                .any(|l| l.contains("DROP TRIGGER")),
            "M092 must not DROP TRIGGER in executable SQL"
        );
        assert_eq!(MIGRATION_092_VERSION, 92);
    }

    #[test]
    fn m092_readiness_skips_incomplete_age_graphs_without_node_edge() {
        // Incomplete AGE stubs (Node XOR EDGE) must not block /ready — same gate as apply.sql.
        let src = include_str!("reconcile/m092.rs");
        assert!(
            src.contains("tablename = 'Node'")
                && src.contains("tablename = 'EDGE'")
                && src.contains("leftover bind_probe"),
            "M092 post-reconcile scoring must require Node+EDGE before degraded"
        );
    }

    #[test]
    fn m086_apply_sql_is_boot_owned_bfs_ssot() {
        assert!(SQL_086_APPLY.contains("idx_edge_source_id"));
        assert!(SQL_086_APPLY.contains("idx_edge_target_id"));
        assert!(SQL_086_APPLY.contains("statement_timeout = 0"));
        assert!(SQL_086_APPLY.contains("lock_timeout"));
        assert!(SQL_086_APPLY.contains("ag_catalog.ag_graph"));
        assert_eq!(MIGRATION_086_VERSION, 86);
    }
}
