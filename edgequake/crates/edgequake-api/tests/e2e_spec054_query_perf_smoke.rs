//! SPEC-054 / specs/054 — Postgres performance smoke (B1-a).
//!
//! Requires `DATABASE_URL`. Skips cleanly when unset.
//! Budget: M083 support apply when UNIQUE indexes already exist must finish
//! in under 2 seconds (warm local graph).

use std::path::PathBuf;
use std::time::{Duration, Instant};

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .or_else(|| {
            std::fs::read_to_string("/tmp/edgequake-db-url")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
}

fn support_083_path() -> PathBuf {
    // crates/edgequake-api → edgequake/migrations/support/083/apply.sql
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations/support/083/apply.sql")
}

#[tokio::test]
async fn e2e_m083_fast_path_under_two_seconds_when_unique_exists() {
    let Some(url) = database_url() else {
        eprintln!("SKIP: DATABASE_URL unset — M083 perf smoke not run");
        return;
    };

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect DATABASE_URL");

    // Precondition: at least one graph has the UNIQUE index (warm DB).
    let unique_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint FROM pg_indexes
        WHERE indexname = 'idx_node_prop_node_id_unique'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("count unique indexes");

    if unique_count == 0 {
        eprintln!("SKIP: no idx_node_prop_node_id_unique — cold DB / AGE unused");
        return;
    }

    let sql = std::fs::read_to_string(support_083_path()).expect("read support/083/apply.sql");

    let start = Instant::now();
    sqlx::raw_sql(&sql)
        .execute(&pool)
        .await
        .expect("M083 apply.sql must succeed");
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "B1-a FAIL: M083 fast-path took {elapsed:?} (budget 2s). \
         Dedup/ANALYZE may have run despite UNIQUE existing — see specs/054-fix-bugs-17/003."
    );
    eprintln!("OK B1-a: M083 apply fast-path in {elapsed:?} (unique_graphs≈{unique_count})");
}

#[tokio::test]
async fn e2e_default_graph_unique_indexes_present() {
    let Some(url) = database_url() else {
        eprintln!("SKIP: DATABASE_URL unset");
        return;
    };

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("connect");

    let graph: Option<String> = sqlx::query_scalar(
        r#"
        SELECT n.nspname::text
        FROM pg_namespace n
        JOIN pg_class c ON c.relnamespace = n.oid
        WHERE c.relname = 'Node'
          AND n.nspname LIKE 'eq\_eq\_%'
        ORDER BY CASE WHEN n.nspname = 'eq_eq_default_graph' THEN 0 ELSE 1 END, n.nspname
        LIMIT 1
        "#,
    )
    .fetch_optional(&pool)
    .await
    .expect("find Node schema");

    let Some(graph) = graph else {
        eprintln!("SKIP: no eq_* Node label tables");
        return;
    };

    let names: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT indexname::text FROM pg_indexes
        WHERE schemaname = $1
          AND indexname IN ('idx_node_prop_node_id_unique', 'idx_edge_source_target_unique')
        ORDER BY 1
        "#,
    )
    .bind(&graph)
    .fetch_all(&pool)
    .await
    .expect("list unique indexes");

    assert!(
        names.iter().any(|n| n == "idx_node_prop_node_id_unique"),
        "graph {graph} missing idx_node_prop_node_id_unique (native upsert / ON CONFLICT)"
    );
    // EDGE table may be empty on tiny graphs but UNIQUE should still exist after M083.
    if names.iter().any(|n| n == "idx_edge_source_target_unique") {
        eprintln!("OK: {graph} has node+edge UNIQUE indexes");
    } else {
        eprintln!("WARN: {graph} missing edge UNIQUE — check M083 for EDGE table");
    }
}
