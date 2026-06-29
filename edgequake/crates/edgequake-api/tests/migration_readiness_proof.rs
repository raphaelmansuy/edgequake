//! SPEC-006 P6 — /ready readiness battle test (postgres feature).
//!
//! cargo test -p edgequake-api migration_readiness_proof --features postgres

#![cfg(feature = "postgres")]

use axum::extract::State;
use axum::http::StatusCode;
use edgequake_api::handlers::health::readiness_check;
use edgequake_api::state::migration_bootstrap::{
    Migration038Report, Migration042Report, Migration043Report, Migration044Report,
    Migration045Report, Migration046Report, Migration047Report, Migration048Report,
    Migration049Report, Migration050Report, Migration051Report, Migration052Report,
    Migration053Report, Migration054Report, Migration055Report, Migration056Report,
    Migration057Report, Migration058Report, Migration059Report, Migration060Report,
    Migration061Report, Migration062Report, Migration063Report, Migration064Report,
    Migration065Report, MigrationBootstrapReport,
};
use edgequake_api::AppState;

fn degraded_bootstrap_report() -> MigrationBootstrapReport {
    MigrationBootstrapReport {
        pending_before: 0,
        applied_versions: vec![38],
        latest_version: Some(38),
        migration_038: Migration038Report {
            age_available: true,
            graphs_checked: 1,
            indexes_ready: false,
            indexes_repaired_inline: false,
            deferred_large_graphs: vec!["large_graph (900000 vertices)".into()],
            missing_indexes: vec!["large_graph.idx_large_graph_vertex_source_ids_gin".into()],
            operator_action: Some(
                "./edgequake/scripts/migrations/apply_038.sh --apply --concurrent --yes".into(),
            ),
        },
        migration_042: Migration042Report {
            pgvector_available: true,
            extversion_before: Some("0.8.0".into()),
            extversion_after: Some("0.8.0".into()),
            shipped_extversion: Some("0.8.3".into()),
            iterative_scan_capable: true,
            indexes_rebuilt: false,
            vector_tables_checked: 0,
        },
        migration_043: Migration043Report {
            age_available: true,
            extversion_before: Some("1.6.0".into()),
            extversion_after: Some("1.6.0".into()),
            extension_updated: false,
        },
        migration_044: Migration044Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_045: Migration045Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_046: Migration046Report {
            marker_present: true,
            apply_executed: false,
            graphs_checked: 0,
            missing_indexes: vec![],
        },
        migration_047: Migration047Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_048: Migration048Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_049: Migration049Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_050: Migration050Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_051: Migration051Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_052: Migration052Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_053: Migration053Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_054: Migration054Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_055: Migration055Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_056: Migration056Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_057: Migration057Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_058: Migration058Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_059: Migration059Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_060: Migration060Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_061: Migration061Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_062: Migration062Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_063: Migration063Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_064: Migration064Report {
            marker_present: true,
            apply_executed: false,
        },
        migration_065: Migration065Report {
            marker_present: true,
            apply_executed: false,
        },
    }
}

#[tokio::test]
async fn migration_readiness_proof_ok_when_no_postgres_bootstrap() {
    let state = AppState::test_state();
    assert_eq!(readiness_check(State(state)).await, StatusCode::OK);
}

#[tokio::test]
async fn migration_readiness_proof_503_when_migration_038_degraded() {
    let mut state = AppState::test_state();
    state.migration_bootstrap = Some(degraded_bootstrap_report());
    assert_eq!(
        readiness_check(State(state)).await,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn migration_readiness_proof_ok_when_indexes_ready() {
    let mut state = AppState::test_state();
    let mut report = degraded_bootstrap_report();
    report.migration_038.indexes_ready = true;
    report.migration_038.missing_indexes.clear();
    report.migration_038.deferred_large_graphs.clear();
    report.migration_038.operator_action = None;
    state.migration_bootstrap = Some(report);
    assert_eq!(readiness_check(State(state)).await, StatusCode::OK);
}
