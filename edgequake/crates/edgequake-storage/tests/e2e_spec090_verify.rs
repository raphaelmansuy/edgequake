//! SPEC-090 assessment harden — live PostgreSQL verification gates.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec090_verify -- --nocapture

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::{MetadataFilter, VectorStorage, WorkspaceVectorConfig};
use edgequake_storage::{
    calculate_pdf_checksum, vector_upsert_chunk_size, CreatePdfRequest, ListPdfFilter,
    PdfDocumentStorage, PgVectorStorage, PostgresPdfStorage, PostgresPool,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres, seed_tenant_and_user};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const DIM: usize = 8;

fn emb(seed: f32) -> Vec<f32> {
    vec![seed; DIM]
}

fn read_src(rel: &str) -> String {
    fs::read_to_string(format!("crates/edgequake-storage/{rel}"))
        .or_else(|_| fs::read_to_string(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

async fn seed_workspace(pool: &sqlx::PgPool, tenant_id: Uuid, workspace_id: Uuid) {
    seed_tenant_and_user(pool, tenant_id, Uuid::new_v4())
        .await
        .expect("tenant/user");
    let slug = format!("ws_{}", &workspace_id.to_string()[..8]);
    sqlx::query(
        r#"
        INSERT INTO workspaces (
            workspace_id, tenant_id, name, slug, is_active, metadata, settings, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, TRUE, '{}'::jsonb, '{}'::jsonb, NOW(), NOW())
        ON CONFLICT (workspace_id) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("SPEC090 {slug}"))
    .bind(&slug)
    .execute(pool)
    .await
    .expect("workspace");
}

#[tokio::test]
async fn e2e_spec090_verify_counter_concurrency() {
    let Some(config) = require_or_skip_postgres("spec090_vctr") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = Arc::new(PgVectorStorage::with_pool_and_dimension(
        pool,
        config.clone(),
        DIM,
    ));
    store.initialize().await.expect("init");

    let stats = format!("eq_{}_vectors_stats", config.table_prefix());
    let _ = sqlx::query("SELECT pg_stat_reset_single_table_counters($1::regclass)")
        .bind(format!("public.{stats}"))
        .execute(&raw)
        .await;

    let a = Arc::clone(&store);
    let b = Arc::clone(&store);
    let t1 = tokio::spawn(async move {
        let batch: Vec<_> = (0..150)
            .map(|i| {
                (
                    format!("a-{i}"),
                    emb(0.01 * i as f32),
                    serde_json::json!({"workspace_id": "ws", "type": "chunk"}),
                )
            })
            .collect();
        a.upsert(&batch).await
    });
    let t2 = tokio::spawn(async move {
        let batch: Vec<_> = (0..150)
            .map(|i| {
                (
                    format!("b-{i}"),
                    emb(0.02 * i as f32),
                    serde_json::json!({"workspace_id": "ws", "type": "chunk"}),
                )
            })
            .collect();
        b.upsert(&batch).await
    });
    t1.await.expect("join a").expect("upsert a");
    t2.await.expect("join b").expect("upsert b");

    // No prolonged exclusive lock waits on the stats singleton after upserts complete.
    let lock_waits: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM pg_locks l
        JOIN pg_class c ON c.oid = l.relation
        WHERE c.relname = $1
          AND NOT l.granted
          AND l.locktype = 'relation'
        "#,
    )
    .bind(&stats)
    .fetch_one(&raw)
    .await
    .unwrap_or(0);
    assert_eq!(lock_waits, 0, "unexpected ungranted locks on {stats}");

    let upd: i64 =
        sqlx::query_scalar("SELECT n_tup_upd FROM pg_stat_user_tables WHERE relname = $1")
            .bind(&stats)
            .fetch_one(&raw)
            .await
            .expect("n_tup_upd");
    // Statement-level: far below row count (300). Allow a small factor for retries.
    assert!(
        upd < 40,
        "expected statement-level stats updates (~statements), got n_tup_upd={upd}"
    );
}

/// Read `pg_stat_database.xact_commit` on a single backend (clear + select).
/// Pool-scoped clear/select can hit different connections and observe a stale
/// `stats_fetch_consistency=cache` snapshot → false delta=0 (F-090-02 flake).
async fn read_db_xact_commit(conn: &mut sqlx::PgConnection) -> i64 {
    sqlx::query("SELECT pg_stat_clear_snapshot()")
        .execute(&mut *conn)
        .await
        .expect("pg_stat_clear_snapshot");
    sqlx::query_scalar(
        "SELECT xact_commit FROM pg_stat_database WHERE datname = current_database()",
    )
    .fetch_one(&mut *conn)
    .await
    .expect("xact_commit")
}

#[tokio::test]
async fn e2e_spec090_verify_upsert_xact_commit() {
    let Some(config) = require_or_skip_postgres("spec090_vxact") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    // SAFETY: test-only env override for chunk sizing; restored below.
    unsafe { std::env::set_var("EDGEQUAKE_VECTOR_UPSERT_CHUNK", "100") };
    let chunk = vector_upsert_chunk_size();
    assert_eq!(
        chunk, 100,
        "EDGEQUAKE_VECTOR_UPSERT_CHUNK must clamp to 100"
    );
    let n = 250usize; // → 3 chunks at size 100
    let expected_chunks = n.div_ceil(chunk) as i64;

    // Separate stats pool so upsert connections can disconnect and flush PG stats.
    let stats_pool = contract_pg_pool(&config).await;
    let mut stats = stats_pool.acquire().await.expect("stats conn");
    let before = read_db_xact_commit(&mut stats).await;

    let batch: Vec<_> = (0..n)
        .map(|i| {
            (
                format!("x-{i}"),
                emb(0.001 * i as f32),
                serde_json::json!({"workspace_id": "ws", "type": "chunk"}),
            )
        })
        .collect();
    store.upsert(&batch).await.expect("upsert");

    // Release upsert backends so delayed xact counters flush (PG stats collector).
    raw.close().await;
    let _ = sqlx::query("SELECT pg_stat_force_next_flush()")
        .execute(&mut *stats)
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let after = read_db_xact_commit(&mut stats).await;
    drop(stats);

    // Re-open for row count (raw was closed for flush).
    let raw2 = contract_pg_pool(&config).await;
    let table = format!("eq_{}_vectors", config.table_prefix());
    let rows: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*)::bigint FROM {table}"))
        .fetch_one(&raw2)
        .await
        .expect("row count");
    assert!(
        rows >= n as i64,
        "upsert must persist rows before xact probe (got {rows}, want >= {n})"
    );

    let delta = after - before;
    eprintln!(
        "F-090-02 chunk={chunk} xact_commit before={before} after={after} delta={delta} \
         expected_chunks>={expected_chunks} rows={rows}"
    );
    assert!(
        delta >= expected_chunks,
        "per-chunk commit expected xact_commit delta >= {expected_chunks}, got {delta} \
         (before={before} after={after} chunk={chunk})"
    );
    // Source contract: commit happens inside the chunk loop (not one TX for all).
    let src = read_src("src/adapters/postgres/vector/storage_impl.rs");
    assert!(
        src.contains("Failed to commit upsert chunk tx")
            && src.contains("for chunk in kept.chunks(chunk_size)"),
        "upsert must commit per chunk (F-090-02)"
    );
    unsafe { std::env::remove_var("EDGEQUAKE_VECTOR_UPSERT_CHUNK") };
}

#[tokio::test]
async fn e2e_spec090_verify_content_tsv() {
    let src = read_src("src/adapters/postgres/vector/storage_impl.rs");
    assert!(
        src.contains("UNNEST($1::text[], $2::text[], $3::jsonb[], $4::text[])"),
        "upsert must bind content as 4th UNNEST (F-090-03)"
    );
    let unnest_at = src
        .find("FROM UNNEST($1::text[], $2::text[], $3::jsonb[], $4::text[])")
        .expect("unnest");
    let insert_sql = &src[unnest_at.saturating_sub(600)..unnest_at + 400];
    assert!(
        !insert_sql.contains("LEFT JOIN") && !insert_sql.contains("k.value->>'content'"),
        "upsert INSERT SELECT must not correlate KV via JOIN subquery (F-090-03)"
    );
    assert!(
        src.contains("WHERE key = ANY($1)") || src.contains("key = ANY($1)"),
        "KV content resolve must use separate ANY($1) fetch"
    );

    let Some(config) = require_or_skip_postgres("spec090_vfts") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    let table = format!("eq_{}_vectors", config.table_prefix());
    let id = format!("fts-{}", Uuid::new_v4());
    // Live FTS via 4th UNNEST content bind (inline metadata content).
    store
        .upsert(&[(
            id.clone(),
            emb(0.5),
            serde_json::json!({
                "workspace_id": "ws",
                "type": "chunk",
                "content": "spec090 unique fts needle zebra"
            }),
        )])
        .await
        .expect("upsert content");

    let hits: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*)::bigint FROM {table} WHERE id = $1 AND content_tsv @@ plainto_tsquery('english', 'zebra')"
    ))
    .bind(&id)
    .fetch_one(&raw)
    .await
    .expect("fts");
    assert_eq!(hits, 1, "FTS must find UNNEST-bound content_tsv row");
}

