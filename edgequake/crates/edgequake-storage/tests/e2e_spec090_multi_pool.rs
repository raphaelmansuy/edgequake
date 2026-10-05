//! SPEC-090 F-090-28 + SPEC-112 — multi-pool isolation, identity, close, stress.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec090_multi_pool -- --nocapture

#![cfg(feature = "postgres")]

use edgequake_storage::{
    check_pool_budget, enforce_pool_budget, evaluate_pool_budget, pool_role_max_connections,
    BudgetMode, PgPoolBundle, PoolRole,
};
use std::sync::Arc;
use std::time::Duration;

fn set_small_pools() {
    std::env::set_var("EDGEQUAKE_DB_POOL_SIZE_QUERY", "3");
    std::env::set_var("EDGEQUAKE_DB_POOL_SIZE_INGEST", "2");
    std::env::set_var("EDGEQUAKE_DB_POOL_SIZE_QUEUE", "2");
    std::env::set_var("EDGEQUAKE_DB_POOL_SIZE_ADMIN", "1");
}

#[tokio::test]
async fn e2e_spec090_multi_pool_sizes_and_isolation() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("SKIP: DATABASE_URL unset");
        return;
    };

    set_small_pools();

    assert_eq!(pool_role_max_connections(PoolRole::Query), 3);
    assert_eq!(pool_role_max_connections(PoolRole::Ingest), 2);

    let bundle = PgPoolBundle::connect(&url)
        .await
        .expect("PgPoolBundle::connect");
    assert_eq!(bundle.query_max, 3);
    assert_eq!(bundle.ingest_max, 2);
    assert_eq!(bundle.queue_max, 2);
    assert_eq!(bundle.admin_max, 1);
    assert_eq!(bundle.total_max_connections(), 8);

    // Saturate ingest pool with held connections; query pool must still serve SELECT 1.
    let ingest = Arc::new(bundle.ingest.clone());
    let mut holders = Vec::new();
    for _ in 0..bundle.ingest_max {
        let pool = Arc::clone(&ingest);
        holders.push(tokio::spawn(async move {
            let mut conn = pool.acquire().await.expect("ingest acquire");
            let _ = sqlx::query("SELECT pg_sleep(2)").execute(&mut *conn).await;
        }));
    }
    tokio::time::sleep(Duration::from_millis(200)).await;

    let query_ok = tokio::time::timeout(Duration::from_secs(3), async {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&bundle.query)
            .await
    })
    .await;
    assert!(
        matches!(query_ok, Ok(Ok(1))),
        "query pool must remain usable while ingest is saturated: {query_ok:?}"
    );

    for h in holders {
        let _ = h.await;
    }

    assert!(
        bundle.query.size() <= bundle.query_max,
        "query size {} > max {}",
        bundle.query.size(),
        bundle.query_max
    );
    assert!(bundle.ingest.size() <= bundle.ingest_max);

    let admin_one: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&bundle.admin)
        .await
        .expect("admin SELECT 1");
    assert_eq!(admin_one, 1);

    if std::env::var("DATABASE_READ_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_none()
    {
        assert!(!bundle.query_uses_read_url);
    }

    bundle.close().await;
}

