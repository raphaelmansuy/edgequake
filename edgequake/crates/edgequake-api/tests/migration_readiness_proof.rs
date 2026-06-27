//! SPEC-006 P6 — /ready readiness battle test (postgres feature).
//!
//! cargo test -p edgequake-api migration_readiness_proof --features postgres

#![cfg(feature = "postgres")]

use axum::extract::State;
use axum::http::StatusCode;
use edgequake_api::handlers::health::readiness_check;
use edgequake_api::state::migration_bootstrap::{
    Migration038Report, Migration042Report, Migration043Report, Migration044Report,
    Migration045Report, MigrationBootstrapReport,
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
