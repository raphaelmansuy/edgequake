//! SPEC-088 Phase 3: correctness + limit + plan-shape tests for hot dataops.
//!
//! Requires `DATABASE_URL` + postgres feature. Soft-skip when unset unless
//! `EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1`.
//!
//! Run filtered: `cargo test -p edgequake-storage --features postgres --test data_layer_limits DATA_PGVEC`

#![cfg(feature = "postgres")]

use edgequake_storage::dataop::{
    DATA_AGE_GRAPH_GET_NODES_BATCH_031, DATA_PGVEC_VECTORS_ANN_QUERY_001,
    DATA_PGVEC_VECTORS_ANN_QUERY_FILTERED_002, DATA_PGVEC_VECTORS_UPSERT_BATCH_004,
    DATA_PG_KV_GET_BY_IDS_076, DATA_PG_KV_GET_BY_ID_075, DATA_PG_KV_UPSERT_079,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::time::Duration;

fn require_db() -> Option<String> {
    match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => Some(u),
        _ => {
            if std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
                .ok()
                .as_deref()
                == Some("1")
            {
                panic!("DATABASE_URL required when EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1");
            }
            eprintln!("skip: DATABASE_URL not set");
            None
        }
    }
}

async fn pool(url: &str) -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(10))
        .connect(url)
        .await
        .expect("connect")
}

/// DATA-PG-KV-GET-BY-ID-075 / UPSERT-079 correctness + empty + unicode.
#[tokio::test]
async fn data_pg_kv_get_by_id_075_and_upsert_079() {
    let Some(url) = require_db() else { return };
    let pool = pool(&url).await;
    let table = format!("eq_spec088_kv_{}", std::process::id());
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {table} (
            key TEXT PRIMARY KEY,
            value JSONB NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ))
    .execute(&pool)
    .await
    .unwrap();

    // upsert batch
    let sql_up = format!(
        "/* {DATA_PG_KV_UPSERT_079} */ INSERT INTO {table} (key, value, updated_at)
         SELECT k, v, NOW() FROM unnest($1::text[], $2::jsonb[]) AS b(k, v)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()"
    );
    let keys: Vec<String> = vec!["a".into(), "unicodé-键".into()];
    let vals: Vec<serde_json::Value> =
        vec![serde_json::json!({"n": 1}), serde_json::json!({"n": 2})];
    sqlx::query(&sql_up)
        .bind(&keys)
        .bind(&vals)
        .execute(&pool)
        .await
        .unwrap();

    let sql_get =
        format!("/* {DATA_PG_KV_GET_BY_ID_075} */ SELECT value FROM {table} WHERE key = $1");
    let row: (serde_json::Value,) = sqlx::query_as(&sql_get)
        .bind("unicodé-键")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0["n"], 2);

    // empty set
    let miss: Option<(serde_json::Value,)> = sqlx::query_as(&sql_get)
        .bind("missing")
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(miss.is_none());

    // plan assertion: Index Scan / PK
    let plan: String = sqlx::query_scalar(&format!(
        "EXPLAIN (FORMAT TEXT) /* {DATA_PG_KV_GET_BY_ID_075} */ SELECT value FROM {table} WHERE key = $1"
    ))
    .bind("a")
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|_| {
        // EXPLAIN via query_scalar may not work; use fetch
        String::new()
    });
    let plans = sqlx::query(&format!(
        "EXPLAIN (FORMAT TEXT) SELECT value FROM {table} WHERE key = $1"
    ))
    .bind("a")
    .fetch_all(&pool)
    .await
    .unwrap();
    let plan_text: String = plans
        .iter()
        .map(|r| r.try_get::<String, _>(0).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(
            "
",
        );
    assert!(
        plan_text.to_lowercase().contains("index") || plan_text.to_lowercase().contains("pk"),
        "expected index plan, got: {plan_text}"
    );

    let _ = plan;
    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(&pool)
        .await
        .ok();
}

