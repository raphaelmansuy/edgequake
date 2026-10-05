//! Isolated, single-connection fixture for extension and pool fault tests.

use edgequake_storage::adapters::postgres::with_session_hygiene_labeled;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;
use std::time::Duration;

#[path = "query_capture.rs"]
mod query_capture;

pub async fn test_pool() -> Option<PgPool> {
    test_pool_with_acquire_timeout(Duration::from_millis(500)).await
}

pub async fn test_pool_with_acquire_timeout(acquire_timeout: Duration) -> Option<PgPool> {
    query_capture::initialize();
    let url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| std::fs::read_to_string("/tmp/edgequake-db-url").ok());
    let Some(url) = url else {
        assert!(
            !std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
                .ok()
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
            "PostgreSQL reliability tests require DATABASE_URL"
        );
        eprintln!("SKIP: PostgreSQL URL unavailable");
        return None;
    };
    let options: PgConnectOptions = url.trim().parse().expect("valid PostgreSQL URL");
    let database = options.get_database().unwrap_or("edgequake");
    let database = if database.ends_with("_test") {
        database.to_string()
    } else {
        format!("{database}_test")
    };
    Some(
        with_session_hygiene_labeled(
            PgPoolOptions::new()
                .max_connections(1)
                .min_connections(0)
                .acquire_timeout(acquire_timeout),
            "edgequake:query",
        )
        .connect_with(options.database(&database))
        .await
        .expect("isolated test database"),
    )
}
