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
    assert!(is_legacy_cutover_assert(legacy_cutover_assert_version()));
    assert!(!is_irreversible_drop(legacy_cutover_assert_version()));

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
    assert!(SQL_083_APPLY.contains("already exists") || SQL_083_APPLY.contains("already present"));
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