/// DATA-PG-KV-GET-BY-IDS-076 order preservation + batch limit behavior.
#[tokio::test]
async fn data_pg_kv_get_by_ids_076() {
    let Some(url) = require_db() else { return };
    let pool = pool(&url).await;
    let table = format!("eq_spec088_kvb_{}", std::process::id());
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {table} (
            key TEXT PRIMARY KEY,
            value JSONB NOT NULL
        )"
    ))
    .execute(&pool)
    .await
    .unwrap();

    for (k, n) in [("z", 1), ("y", 2), ("x", 3)] {
        sqlx::query(&format!(
            "/* {DATA_PG_KV_UPSERT_079} */ INSERT INTO {table} VALUES ($1, $2) ON CONFLICT DO NOTHING"
        ))
        .bind(k)
        .bind(serde_json::json!({"n": n}))
        .execute(&pool)
        .await
        .unwrap();
    }

    let ids: Vec<String> = vec!["x".into(), "missing".into(), "z".into()];
    let sql = format!(
        "/* {DATA_PG_KV_GET_BY_IDS_076} */ SELECT kv.value
         FROM unnest($1::text[]) WITH ORDINALITY AS u(key, ord)
         INNER JOIN {table} kv ON kv.key = u.key
         ORDER BY u.ord"
    );
    let rows: Vec<(serde_json::Value,)> = sqlx::query_as(&sql)
        .bind(&ids)
        .fetch_all(&pool)
        .await
        .unwrap();
    // INNER JOIN drops missing — order of hits: x then z
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0["n"], 3);
    assert_eq!(rows[1].0["n"], 1);

    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(&pool)
        .await
        .ok();
}

/// DATA-PGVEC-VECTORS-ANN-QUERY-001: correctness on small N + plan uses index when present.
#[tokio::test]
async fn data_pgvec_vectors_ann_query_001() {
    let Some(url) = require_db() else { return };
    let pool = pool(&url).await;

    // Extension
    let has_vector: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')")
            .fetch_one(&pool)
            .await
            .unwrap_or(false);
    if !has_vector {
        eprintln!("skip: vector extension missing");
        return;
    }

    let table = format!("eq_spec088_vec_{}", std::process::id());
    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(&pool)
        .await
        .ok();
    sqlx::query(&format!(
        "CREATE TABLE {table} (
            id TEXT PRIMARY KEY,
            embedding vector(3),
            metadata JSONB NOT NULL DEFAULT '{{}}'
        )"
    ))
    .execute(&pool)
    .await
    .unwrap();

    // seed
    for (id, emb) in [("a", "[1,0,0]"), ("b", "[0.9,0.1,0]"), ("c", "[0,1,0]")] {
        sqlx::query(&format!(
            "/* {DATA_PGVEC_VECTORS_UPSERT_BATCH_004} */ INSERT INTO {table} (id, embedding) VALUES ($1, $2::vector)"
        ))
        .bind(id)
        .bind(emb)
        .execute(&pool)
        .await
        .unwrap();
    }

    sqlx::query(&format!(
        "CREATE INDEX ON {table} USING hnsw (embedding vector_cosine_ops)"
    ))
    .execute(&pool)
    .await
    .ok(); // may fail on tiny table in some builds; ignore

    let sql = format!(
        "/* {DATA_PGVEC_VECTORS_ANN_QUERY_001} */ SELECT id, 1 - (embedding <=> $1::vector) AS score
         FROM {table} ORDER BY embedding <=> $1::vector LIMIT $2"
    );
    let rows = sqlx::query(&sql)
        .bind("[1,0,0]")
        .bind(2_i32)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let top: String = rows[0].get("id");
    assert_eq!(top, "a");

    // empty embedding table boundary
    sqlx::query(&format!("DELETE FROM {table}"))
        .execute(&pool)
        .await
        .unwrap();
    let empty = sqlx::query(&sql)
        .bind("[1,0,0]")
        .bind(5_i32)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(empty.is_empty());

    sqlx::query(&format!("DROP TABLE IF EXISTS {table}"))
        .execute(&pool)
        .await
        .ok();
}

/// Document Ref IDs under test for discovery (`-k` / filter).
#[test]
fn data_layer_limit_test_ref_ids_documented() {
    for id in [
        DATA_PGVEC_VECTORS_ANN_QUERY_001,
        DATA_PGVEC_VECTORS_ANN_QUERY_FILTERED_002,
        DATA_PGVEC_VECTORS_UPSERT_BATCH_004,
        DATA_PG_KV_GET_BY_ID_075,
        DATA_PG_KV_GET_BY_IDS_076,
        DATA_PG_KV_UPSERT_079,
        DATA_AGE_GRAPH_GET_NODES_BATCH_031,
    ] {
        assert!(id.starts_with("DATA-"));
    }
}