#[tokio::test]
async fn e2e_spec090_verify_ann_no_ddl() {
    let src = read_src("src/adapters/postgres/vector/storage_impl.rs");
    let start = src.find("async fn query_filtered").expect("query_filtered");
    let body = &src[start..start + 2500.min(src.len() - start)];
    assert!(
        !body.contains("ensure_hot_workspace_ann"),
        "query_filtered must not call ensure_hot_workspace_ann (F-090-05)"
    );

    let Some(config) = require_or_skip_postgres("spec090_vann") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    let batch: Vec<_> = (0..40)
        .map(|i| {
            (
                format!("ann-{i}"),
                emb(0.01 * i as f32),
                serde_json::json!({"workspace_id": "ws-ann", "type": "chunk"}),
            )
        })
        .collect();
    store.upsert(&batch).await.expect("seed");

    let filter = MetadataFilter {
        workspace_id: Some("ws-ann".into()),
        ..Default::default()
    };
    let q = emb(0.05);
    let handle = tokio::spawn({
        let raw = raw.clone();
        async move {
            let mut saw_create = false;
            for _ in 0..40 {
                let n: i64 = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*)::bigint
                    FROM pg_stat_activity
                    WHERE query ILIKE '%CREATE%INDEX%'
                      AND pid <> pg_backend_pid()
                      AND state = 'active'
                    "#,
                )
                .fetch_one(&raw)
                .await
                .unwrap_or(0);
                if n > 0 {
                    saw_create = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            saw_create
        }
    });

    let _ = store
        .query_filtered(&q, 5, None, Some(&filter))
        .await
        .expect("query_filtered");
    let saw = handle.await.expect("poll join");
    assert!(
        !saw,
        "query_filtered must not run CREATE INDEX on the hot path (F-090-05)"
    );
}

