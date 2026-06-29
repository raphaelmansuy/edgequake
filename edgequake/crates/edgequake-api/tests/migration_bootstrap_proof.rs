//! SPEC-006 P5 — migration bootstrap Postgres E2E proof.
//!
//! Requires: `DATABASE_URL` and `--features postgres`
//!
//! Run:
//!   cargo test -p edgequake-api migration_bootstrap_proof --features postgres

#![cfg(feature = "postgres")]

mod common;

use edgequake_api::state::migration_bootstrap::{
    is_ready_for_traffic, run_postgres_migrations, MIGRATION_038_VERSION, MIGRATION_046_VERSION,
};
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn migration_bootstrap_proof_postgres_e2e() {
    let database_url = match common::spec013_postgres::database_url() {
        Some(url) => url,
        None => {
            eprintln!("SKIP migration_bootstrap_proof_postgres_e2e: DATABASE_URL not set");
            return;
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET search_path TO public")
                    .execute(conn)
                    .await
                    .map(|_| ())
            })
        })
        .connect(&database_url)
        .await
        .expect("connect postgres");

    let report = run_postgres_migrations(&pool)
        .await
        .expect("bootstrap migrations");

    let version_applied: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = $1 AND success = true)",
    )
    .bind(MIGRATION_038_VERSION)
    .fetch_one(&pool)
    .await
    .expect("check 038 marker");

    assert!(
        version_applied || report.latest_version.unwrap_or(0) >= MIGRATION_038_VERSION,
        "migration 038 marker should be recorded"
    );

    let age_available = report.migration_038.age_available;
    if age_available {
        if report.migration_038.indexes_ready {
            assert!(
                is_ready_for_traffic(&Some(report.clone())),
                "ready when indexes present"
            );
        } else {
            assert!(
                !is_ready_for_traffic(&Some(report.clone())),
                "not ready when indexes missing on AGE graph"
            );
            assert!(
                report.migration_038.operator_action.is_some(),
                "degraded state must include operator_action"
            );
        }
    } else {
        assert!(is_ready_for_traffic(&Some(report)));
    }
}

#[tokio::test]
async fn migration_bootstrap_proof_idempotent_second_run() {
    let database_url = match common::spec013_postgres::database_url() {
        Some(url) => url,
        None => {
            eprintln!("SKIP migration_bootstrap_proof_idempotent_second_run: DATABASE_URL not set");
            return;
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect postgres");

    let first = run_postgres_migrations(&pool)
        .await
        .expect("first bootstrap");
    let second = run_postgres_migrations(&pool)
        .await
        .expect("second bootstrap");

    assert_eq!(
        second.pending_before, 0,
        "second bootstrap should have no pending sqlx migrations"
    );
    assert_eq!(
        first.migration_038.indexes_ready, second.migration_038.indexes_ready,
        "index readiness must be stable across restarts"
    );
    assert!(
        second.migration_046.marker_present || second.migration_046.apply_executed,
        "migration 046 graph isolation indexes must be reconciled at startup (migration_047 status: {})",
        second.migration_047.marker_present || second.migration_047.apply_executed,
    );
    let version_046: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = $1 AND success = true)",
    )
    .bind(MIGRATION_046_VERSION)
    .fetch_one(&pool)
    .await
    .expect("check 046 marker");
    assert!(
        version_046 || second.latest_version.unwrap_or(0) >= MIGRATION_046_VERSION,
        "migration 046 marker should be recorded after bootstrap"
    );
}
