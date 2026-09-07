//! SPEC-396 residuals on top of SPEC-139: alias-pair fleet batch, SQL
//! DISTINCT ON belt, W3 missing-spine honesty, 21000 cursor hold.
//!
//! Run:
//!   DATABASE_URL=… cargo test -p edgequake-storage --features postgres \
//!     --test e2e_spec396_engine -- --nocapture --test-threads=1
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::entity_id::normalize_entity_name;
use edgequake_storage::migration_engine::advisor;
use edgequake_storage::migration_engine::chunk_embedding_backfill::ChunkEmbeddingBackfillJob;
use edgequake_storage::migration_engine::coverage;
use edgequake_storage::migration_engine::fleet_embedding_backfill::FleetEmbeddingBackfillJob;
use edgequake_storage::migration_engine::BackfillJob;
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

const DIM: usize = 1536;

const DROP_125: &str = include_str!("../../../migrations/125_spec091_kv_drop.sql");
const DROP_126: &str = include_str!("../../../migrations/126_spec091_vector_drop.sql");
const DROP_131: &str = include_str!("../../../migrations/131_spec091_fleet_vector_drop.sql");

async fn run_to_completion(pool: &PgPool, job: &dyn BackfillJob) {
    let mut cursor = job.initial_cursor();
    for _ in 0..64 {
        let mut tx = pool.begin().await.expect("begin");
        let outcome = job.run_batch(&mut tx, &cursor, 64).await.expect("batch");
        tx.commit().await.expect("commit");
        match outcome.next_cursor {
            Some(next) => cursor = next,
            None => return,
        }
    }
    panic!("backfill exceeded 64 batches (possible held-cursor spin)");
}

