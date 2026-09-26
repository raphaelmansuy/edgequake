//! Ready-for-traffic helpers and boot-gate exit constants (SPEC-045 / SPEC-091).

use super::helpers;
use super::reports::MigrationBootstrapReport;

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