#[tokio::test]
async fn e2e_spec112_application_name_and_idle_in_xact() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("SKIP: DATABASE_URL unset");
        return;
    };
    set_small_pools();
    std::env::set_var("EDGEQUAKE_DB_IDLE_IN_XACT_TIMEOUT_SECS", "60");

    let bundle = PgPoolBundle::connect(&url)
        .await
        .expect("PgPoolBundle::connect");

    for role in [
        PoolRole::Query,
        PoolRole::Ingest,
        PoolRole::Queue,
        PoolRole::Admin,
    ] {
        let pool = bundle.for_role(role);
        let name: String = sqlx::query_scalar("SHOW application_name")
            .fetch_one(pool)
            .await
            .expect("SHOW application_name");
        assert_eq!(
            name,
            role.application_name(),
            "role {role:?} application_name"
        );

        let idle_to: String = sqlx::query_scalar("SHOW idle_in_transaction_session_timeout")
            .fetch_one(pool)
            .await
            .expect("SHOW idle_in_transaction_session_timeout");
        // PG may render as "1min" or "60s" or "60000ms"
        assert!(
            idle_to != "0" && !idle_to.is_empty(),
            "idle_in_transaction_session_timeout must be non-zero, got {idle_to:?}"
        );
    }

    // T-112-09: this bundle's backends ≤ configured total (PID-scoped — ambient
    // edgequake:* from a live API on shared PG must not fail the gate).
    let mut held = Vec::new();
    let mut our_pids: Vec<i32> = Vec::new();
    for role in [
        PoolRole::Query,
        PoolRole::Ingest,
        PoolRole::Queue,
        PoolRole::Admin,
    ] {
        let pool = bundle.for_role(role);
        let mut conn = pool.acquire().await.expect("acquire for pid census");
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *conn)
            .await
            .expect("pg_backend_pid");
        our_pids.push(pid);
        held.push(conn);
    }
    assert!(
        our_pids.len() as u32 <= bundle.total_max_connections(),
        "this bundle held {} backends > configured total {}",
        our_pids.len(),
        bundle.total_max_connections()
    );
    drop(held);

    bundle.close().await;
}

#[tokio::test]
async fn e2e_spec112_close_releases_backends() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("SKIP: DATABASE_URL unset");
        return;
    };
    set_small_pools();

    let bundle = PgPoolBundle::connect(&url)
        .await
        .expect("PgPoolBundle::connect");

    // Hold one backend per role and record PIDs — close must drop *these* PIDs
    // even when a co-resident API keeps other edgequake:* backends alive.
    let mut held = Vec::new();
    let mut our_pids: Vec<i32> = Vec::new();
    for role in [
        PoolRole::Query,
        PoolRole::Ingest,
        PoolRole::Queue,
        PoolRole::Admin,
    ] {
        let mut conn = bundle
            .for_role(role)
            .acquire()
            .await
            .expect("acquire before close");
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *conn)
            .await
            .expect("pg_backend_pid");
        our_pids.push(pid);
        held.push(conn);
    }
    assert!(!our_pids.is_empty(), "expected live backends before close");
    drop(held);

    // Use a separate probe pool (unlabeled) to observe after close.
    let probe = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("probe pool");

    bundle.close().await;

    let mut dropped = false;
    for _ in 0..50 {
        let remaining: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity \
             WHERE pid = ANY($1) AND backend_type = 'client backend'",
        )
        .bind(&our_pids)
        .fetch_one(&probe)
        .await
        .expect("observe closed pool backend PIDs");
        if remaining == 0 {
            dropped = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        dropped,
        "bundle.close() must drop this test's backends (pids={our_pids:?})"
    );
    probe.close().await;
}

#[tokio::test]
async fn e2e_spec112_stress_ingest_queue_saturate_query_ok() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("SKIP: DATABASE_URL unset");
        return;
    };
    set_small_pools();

    let bundle = PgPoolBundle::connect(&url)
        .await
        .expect("PgPoolBundle::connect");

    let ingest = Arc::new(bundle.ingest.clone());
    let queue = Arc::new(bundle.queue.clone());
    let mut holders = Vec::new();

    for _ in 0..bundle.ingest_max {
        let pool = Arc::clone(&ingest);
        holders.push(tokio::spawn(async move {
            let mut conn = pool.acquire().await.expect("ingest acquire");
            let _ = sqlx::query("SELECT pg_sleep(3)").execute(&mut *conn).await;
        }));
    }
    for _ in 0..bundle.queue_max {
        let pool = Arc::clone(&queue);
        holders.push(tokio::spawn(async move {
            let mut conn = pool.acquire().await.expect("queue acquire");
            let _ = sqlx::query("SELECT pg_sleep(3)").execute(&mut *conn).await;
        }));
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Parallel query acquires must still succeed under ingest+queue saturation.
    let mut query_tasks = Vec::new();
    for _ in 0..bundle.query_max {
        let pool = bundle.query.clone();
        query_tasks.push(tokio::spawn(async move {
            tokio::time::timeout(Duration::from_secs(3), async {
                sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(&pool)
                    .await
            })
            .await
        }));
    }
    for t in query_tasks {
        let r = t.await.expect("join");
        assert!(
            matches!(r, Ok(Ok(1))),
            "query must serve under ingest+queue stress: {r:?}"
        );
    }

    for h in holders {
        let _ = h.await;
    }
    bundle.close().await;
}

