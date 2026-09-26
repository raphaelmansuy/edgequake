//! Migration readiness report types (SPEC-006 / SPEC-017).
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