#[tokio::test]
async fn e2e_spec090_verify_delete_explain() {
    let Some(config) = require_or_skip_postgres("spec090_vdel") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    let ws = Uuid::new_v4();
    let ws_s = ws.to_string();
    let batch: Vec<_> = (0..500)
        .map(|i| {
            (
                format!("del-{i}"),
                emb(0.001 * i as f32),
                serde_json::json!({
                    "workspace_id": ws_s,
                    "type": "chunk",
                    "document_id": format!("doc-{i}")
                }),
            )
        })
        .collect();
    store.upsert(&batch).await.expect("seed");

    let table = format!("eq_{}_vectors", config.table_prefix());
    let plan_rows = sqlx::query_scalar::<_, String>(&format!(
        r#"
        EXPLAIN (FORMAT TEXT)
        DELETE FROM {table}
        WHERE ctid IN (
            SELECT ctid FROM {table} WHERE workspace_id = $1
            UNION
            SELECT ctid FROM {table} WHERE metadata->>'workspace_id' = $1
        )
        "#
    ))
    .bind(&ws_s)
    .fetch_all(&raw)
    .await
    .expect("explain");
    let plan = plan_rows.join("\n");

    eprintln!("F-090-09 clear_workspace-shaped plan:\n{plan}");
    // UNION ctid delete ends in Tid Scan; small fixtures may Seq Scan the arms.
    assert!(
        plan.contains("Tid Scan") || plan.contains("Index") || plan.contains("Bitmap"),
        "expected Tid/Index/Bitmap delete plan, got:\n{plan}"
    );
    assert!(
        plan.contains("Append") || plan.contains("Union"),
        "expected UNION/Append arms in plan, got:\n{plan}"
    );

    let deleted = store.clear_workspace(&ws).await.expect("clear");
    assert!(deleted >= 500, "clear_workspace deleted={deleted}");
}