async fn seed_entity(pool: &PgPool, ws: Uuid, name: &str) -> Uuid {
    sqlx::query(
        "INSERT INTO public.entities (id, name, workspace_id, entity_type, description) \
         VALUES ($1, $2, $3, 'ORG', '') ON CONFLICT DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(name)
    .bind(ws)
    .execute(pool)
    .await
    .expect("seed entity");
    sqlx::query_scalar("SELECT id FROM public.entities WHERE name = $1 AND workspace_id = $2")
        .bind(name)
        .bind(ws)
        .fetch_one(pool)
        .await
        .expect("entity id")
}

async fn ensure_legacy_vector_id(pool: &PgPool) {
    let _ = sqlx::raw_sql(include_str!(
        "../../../migrations/143_spec111_legacy_vector_id.sql"
    ))
    .execute(pool)
    .await;
}

async fn upsert_model(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO embedding_models (name, dimensions) VALUES ($1, $2) \
         ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(name)
    .bind(DIM as i32)
    .fetch_one(pool)
    .await
    .expect("model")
}

fn assert_21000(result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>) {
    match result {
        Err(sqlx::Error::Database(db)) => {
            assert_eq!(
                db.code().as_deref(),
                Some("21000"),
                "broken UNNEST must raise cardinality 21000, got {:?}",
                db.code()
            );
        }
        other => panic!("expected SQLSTATE 21000, got {other:?}"),
    }
}

/// DROP 125 / 126 / 131 bodies stay fail-closed (no this-issue edit).
#[test]
fn contract_spec396_drop_sql_unchanged_fail_closed() {
    assert!(
        DROP_125.contains("ABORT") && DROP_125.contains("un-migrated durable"),
        "125 must stay fail-closed"
    );
    assert!(
        DROP_126.contains("SPEC-091 W4 ABORT")
            && DROP_126.contains("NOT EXISTS")
            && DROP_126.contains("chunk_embeddings"),
        "126 must stay fail-closed on uncovered chunks"
    );
    assert!(
        DROP_131.contains("SPEC-091 IW2 ABORT") && DROP_131.contains("legacy_vector_id"),
        "131 must stay fail-closed on missing provenance"
    );
}

/// Prod-like: two `eq_*_vectors` tables + alias pair in one keyset batch.
#[tokio::test]
async fn e2e_spec396_01_iw2_alias_pair_two_tables_one_batch() {
    let Some(cfg) = require_or_skip_postgres("spec396_01") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    ensure_legacy_vector_id(&pool).await;
    w3::drop_all_vector_tables_except(&pool, "__none__").await;

    let ws = w3::seed_workspace(&pool, "e2e39601").await;
    let display = "Acme Corp Ltd";
    let eid = seed_entity(&pool, ws, display).await;
    assert_eq!(normalize_entity_name(display), "ACME_CORP_LTD");
    let other = seed_entity(&pool, ws, "Other Org").await;

    let shared = w3::create_vectors_table(&pool, "e2e39601a").await;
    let workspace = w3::create_vectors_table(&pool, "e2e39601b").await;
    let meta = json!({"workspace_id": ws.to_string(), "entity_type": "ORG"});
    for (i, key) in ["entity:Acme Corp Ltd", "entity:ACME_CORP_LTD"]
        .iter()
        .enumerate()
    {
        sqlx::query(&format!(
            "INSERT INTO public.{shared} (id, embedding, metadata) VALUES ($1, $2::vector, $3)"
        ))
        .bind(*key)
        .bind(w3::vector_to_text(&w3::make_embedding(DIM, 20 + i as u32)))
        .bind(&meta)
        .execute(&pool)
        .await
        .expect("seed colliding aliases on shared table");
    }
    sqlx::query(&format!(
        "INSERT INTO public.{workspace} (id, embedding, metadata) VALUES ($1, $2::vector, $3)"
    ))
    .bind("entity:Other Org")
    .bind(w3::vector_to_text(&w3::make_embedding(DIM, 99)))
    .bind(&meta)
    .execute(&pool)
    .await
    .expect("seed second fleet table");

    sqlx::query("DELETE FROM entity_embeddings WHERE entity_id = ANY($1::uuid[])")
        .bind(&[eid, other][..])
        .execute(&pool)
        .await
        .ok();

    let job = FleetEmbeddingBackfillJob::new("spec396-iw2-model".into());
    run_to_completion(&pool, &job).await;

    let typed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.entity_embeddings WHERE entity_id = $1")
            .bind(eid)
            .fetch_one(&pool)
            .await
            .expect("typed count");
    assert_eq!(typed, 1, "alias pair collapses to one typed row");
    let other_typed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.entity_embeddings WHERE entity_id = $1")
            .bind(other)
            .fetch_one(&pool)
            .await
            .expect("other typed");
    assert_eq!(other_typed, 1, "second table entity must copy");

    let report = job.verify(&pool).await.expect("iw2 verify");
    assert!(
        report.actual >= 2,
        "fleet coverage SUM across two tables, got actual={}",
        report.actual
    );
    eprintln!(
        "UNFAKABLE E2E-396-01 tables=2 alias_typed={typed} other_typed={other_typed} \
         verify_actual={}",
        report.actual
    );

    w3::drop_table(&pool, &shared).await;
    w3::drop_table(&pool, &workspace).await;
    w3::cleanup_workspace(&pool, ws).await;
}

/// Raw UNNEST of `(eid, eid)` raises 21000; DISTINCT ON belt commits one row.
#[tokio::test]
async fn e2e_spec396_02_distinct_on_unnest_avoids_21000() {
    let Some(cfg) = require_or_skip_postgres("spec396_02") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    ensure_legacy_vector_id(&pool).await;

    let ws = w3::seed_workspace(&pool, "e2e39602").await;
    let eid = seed_entity(&pool, ws, "Acme Corp Ltd").await;
    let model_id = upsert_model(&pool, "spec396-distinct-on").await;
    let emb = w3::vector_to_text(&w3::make_embedding(DIM, 3));
    let dup_ids = vec![eid, eid];
    let workspaces = vec![ws, ws];
    let lids = vec![
        "entity:Acme Corp Ltd".to_string(),
        "entity:ACME_CORP_LTD".to_string(),
    ];
    let vectors = vec![emb.clone(), emb.clone()];
    let dims = vec![DIM as i32, DIM as i32];

    let mut broken = pool.begin().await.expect("broken tx");
    let raw = sqlx::query(
        "INSERT INTO entity_embeddings \
         (model_id, entity_id, workspace_id, embedding, dimensions, legacy_vector_id) \
         SELECT $1, e, w, v::halfvec, d, lid \
         FROM unnest($2::uuid[], $3::uuid[], $4::text[], $5::int[], $6::text[]) \
           AS t(e, w, v, d, lid) \
         ON CONFLICT (model_id, entity_id) DO UPDATE \
           SET legacy_vector_id = COALESCE(entity_embeddings.legacy_vector_id, EXCLUDED.legacy_vector_id)",
    )
    .bind(model_id)
    .bind(&dup_ids)
    .bind(&workspaces)
    .bind(&vectors)
    .bind(&dims)
    .bind(&lids)
    .execute(&mut *broken)
    .await;
    assert_21000(raw);
    broken.rollback().await.ok();

    sqlx::query("DELETE FROM entity_embeddings WHERE entity_id = $1")
        .bind(eid)
        .execute(&pool)
        .await
        .ok();

    let mut belt = pool.begin().await.expect("belt tx");
    sqlx::query(
        "INSERT INTO entity_embeddings \
         (model_id, entity_id, workspace_id, embedding, dimensions, legacy_vector_id) \
         SELECT $1, e, w, v::halfvec, d, lid \
         FROM ( \
           SELECT DISTINCT ON (e) e, w, v, d, lid \
           FROM unnest($2::uuid[], $3::uuid[], $4::text[], $5::int[], $6::text[]) \
             WITH ORDINALITY AS t(e, w, v, d, lid, ord) \
           ORDER BY e, ord DESC \
         ) s \
         ON CONFLICT (model_id, entity_id) DO UPDATE \
           SET legacy_vector_id = COALESCE(entity_embeddings.legacy_vector_id, EXCLUDED.legacy_vector_id)",
    )
    .bind(model_id)
    .bind(&dup_ids)
    .bind(&workspaces)
    .bind(&vectors)
    .bind(&dims)
    .bind(&lids)
    .execute(&mut *belt)
    .await
    .expect("DISTINCT ON belt must not 21000");
    belt.commit().await.expect("belt commit");

    let typed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.entity_embeddings WHERE entity_id = $1")
            .bind(eid)
            .fetch_one(&pool)
            .await
            .expect("typed");
    assert_eq!(typed, 1);
    let lid: Option<String> = sqlx::query_scalar(
        "SELECT legacy_vector_id FROM public.entity_embeddings WHERE entity_id = $1",
    )
    .bind(eid)
    .fetch_one(&pool)
    .await
    .expect("lid");
    assert_eq!(
        lid.as_deref(),
        Some("entity:ACME_CORP_LTD"),
        "ord DESC last-write-wins keeps the later alias"
    );
    eprintln!("UNFAKABLE E2E-396-02 distinct_on_typed=1 last_write=entity:ACME_CORP_LTD");

    sqlx::query("DELETE FROM entity_embeddings WHERE entity_id = $1")
        .bind(eid)
        .execute(&pool)
        .await
        .ok();
    w3::cleanup_workspace(&pool, ws).await;
}

/// Missing `chunks` spine increments failed_count and advisor split.
#[tokio::test]
async fn e2e_spec396_03_w3_missing_spine_failed_count_and_split() {
    let Some(cfg) = require_or_skip_postgres("spec396_03") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    w3::drop_all_vector_tables_except(&pool, "__none__").await;

    let ws = w3::seed_workspace(&pool, "e2e39603").await;
    let doc_ok = w3::seed_document(&pool, ws).await;
    let doc_orphan = w3::seed_document(&pool, ws).await;
    let table_a = w3::create_vectors_table(&pool, "e2e39603a").await;
    let table_b = w3::create_vectors_table(&pool, "e2e39603b").await;

    w3::seed_chunk(&pool, doc_ok, ws, 0, "copyable").await;
    w3::seed_legacy_chunk_vector(&pool, &table_a, doc_ok, 0, &w3::make_embedding(DIM, 1)).await;
    w3::seed_legacy_chunk_vector(&pool, &table_a, doc_orphan, 0, &w3::make_embedding(DIM, 2)).await;
    w3::seed_legacy_chunk_vector(&pool, &table_b, doc_orphan, 1, &w3::make_embedding(DIM, 3)).await;

    let job = ChunkEmbeddingBackfillJob::new(table_a.clone(), "spec396-w3-model".into());
    let mut cursor = job.initial_cursor();
    let mut failed_total = 0i64;
    let mut written_total = 0i64;
    loop {
        let mut tx = pool.begin().await.expect("begin");
        let outcome = job.run_batch(&mut tx, &cursor, 64).await.expect("batch");
        tx.commit().await.expect("commit");
        failed_total += outcome.failed;
        written_total += outcome.written;
        match outcome.next_cursor {
            Some(next) => cursor = next,
            None => break,
        }
    }

    assert_eq!(written_total, 1, "only the copyable spine row writes");
    assert!(
        failed_total >= 2,
        "two orphan {{doc}}-chunk-{{n}} keys must increment failed_count, got {failed_total}"
    );

    let verify = job.verify(&pool).await.expect("verify");
    assert_eq!(
        verify.expected, 3,
        "expected stays all legacy UUID-chunk ids"
    );
    assert_eq!(verify.actual, 1, "actual is per-table coverage SUM");
    assert!(
        !verify.passes(),
        "orphans keep verify fail-closed (DROP 126)"
    );

    let split = coverage::count_uncovered_chunk_split(&pool)
        .await
        .expect("split");
    assert_eq!(split.missing_spine, 2);
    assert_eq!(split.missing_embedding, 0);
    assert_eq!(split.total(), 2);
    assert_eq!(
        coverage::count_uncovered_chunk_rows(&pool)
            .await
            .expect("uncovered"),
        split.total(),
        "split.total ≡ 126 uncovered"
    );

    let posture = advisor::posture(&pool).await.expect("posture");
    assert_eq!(posture.vector.uncovered_chunk_rows, 2);
    assert_eq!(posture.vector.uncovered_chunk_missing_spine_rows, 2);
    assert_eq!(posture.vector.uncovered_chunk_missing_embedding_rows, 0);
    assert!(
        !posture.vector.chunk_retirable(),
        "DROP 126 stays gated on uncovered including missing_spine"
    );

    eprintln!(
        "UNFAKABLE E2E-396-03 written={written_total} failed={failed_total} \
         expected={} actual={} missing_spine={} missing_embedding={}",
        verify.expected, verify.actual, split.missing_spine, split.missing_embedding
    );

    w3::drop_table(&pool, &table_a).await;
    w3::drop_table(&pool, &table_b).await;
    w3::cleanup_workspace(&pool, ws).await;
}

/// W3 verify actual is coverage SUM; equality stays opt-in.
#[tokio::test]
async fn e2e_spec396_04_w3_verify_sum_and_passes_ignores_mismatches() {
    let Some(cfg) = require_or_skip_postgres("spec396_04") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    w3::drop_all_vector_tables_except(&pool, "__none__").await;

    let src = include_str!("../src/migration_engine/chunk_embedding_backfill.rs");
    assert!(
        src.contains("agg.actual += r.actual"),
        "W3 fleet verify must SUM per-table actual"
    );
    assert!(
        !src.contains("agg.actual = agg.actual.max"),
        "W3 must not MAX global typed counts"
    );
    let verify_src = include_str!("../src/migration_engine/verify.rs");
    assert!(
        verify_src.contains("LAW-139-3"),
        "verify_chunk_embedding_backfill must stay per-table coverage"
    );

    let report = edgequake_storage::migration_engine::VerifyReport {
        metric: "w3".into(),
        expected: 4,
        actual: 4,
        sampled: 4,
        mismatches: 2,
    };
    assert!(
        report.passes(),
        "default passes() ignores sampled mismatches"
    );
    let runner = include_str!("../src/migration_engine/runner.rs");
    assert!(
        runner.contains("EDGEQUAKE_MIGRATION_VERIFY_EQUALITY"),
        "equality opt-in env must remain"
    );
    assert!(
        runner.contains("mismatches == 0"),
        "equality opt-in still enforces mismatches == 0"
    );
    eprintln!("UNFAKABLE E2E-396-04 passes_default_ignores_mismatches=1 equality_opt_in=1");

    let _ = pool;
}
