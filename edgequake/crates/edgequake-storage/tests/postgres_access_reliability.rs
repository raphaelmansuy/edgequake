//! Live pool and timeout fault tests. No persistent tables or schema mutations.
//! CI must set EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 to forbid a missing-DB skip.
#![cfg(feature = "postgres")]

#[path = "support/perf_harness.rs"]
mod perf_harness;

use edgequake_storage::adapters::postgres::LocalTimeoutTx;
#[path = "support/postgres_access_pool.rs"]
mod postgres_access_pool;
use postgres_access_pool::test_pool;
use std::time::{Duration, Instant};

#[tokio::test]
async fn statement_timeout_rolls_back_writes_and_pool_recovers() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("CREATE TEMP TABLE access_timeout_probe (id integer)")
        .execute(&mut *conn)
        .await
        .unwrap();
    let mut tx = LocalTimeoutTx::begin(&mut conn, 75).await.unwrap();
    sqlx::query("INSERT INTO access_timeout_probe VALUES (1)")
        .execute(tx.as_mut())
        .await
        .unwrap();
    let started = Instant::now();
    let error = sqlx::query("SELECT pg_sleep(2)")
        .execute(tx.as_mut())
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("57014")
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "server must cancel before sleep completes"
    );
    assert!(matches!(
        edgequake_storage::error::postgres_access_error(error),
        edgequake_storage_contracts::AccessError::DeadlineExceeded(_)
    ));
    tx.rollback().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM access_timeout_probe")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(
        count, 0,
        "timed-out transaction must not commit its preceding writes"
    );
    drop(conn);
    let timeout: String = sqlx::query_scalar("SHOW statement_timeout")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        timeout, "0",
        "transaction timeout must not leak onto the pooled session"
    );
    pool.close().await;
}

#[tokio::test]
async fn abandoned_query_releases_single_connection_within_server_deadline() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let started = Instant::now();
    let mut conn = pool.acquire().await.unwrap();
    let mut tx = LocalTimeoutTx::begin(&mut conn, 100).await.unwrap();
    let result = tokio::time::timeout(
        Duration::from_millis(25),
        sqlx::query("SELECT pg_sleep(2)").execute(tx.as_mut()),
    )
    .await;
    assert!(
        result.is_err(),
        "exercise client-side cancellation before the server deadline"
    );
    drop(tx);
    drop(conn);
    let healthy = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&pool),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(healthy, 1);
    assert!(started.elapsed() < Duration::from_secs(2));
    pool.close().await;
}

#[tokio::test]
async fn saturation_is_bounded_and_session_hygiene_is_measured() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let mut held = pool.acquire().await.unwrap();
    let started = Instant::now();
    assert!(matches!(
        pool.acquire().await,
        Err(sqlx::Error::PoolTimedOut)
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
    sqlx::raw_sql("SET search_path = pg_catalog; SET statement_timeout = '123ms'; SET application_name = 'dirty';")
        .execute(&mut *held).await.unwrap();
    drop(held);
    let mut samples = Vec::new();
    for _ in 0..31 {
        let start = Instant::now();
        let mut conn = pool.acquire().await.unwrap();
        let (name, path, timeout): (String, String, String) = sqlx::query_as(
            "SELECT current_setting('application_name'), current_setting('search_path'), current_setting('statement_timeout')")
            .fetch_one(&mut *conn).await.unwrap();
        assert_eq!(name, "edgequake:query");
        assert_eq!(path, "public");
        assert_eq!(timeout, "0");
        samples.push(start.elapsed());
    }
    perf_harness::finish_report(
        "pool_hygiene_round_trip",
        &perf_harness::samples_after_warmup(&samples, 30),
        500.0,
        "acquire_and_select",
        false,
        "pool=1 samples=30 includes release cleanup wait",
    );
    pool.close().await;
}

#[tokio::test]
async fn terminated_backend_is_replaced_with_a_clean_session() {
    let Some(pool) = test_pool().await else {
        return;
    };
    let Some(probe) = test_pool().await else {
        return;
    };
    let mut conn = pool.acquire().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    let terminated: bool = sqlx::query_scalar("SELECT pg_terminate_backend($1)")
        .bind(pid)
        .fetch_one(&probe)
        .await
        .unwrap();
    assert!(terminated);
    assert!(sqlx::query("SELECT 1").execute(&mut *conn).await.is_err());
    drop(conn);
    let (new_pid, name): (i32, String) =
        sqlx::query_as("SELECT pg_backend_pid(), current_setting('application_name')")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(new_pid, pid);
    assert_eq!(name, "edgequake:query");
    pool.close().await;
    probe.close().await;
}

#[tokio::test]
async fn typed_access_errors_preserve_live_constraint_and_retry_categories() {
    use edgequake_storage::error::postgres_access_error;
    use edgequake_storage_contracts::AccessError;
    let Some(pool) = test_pool().await else {
        return;
    };
    let mut conn = pool.acquire().await.unwrap();
    sqlx::raw_sql("CREATE TEMP TABLE access_error_parent(id integer PRIMARY KEY); CREATE TEMP TABLE access_error_child(id integer PRIMARY KEY REFERENCES access_error_parent(id), CHECK(id > 0)); INSERT INTO access_error_parent VALUES(1); INSERT INTO access_error_child VALUES(1);")
        .execute(&mut *conn).await.unwrap();
    for (statement, code) in [
        ("INSERT INTO access_error_child VALUES(1)", "23505"),
        ("INSERT INTO access_error_child VALUES(2)", "23503"),
        ("INSERT INTO access_error_child VALUES(-1)", "23514"),
    ] {
        let error = sqlx::query(statement)
            .execute(&mut *conn)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some(code)
        );
        assert!(matches!(
            postgres_access_error(error),
            AccessError::Conflict(_)
        ));
    }
    for code in ["40001", "40P01"] {
        let statement = format!("DO $$ BEGIN RAISE EXCEPTION USING ERRCODE = '{code}', MESSAGE = 'retry mapping probe'; END $$");
        let error = sqlx::query(&statement)
            .execute(&mut *conn)
            .await
            .unwrap_err();
        assert!(matches!(
            postgres_access_error(error),
            AccessError::SerializationRetry(_)
        ));
    }
    drop(conn);
    pool.close().await;
}