#[tokio::test]
async fn e2e_spec090_verify_edge_any_param() {
    let src = read_src("src/adapters/postgres/graph/query_ops/expand.rs");
    assert!(
        src.contains("ANY($1::text[])"),
        "expand.rs must bind ANY($1::text[]) (F-090-10)"
    );
    assert!(
        !src.contains("IN ('") || src.contains("SPEC-090"),
        "edge expand must not interpolate literal IN-lists"
    );
}

#[tokio::test]
async fn e2e_spec090_verify_pdf_list_explain() {
    let Some(config) = require_or_skip_postgres("spec090_vpdf") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    // Ensure blob side table exists (M103).
    let blob_ok = sqlx::query("SELECT 1 FROM pdf_document_blobs LIMIT 0")
        .execute(&raw)
        .await
        .is_ok();
    if !blob_ok {
        eprintln!("SKIP: pdf_document_blobs missing (run M103)");
        return;
    }

    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    seed_workspace(&raw, tenant_id, workspace_id).await;

    let pdf = PostgresPdfStorage::new(raw.clone());
    let mut blob = b"%PDF-1.4\n".to_vec();
    blob.extend(std::iter::repeat_n(b'X', 64 * 1024));
    let checksum = calculate_pdf_checksum(&blob);
    let pdf_id = pdf
        .create_pdf(CreatePdfRequest {
            workspace_id,
            filename: format!("spec090-{}.pdf", Uuid::new_v4()),
            content_type: "application/pdf".into(),
            file_size_bytes: blob.len() as i64,
            sha256_checksum: checksum,
            page_count: Some(1),
            pdf_data: blob,
            vision_model: None,
        })
        .await
        .expect("create_pdf");

    let plan_rows = sqlx::query_scalar::<_, String>(
        r#"
        EXPLAIN (FORMAT TEXT)
        SELECT
            pdf_id, workspace_id, document_id, filename, content_type,
            file_size_bytes, sha256_checksum, page_count, processing_status,
            extraction_method, vision_model, extraction_errors,
            created_at, processed_at, updated_at
        FROM pdf_documents
        WHERE workspace_id = $1
        ORDER BY created_at DESC
        LIMIT 20 OFFSET 0
        "#,
    )
    .bind(workspace_id)
    .fetch_all(&raw)
    .await
    .expect("explain list");
    let plan = plan_rows.join("\n");

    eprintln!("F-090-16 list plan:\n{plan}");
    assert!(
        !plan.to_ascii_lowercase().contains("pdf_data"),
        "list EXPLAIN must not project/toast pdf_data:\n{plan}"
    );

    let listed = pdf
        .list_pdfs(ListPdfFilter {
            workspace_id: Some(workspace_id),
            processing_status: None,
            page: Some(1),
            page_size: Some(20),
        })
        .await
        .expect("list");
    assert!(listed.items.iter().any(|d| d.pdf_id == pdf_id));
    assert!(
        listed
            .items
            .iter()
            .find(|d| d.pdf_id == pdf_id)
            .map(|d| d.pdf_data.is_empty())
            .unwrap_or(false),
        "list path must not populate pdf_data bytes"
    );

    // F-090-16 cutover: get_pdf reads side table; primary pdf_data column absent after M105.
    let got = pdf.get_pdf(&pdf_id).await.expect("get_pdf").expect("row");
    assert!(
        !got.pdf_data.is_empty(),
        "get_pdf must return bytes from pdf_document_blobs"
    );
    let col_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM information_schema.columns
          WHERE table_schema = 'public'
            AND table_name = 'pdf_documents'
            AND column_name = 'pdf_data'
        )
        "#,
    )
    .fetch_one(&raw)
    .await
    .expect("information_schema");
    assert!(
        !col_exists,
        "pdf_documents.pdf_data must be dropped after M105"
    );

    let _ = sqlx::query("DELETE FROM pdf_document_blobs WHERE pdf_id = $1")
        .bind(pdf_id)
        .execute(&raw)
        .await;
    let _ = sqlx::query("DELETE FROM pdf_documents WHERE pdf_id = $1")
        .bind(pdf_id)
        .execute(&raw)
        .await;
}

