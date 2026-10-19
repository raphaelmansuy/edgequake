//! Isolated, single-connection fixture for extension and pool fault tests.

use edgequake_storage::adapters::postgres::with_session_hygiene_labeled;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use std::time::Duration;

#[path = "postgres_test_config.rs"]
mod postgres_test_config;

pub async fn test_pool() -> Option<PgPool> {
    test_pool_with_acquire_timeout(Duration::from_millis(500)).await
}

pub async fn test_pool_with_acquire_timeout(acquire_timeout: Duration) -> Option<PgPool> {
    // The harness resolves the scratch database (honouring
    // `EDGEQUAKE_TEST_DATABASE`) and provisions it before anything connects.
    // Resolving it here as well is how this fixture once ran against a stale,
    // never-migrated `{db}_test`.
    let Some(config) = postgres_test_config::contract_postgres_config("access_pool") else {
        assert!(
            !postgres_test_config::postgres_tests_required(),
            "PostgreSQL reliability tests require DATABASE_URL"
        );
        eprintln!("SKIP: PostgreSQL URL unavailable");
        return None;
    };
    let options: PgConnectOptions = config
        .connect_options()
        .options([("search_path", "public")]);
    Some(
        with_session_hygiene_labeled(
            PgPoolOptions::new()
                .max_connections(1)
                .min_connections(0)
                .acquire_timeout(acquire_timeout),
            "edgequake:query",
        )
        .connect_with(options)
        .await
        .expect("isolated test database"),
    )
}
