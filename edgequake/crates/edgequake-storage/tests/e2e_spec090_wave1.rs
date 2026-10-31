//! SPEC-090 Wave 1 e2e gates against live PostgreSQL.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec090_wave1 -- --nocapture

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::{
    with_session_hygiene, AnnExactReorderPolicy, PgVectorStorage, PostgresPool,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::fs;
use uuid::Uuid;

const DIM: usize = 8;

fn emb(seed: f32) -> Vec<f32> {
    vec![seed; DIM]
}

#[tokio::test]
async fn e2e_spec090_counter_statement_trigger() {
    let Some(config) = require_or_skip_postgres("spec090_ctr") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    let stats = format!("eq_{}_vectors_stats", config.table_prefix());
    let before: i64 =
        sqlx::query_scalar("SELECT n_tup_upd FROM pg_stat_user_tables WHERE relname = $1")
            .bind(&stats)
            .fetch_optional(&raw)
            .await
            .expect("stat")
            .unwrap_or(0);

    // Force stats refresh baseline
    let _ = sqlx::query("SELECT pg_stat_reset_single_table_counters($1::regclass)")
        .bind(format!("public.{stats}"))
        .execute(&raw)
        .await;

    let n = 200usize;
    let batch: Vec<_> = (0..n)
        .map(|i| {
            (
                format!("c-{i}"),
                emb(0.01 * i as f32),
                serde_json::json!({"workspace_id": "ws", "type": "chunk", "content": "hello"}),
            )
        })
        .collect();
    store.upsert(&batch).await.expect("upsert");

    // Statement-level trigger: one UPDATE per INSERT statement (here one chunk).
    let upd: i64 =
        sqlx::query_scalar("SELECT n_tup_upd FROM pg_stat_user_tables WHERE relname = $1")
            .bind(&stats)
            .fetch_one(&raw)
            .await
            .expect("n_tup_upd");

    let count: i64 = sqlx::query_scalar(&format!("SELECT row_count FROM {stats} WHERE id = 1"))
        .fetch_one(&raw)
        .await
        .expect("row_count");

    assert_eq!(count, n as i64, "maintained counter must match inserts");
    // After reset, a single statement insert of 200 rows → ~1 stats update (not 200).
    assert!(
        upd <= 5,
        "expected ~1 statement-level stats update, got n_tup_upd={upd} (before={before})"
    );

    let trg: String = sqlx::query_scalar(
        r#"
        SELECT pg_get_triggerdef(t.oid)
        FROM pg_trigger t
        JOIN pg_class c ON c.oid = t.tgrelid
        WHERE c.relname = $1 AND NOT t.tgisinternal
          AND pg_get_triggerdef(t.oid) ILIKE '%stats_insert%'
        LIMIT 1
        "#,
    )
    .bind(format!("eq_{}_vectors", config.table_prefix()))
    .fetch_one(&raw)
    .await
    .expect("trigger def");
    assert!(
        trg.to_ascii_uppercase().contains("FOR EACH STATEMENT"),
        "trigger must be FOR EACH STATEMENT, got: {trg}"
    );
}

#[tokio::test]
async fn e2e_spec090_pdf_list_no_blob() {
    let src =
        fs::read_to_string("crates/edgequake-storage/src/adapters/postgres/pdf_list_query.rs")
            .or_else(|_| fs::read_to_string("src/adapters/postgres/pdf_list_query.rs"))
            .expect("read pdf_list_query.rs");
    assert!(
        !src.contains("pdf_data,") && !src.contains("markdown_content,"),
        "list projection must not select pdf_data/markdown_content"
    );
    assert!(
        src.contains("SPEC-090") || src.contains("metadata-only"),
        "must document SPEC-090 list projection"
    );
}

#[tokio::test]
async fn e2e_spec090_pool_discard() {
    let Some(config) = require_or_skip_postgres("spec090_pool") else {
        return;
    };
    // Single connection so after_release runs on the same backend we polluted.
    let raw = with_session_hygiene(sqlx::postgres::PgPoolOptions::new().max_connections(1))
        .connect(&config.connection_url())
        .await
        .expect("pool");

    let default_mem: String = {
        let mut c = raw.acquire().await.expect("def");
        sqlx::query_scalar("SHOW maintenance_work_mem")
            .fetch_one(&mut *c)
            .await
            .expect("def mem")
    };

    {
        let mut conn = raw.acquire().await.expect("acq");
        sqlx::query("SET maintenance_work_mem = '256MB'")
            .execute(&mut *conn)
            .await
            .expect("set mem");
        sqlx::query("SET search_path TO ag_catalog, public")
            .execute(&mut *conn)
            .await
            .expect("set path");
        let polluted: String = sqlx::query_scalar("SHOW maintenance_work_mem")
            .fetch_one(&mut *conn)
            .await
            .expect("polluted");
        assert!(
            polluted.to_ascii_lowercase().contains("256"),
            "precondition: mem polluted"
        );
        // drop conn → after_release RESET ALL + pin search_path
    }

    let mut conn = raw.acquire().await.expect("acq2");
    let mem: String = sqlx::query_scalar("SHOW maintenance_work_mem")
        .fetch_one(&mut *conn)
        .await
        .expect("show mem");
    let path: String = sqlx::query_scalar("SHOW search_path")
        .fetch_one(&mut *conn)
        .await
        .expect("path");

    assert_eq!(
        mem.to_ascii_lowercase(),
        default_mem.to_ascii_lowercase(),
        "maintenance_work_mem must reset after release (was forced to 256MB)"
    );
    assert!(
        path == "public" || path.starts_with("public"),
        "search_path must be pinned to public after release, got {path}"
    );
}

#[tokio::test]
async fn e2e_spec090_edge_any_param() {
    let src = fs::read_to_string(
        "crates/edgequake-storage/src/adapters/postgres/graph/query_ops/expand.rs",
    )
    .or_else(|_| fs::read_to_string("src/adapters/postgres/graph/query_ops/expand.rs"))
    .expect("expand.rs");
    assert!(
        src.contains("= ANY($1::text[])"),
        "edge lookup must bind ANY($1::text[])"
    );
    assert!(
        !src.contains("IN ({})") || !src.contains("ids_str"),
        "must not interpolate IN-list literals"
    );
}

#[tokio::test]
async fn e2e_spec090_relaxed_reorder() {
    let Some(config) = require_or_skip_postgres("spec090_ord") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw, config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    let ws = format!("ws-{}", Uuid::new_v4());
    let mut batch = Vec::new();
    for i in 0..80 {
        let mut v = emb(1.0 + i as f32 * 0.05);
        // Make vectors linearly independent-ish for stable cosine order.
        v[0] = 1.0 + (i as f32) * 0.1;
        v[1] = (i as f32) * 0.07;
        batch.push((
            format!("o-{i}"),
            v,
            serde_json::json!({
                "workspace_id": ws,
                "tenant_id": "t",
                "type": "chunk",
            }),
        ));
    }
    store.upsert(&batch).await.expect("upsert");

    let policy = AnnExactReorderPolicy::for_search("relaxed_order", 10);
    assert!(policy.enabled);
    assert!(policy.effective_candidate_k(10) >= 40);

    let mf = MetadataFilter {
        workspace_id: Some(ws),
        tenant_id: Some("t".into()),
        vector_type: Some("chunk".into()),
        document_ids: None,
        modalities: None,
        embedding_model: None,
        kinds: None,
    };
    let mut q = emb(1.0);
    q[0] = 1.0;
    q[1] = 0.0;
    let results = store
        .query_filtered(&q, 10, None, Some(&mf))
        .await
        .expect("query");
    assert!(!results.is_empty());
    assert!(
        results.iter().all(|r| r.score.is_finite()),
        "scores must be finite"
    );
    // Scores are 1 - distance; must be non-increasing when reorder is on.
    for w in results.windows(2) {
        assert!(
            w[0].score + 1e-5 >= w[1].score,
            "scores must be non-increasing: {} then {}",
            w[0].score,
            w[1].score
        );
    }
}

#[tokio::test]
async fn e2e_spec090_ann_off_query_path() {
    let src =
        fs::read_to_string("crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs")
            .or_else(|_| fs::read_to_string("src/adapters/postgres/vector/storage_impl.rs"))
            .expect("storage_impl");
    // query_filtered must not call ensure_hot_workspace_ann anymore.
    let qf_start = src.find("async fn query_filtered").expect("query_filtered");
    let qf_body = &src[qf_start..qf_start + 2500.min(src.len() - qf_start)];
    assert!(
        !qf_body.contains("ensure_hot_workspace_ann"),
        "query_filtered must not create ANN indexes (SPEC-090 F-090-05)"
    );
    assert!(
        qf_body.contains("workspace_probe_cache") || qf_body.contains("partial_ann_index_exists"),
        "query_filtered should use cache / exists probe only"
    );
}

#[tokio::test]
async fn e2e_spec090_upsert_chunk_commit() {
    let Some(config) = require_or_skip_postgres("spec090_ups") else {
        return;
    };
    std::env::set_var("EDGEQUAKE_VECTOR_UPSERT_CHUNK", "50");
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    let batch: Vec<_> = (0..120)
        .map(|i| {
            (
                format!("u-{i}"),
                emb(0.02 * i as f32),
                serde_json::json!({"content": format!("chunk {i}"), "type": "chunk"}),
            )
        })
        .collect();
    let created = store.upsert_report_created(&batch).await.expect("upsert");
    assert_eq!(created.len(), 120);

    // Retry converges (idempotent).
    let created2 = store.upsert_report_created(&batch).await.expect("retry");
    assert!(created2.is_empty(), "retry should update not insert");

    let table = format!("eq_{}_vectors", config.table_prefix());
    let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(&raw)
        .await
        .expect("count");
    assert_eq!(n, 120);
    std::env::remove_var("EDGEQUAKE_VECTOR_UPSERT_CHUNK");
}

#[tokio::test]
async fn contract_spec090_no_foreach_row_stats() {
    let src =
        fs::read_to_string("crates/edgequake-storage/src/adapters/postgres/row_count_stats.rs")
            .or_else(|_| fs::read_to_string("src/adapters/postgres/row_count_stats.rs"))
            .expect("row_count_stats");
    assert!(
        src.contains("FOR EACH STATEMENT"),
        "must use STATEMENT triggers"
    );
    assert!(
        !src.contains("FOR EACH ROW"),
        "must not use ROW triggers for stats"
    );
    assert!(src.contains("REFERENCING NEW TABLE") || src.contains("REFERENCING OLD TABLE"));
}