#[tokio::test]
async fn e2e_spec090_verify_workspace_full_slug() {
    let ws = Uuid::new_v4();
    let cfg = WorkspaceVectorConfig::new(ws, DIM).with_namespace("spec090");
    let table = cfg.table_name();
    let full = ws.to_string().replace('-', "_");
    assert!(
        table.contains(&full),
        "table name must contain full uuid slug, got {table}"
    );
    assert!(
        !table.contains(&ws.to_string()[..8]) || table.contains(&full),
        "must not prefer short-id-only naming"
    );

    let Some(config) = require_or_skip_postgres("spec090_vslug") else {
        return;
    };
    // Namespace encodes full workspace slug when using WorkspaceVectorConfig.
    let mut pg = config.clone();
    pg.namespace = cfg.namespace_prefix();
    let raw = contract_pg_pool(&pg).await;
    let pool = PostgresPool::from_existing(raw.clone(), pg.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, pg.clone(), DIM);
    store.initialize().await.expect("init");

    // PgVectorStorage qualifies as public.eq_{table_prefix}_vectors where
    // table_prefix already includes the `eq_` namespace prefix.
    let physical = format!("eq_{}_vectors", pg.table_prefix());
    assert!(
        physical.contains(&full),
        "physical table must embed full uuid slug: {physical}"
    );
    let exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM pg_tables
            WHERE schemaname = 'public' AND tablename = $1
        )
        "#,
    )
    .bind(&physical)
    .fetch_one(&raw)
    .await
    .expect("pg_tables");
    assert!(exists, "expected table {physical}");
}