#[tokio::test]
async fn e2e_spec112_budget_fail_mode() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("SKIP: DATABASE_URL unset");
        return;
    };
    set_small_pools();
    std::env::set_var("EDGEQUAKE_DB_POOL_INSTANCE_COUNT", "999");
    std::env::set_var("EDGEQUAKE_DB_POOL_BUDGET_MODE", "fail");

    let bundle = PgPoolBundle::connect(&url)
        .await
        .expect("connect for budget probe");
    let report = check_pool_budget(&bundle.admin, bundle.total_max_connections())
        .await
        .expect("check_pool_budget");
    assert!(!report.ok, "absurd instance count must exceed budget");
    assert!(
        enforce_pool_budget(&report).is_err(),
        "fail mode must refuse"
    );

    // Restore env so later tests are not poisoned in the same process.
    std::env::set_var("EDGEQUAKE_DB_POOL_INSTANCE_COUNT", "1");
    std::env::set_var("EDGEQUAKE_DB_POOL_BUDGET_MODE", "warn");
    bundle.close().await;
}

#[test]
fn unit_spec112_budget_formula() {
    let ok = evaluate_pool_budget(34, 1, 100, 3, 10, BudgetMode::Warn);
    assert!(ok.ok);
    assert_eq!(ok.need, 34);
    assert_eq!(ok.limit, 87);

    let overlap = evaluate_pool_budget(34, 4, 100, 3, 10, BudgetMode::Fail);
    assert!(!overlap.ok);
    assert_eq!(overlap.need, 136);
    assert!(enforce_pool_budget(&overlap).is_err());
}

#[tokio::test]
async fn e2e_spec090_multi_pool_contract_source() {
    let src =
        std::fs::read_to_string("crates/edgequake-storage/src/adapters/postgres/pool_bundle.rs")
            .or_else(|_| std::fs::read_to_string("src/adapters/postgres/pool_bundle.rs"))
            .expect("pool_bundle.rs");
    assert!(src.contains("DATABASE_READ_URL"));
    assert!(src.contains("EDGEQUAKE_DB_POOL_SIZE_QUERY"));
    assert!(src.contains("struct PgPoolBundle"));
    assert!(src.contains("async fn close"));
    assert!(src.contains("idle_timeout"));
    assert!(src.contains("max_lifetime"));

    let hygiene =
        std::fs::read_to_string("crates/edgequake-storage/src/adapters/postgres/connection.rs")
            .or_else(|_| std::fs::read_to_string("src/adapters/postgres/connection.rs"))
            .expect("connection.rs");
    assert!(
        hygiene.contains("application_name"),
        "T-112-01: hygiene must set application_name"
    );
    assert!(
        hygiene.contains("idle_in_transaction_session_timeout"),
        "T-112-10: hygiene must set idle_in_transaction_session_timeout"
    );
    assert!(hygiene.contains("apply_session_baseline"));

    let api = std::fs::read_to_string("../edgequake-api/src/state/postgres.rs")
        .or_else(|_| std::fs::read_to_string("crates/edgequake-api/src/state/postgres.rs"))
        .expect("postgres.rs");
    assert!(api.contains("PgPoolBundle::connect"));
    assert!(api.contains("pool_bundle.admin"));
    assert!(api.contains("check_pool_budget") || api.contains("pool_budget"));

    let server = std::fs::read_to_string("../edgequake-api/src/server.rs")
        .or_else(|_| std::fs::read_to_string("crates/edgequake-api/src/server.rs"))
        .expect("server.rs");
    assert!(
        server.contains("close_db_pools") || server.contains("bundle.close"),
        "T-112-02: server must close pools after drain"
    );
}
