//! Embedded support SQL (`migrations/support/**`) — SPEC-150 WP-9 split.

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
/// Migration 083 AGE unique index reconcile marker (support SQL still applied).
#[allow(dead_code)] // retained for reconcile/version SSOT; exercised in integration paths
pub const MIGRATION_083_VERSION: i64 = 83;

/// Native UNIQUE index reconcile — SSOT: `migrations/support/083/apply.sql`
pub const SQL_083_APPLY: &str = include_str!("../../../../../migrations/support/083/apply.sql");

/// JSONB→column stats backfill — SSOT: `migrations/support/083/stats_backfill.sql`
pub const SQL_083_STATS_BACKFILL: &str =
    include_str!("../../../../../migrations/support/083/stats_backfill.sql");

/// sqlx migration version for EDGE BFS index reconcile (SPEC-053 / SPEC-070).
#[allow(dead_code)] // used by bootstrap unit tests / reconcile SSOT
pub const MIGRATION_086_VERSION: i64 = 86;

/// EDGE BFS index reconcile — SSOT: `migrations/support/086/apply.sql`
pub const SQL_086_APPLY: &str = include_str!("../../../../../migrations/support/086/apply.sql");

/// sqlx migration version for eq_* denorm marker (SPEC-062 / SPEC-069).
#[allow(dead_code)] // used by bootstrap unit tests / reconcile SSOT
pub const MIGRATION_092_VERSION: i64 = 92;

/// eq_* denorm reconcile — SSOT: `migrations/support/092/apply.sql`
pub const SQL_092_APPLY: &str = include_str!("../../../../../migrations/support/092/apply.sql");