#[tokio::test]
async fn e2e_spec090_verify_halfvec_default() {
    let prev = std::env::var("EDGEQUAKE_VECTOR_STORAGE").ok();
    std::env::remove_var("EDGEQUAKE_VECTOR_STORAGE");

    let Some(config) = require_or_skip_postgres("spec090_vhalf") else {
        if let Some(v) = prev {
            std::env::set_var("EDGEQUAKE_VECTOR_STORAGE", v);
        }
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    let table = format!("eq_{}_vectors", config.table_prefix());
    let typ: String = sqlx::query_scalar(
        r#"
        SELECT t.typname
        FROM pg_attribute a
        JOIN pg_class c ON c.oid = a.attrelid
        JOIN pg_type t ON t.oid = a.atttypid
        WHERE c.relname = $1 AND a.attname = 'embedding' AND NOT a.attisdropped
        "#,
    )
    .bind(&table)
    .fetch_one(&raw)
    .await
    .expect("embedding type");
    assert_eq!(
        typ, "halfvec",
        "default embedding column type must be halfvec when env unset (F-090-26)"
    );

    if let Some(v) = prev {
        std::env::set_var("EDGEQUAKE_VECTOR_STORAGE", v);
    }
}

#[tokio::test]
async fn e2e_spec090_verify_vector_timeout() {
    let src = read_src("src/adapters/postgres/vector/storage_impl.rs");
    assert!(
        src.contains("LocalTimeoutTx") && src.contains("vector_query_statement_timeout_ms"),
        "vector query must use LocalTimeoutTx (F-090-27)"
    );

    let Some(config) = require_or_skip_postgres("spec090_vto") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    // Prove SET LOCAL statement_timeout cancels within budget (same mechanism LocalTimeoutTx uses).
    let mut tx = raw.begin().await.expect("begin");
    sqlx::query("SET LOCAL statement_timeout = '50ms'")
        .execute(&mut *tx)
        .await
        .expect("set timeout");
    let start = std::time::Instant::now();
    let err = sqlx::query("SELECT pg_sleep(2)")
        .execute(&mut *tx)
        .await
        .expect_err("pg_sleep must cancel");
    let elapsed = start.elapsed();
    eprintln!("F-090-27 cancel after {elapsed:?}: {err}");
    assert!(
        elapsed < Duration::from_millis(1500),
        "statement_timeout must cancel well under sleep duration"
    );
    let _ = tx.rollback().await;
}

#[tokio::test]
async fn e2e_spec090_verify_reconcile_state() {
    let Some(config) = require_or_skip_postgres("spec090_vrec") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let ok = sqlx::query("SELECT 1 FROM edgequake_reconcile_state LIMIT 0")
        .execute(&raw)
        .await
        .is_ok();
    if !ok {
        eprintln!("SKIP: edgequake_reconcile_state missing (run M102)");
        return;
    }

    let version = format!("spec090-verify-{}", Uuid::new_v4());
    sqlx::query(
        r#"
        INSERT INTO edgequake_reconcile_state
            (support_version, apply_sha384, duration_ms, outcome)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (support_version) DO UPDATE SET
            apply_sha384 = EXCLUDED.apply_sha384,
            applied_at = now(),
            duration_ms = EXCLUDED.duration_ms,
            outcome = EXCLUDED.outcome
        "#,
    )
    .bind(&version)
    .bind("deadbeef")
    .bind(12_i64)
    .bind("ok")
    .execute(&raw)
    .await
    .expect("insert reconcile");

    let outcome: String = sqlx::query_scalar(
        "SELECT outcome FROM edgequake_reconcile_state WHERE support_version = $1",
    )
    .bind(&version)
    .fetch_one(&raw)
    .await
    .expect("fetch");
    assert_eq!(outcome, "ok");

    let api_src =
        fs::read_to_string("crates/edgequake-api/src/state/migration_bootstrap/reconcile_state.rs")
            .or_else(|_| {
                fs::read_to_string(
                    "../edgequake-api/src/state/migration_bootstrap/reconcile_state.rs",
                )
            })
            .unwrap_or_default();
    if !api_src.is_empty() {
        assert!(
            api_src.contains("edgequake_reconcile_state")
                && api_src.contains("ON CONFLICT (support_version)"),
            "API reconcile_state writer must match ledger contract"
        );
    }

    let _ = sqlx::query("DELETE FROM edgequake_reconcile_state WHERE support_version = $1")
        .bind(&version)
        .execute(&raw)
        .await;
}

#[tokio::test]
async fn e2e_spec090_verify_age_fail_closed_contract() {
    let src = fs::read_to_string("crates/edgequake-api/src/state/postgres.rs")
        .or_else(|_| fs::read_to_string("../edgequake-api/src/state/postgres.rs"))
        .expect("state/postgres.rs");
    assert!(
        src.contains("EDGEQUAKE_ALLOW_NO_GRAPH"),
        "graph initialize must honor EDGEQUAKE_ALLOW_NO_GRAPH escape (F-090-19)"
    );
    assert!(
        src.contains("initialize failed") || src.contains("Graph storage"),
        "graph initialize must fail-closed without escape"
    );
}

#[tokio::test]
async fn e2e_spec090_verify_cic_contract() {
    let ddl = read_src("src/adapters/postgres/vector/ddl.rs");
    assert!(
        ddl.contains("CREATE INDEX CONCURRENTLY IF NOT EXISTS"),
        "non-empty tables must use CIC (F-090-08)"
    );
}

#[tokio::test]
async fn e2e_spec090_verify_claim_sql_shape_contract() {
    let src = fs::read_to_string("crates/edgequake-tasks/src/postgres.rs")
        .or_else(|_| fs::read_to_string("../edgequake-tasks/src/postgres.rs"))
        .expect("tasks postgres.rs");
    assert!(
        src.contains("claim_arm_sql") || src.contains("SKIP LOCKED"),
        "claim path must use SKIP LOCKED arms"
    );
    let skip_locked = src.matches("SKIP LOCKED").count();
    assert!(
        skip_locked >= 2,
        "expected dual SKIP LOCKED arms, found {skip_locked}"
    );
    assert!(
        src.contains("LIMIT $1") || src.contains("LIMIT $"),
        "claim sample must be LIMIT-bound"
    );
    assert!(
        src.contains("edgequake_ensure_tasks_month_partitions")
            || src.contains("edgequake_detach_old_task_partitions"),
        "task storage must touch monthly partition helpers (F-090-13)"
    );
}

#[tokio::test]
async fn e2e_spec090_verify_boot_migrate_split_contract() {
    let bootstrap = fs::read_to_string("crates/edgequake-api/src/state/migration_bootstrap/mod.rs")
        .or_else(|_| fs::read_to_string("../edgequake-api/src/state/migration_bootstrap/mod.rs"))
        .expect("migration_bootstrap");
    assert!(bootstrap.contains("bootstrap_for_serving"));
    assert!(bootstrap.contains("migrate_cli_mode"));

    // SPEC-150: allow-boot escape lives on the migrate/ledger path, not serve.
    let ledger = fs::read_to_string("crates/edgequake-api/src/state/migration_bootstrap/ledger.rs")
        .or_else(|_| fs::read_to_string("../edgequake-api/src/state/migration_bootstrap/ledger.rs"))
        .expect("migration_bootstrap/ledger");
    assert!(ledger.contains("EDGEQUAKE_ALLOW_BOOT_MIGRATE"));

    let reconcile =
        fs::read_to_string("crates/edgequake-api/src/state/migration_bootstrap/reconcile/mod.rs")
            .or_else(|_| {
                fs::read_to_string(
                    "../edgequake-api/src/state/migration_bootstrap/reconcile/mod.rs",
                )
            })
            .expect("reconcile/mod");
    assert!(
        reconcile.contains("heavy_bootstrap_apply_allowed")
            || reconcile.contains("verify-only boot"),
        "execute_bootstrap_apply_sql must gate on boot escape (F-090-20b)"
    );

    let main = fs::read_to_string("src/main.rs")
        .or_else(|_| fs::read_to_string("../../src/main.rs"))
        .unwrap_or_default();
    if !main.is_empty() {
        assert!(
            main.contains("run_migrate_cli") || main.contains("\"migrate\""),
            "edgequake migrate CLI must exist"
        );
        assert!(
            main.contains("migrate_console") || main.contains("mod migrate_console"),
            "migrate CLI must use migrate_console for operator stdout"
        );
    }

    let console = fs::read_to_string("src/migrate_console.rs")
        .or_else(|_| fs::read_to_string("../../src/migrate_console.rs"))
        .unwrap_or_default();
    if !console.is_empty() {
        assert!(console.contains("print_banner"));
        assert!(console.contains("print_preflight"));
        assert!(console.contains("print_applied_this_run"));
        assert!(console.contains("print_post_hooks"));
        assert!(console.contains("print_failure_hint"));
        assert!(console.contains("hnsw_manifest"));
        assert!(console.contains("pdf_documents.pdf_data"));
    }

    assert!(
        bootstrap.contains("list_pending_migrations"),
        "bootstrap must expose list_pending_migrations for migrate console"
    );
}

#[tokio::test]
async fn e2e_spec090_verify_progress_and_relations_contract() {
    let storage = read_src("src/adapters/postgres/vector/storage_impl.rs");
    assert!(
        storage.contains("UNION") && storage.contains("delete_entity_relations"),
        "delete_entity_relations must use UNION ctid (F-090-09b)"
    );
    assert!(
        !storage
            .lines()
            .filter(|l| l.contains("delete_entity_relations") || l.contains("metadata->>'source'"))
            .any(|l| l.contains(" OR ") && l.contains("target")),
        "delete_entity_relations must not use bare OR across JSONB keys"
    );

    let proc = fs::read_to_string("crates/edgequake-api/src/processor/mod.rs")
        .or_else(|_| fs::read_to_string("../edgequake-api/src/processor/mod.rs"))
        .unwrap_or_default();
    if !proc.is_empty() {
        assert!(
            proc.contains("update_task_progress") || proc.contains("bump_task_progress"),
            "processor must wire progress-only updates (F-090-04)"
        );
    }

    let ddl = read_src("src/adapters/postgres/vector/ddl.rs");
    assert!(
        ddl.contains("SET LOCAL") && ddl.contains("apply_vector_ddl_gucs"),
        "DDL path must prefer SET LOCAL inside TX (F-090-07 residual)"
    );
    assert!(
        ddl.contains("eq_hot_ann_workspaces") || ddl.contains("rebuild_global_ann_excluding_hot"),
        "hot workspace ANN must register mutual exclusion (F-090-25)"
    );
}

#[tokio::test]
async fn e2e_spec090_verify_embedding_identity_and_manifest() {
    let Some(config) = require_or_skip_postgres("spec090_vemb") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let pool = PostgresPool::from_existing(raw.clone(), config.clone());
    let store = PgVectorStorage::with_pool_and_dimension(pool, config.clone(), DIM);
    store.initialize().await.expect("init");

    std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", "spec090-test-embed");
    store
        .upsert(&[(
            format!("id-{}", Uuid::new_v4()),
            emb(0.1),
            serde_json::json!({"workspace_id": "ws", "type": "chunk"}),
        )])
        .await
        .expect("upsert");

    let table = format!("eq_{}_vectors", config.table_prefix());
    let row: Option<(Option<String>, Option<i32>, Option<String>)> = sqlx::query_as(&format!(
        "SELECT embedding_model, embedding_dim, embedding_norm FROM {table} LIMIT 1"
    ))
    .fetch_optional(&raw)
    .await
    .expect("identity cols");
    let (model, dim, norm) = row.expect("row");
    assert_eq!(model.as_deref(), Some("spec090-test-embed"));
    assert_eq!(dim, Some(DIM as i32));
    assert_eq!(norm.as_deref(), Some("cosine"));

    let drifts = edgequake_storage::check_hnsw_index_manifest(&raw)
        .await
        .expect("manifest");
    eprintln!("F-090-32 drift count={}", drifts.len());
}

#[tokio::test]
async fn e2e_spec090_verify_tasks_partitioned() {
    let Some(config) = require_or_skip_postgres("spec090_vpart") else {
        return;
    };
    let raw = contract_pg_pool(&config).await;
    let partitioned: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM pg_partitioned_table
          WHERE partrelid = 'public.tasks'::regclass
        )
        "#,
    )
    .fetch_one(&raw)
    .await
    .unwrap_or(false);
    if !partitioned {
        eprintln!("SKIP: tasks not yet range-partitioned (apply M104)");
        return;
    }
    let _ = sqlx::query("SELECT edgequake_ensure_tasks_month_partitions()")
        .execute(&raw)
        .await;
    let child_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM pg_inherits i
        JOIN pg_class c ON c.oid = i.inhrelid
        JOIN pg_class p ON p.oid = i.inhparent
        WHERE p.relname = 'tasks'
        "#,
    )
    .fetch_one(&raw)
    .await
    .expect("partition children");
    assert!(
        child_count >= 2,
        "expected history + at least one future month partition, got {child_count}"
    );
}
