//! SPEC-111 residual harden — provenance-only LAW-C3 parity.
//!
//! E2E-111-10..17: advisor GREEN iff 131 provenance guard would pass;
//! dataful 131 DROP / ABORT; stall reporting.
//!
//! Run:
//!   EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 DATABASE_URL=… \
//!   cargo test -p edgequake-storage --features postgres \
//!     --test e2e_spec111_provenance_parity -- --nocapture
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::embedding_family::EmbeddingFamily;
use edgequake_storage::entity_id::normalize_entity_name;
use edgequake_storage::migration_engine::advisor;
use edgequake_storage::migration_engine::coverage::{
    count_provenance_stall_rows, count_uncovered_fleet_rows, sample_provenance_stall_ids,
};
use edgequake_storage::migration_engine::fleet_provenance_stamp::FleetProvenanceStampJob;
use edgequake_storage::migration_engine::BackfillJob;
use edgequake_storage::traits::domain::{
    FleetEmbeddingIndex, FleetEmbeddingKey, FleetEmbeddingRow, ModelId, WorkspaceId,
};
use edgequake_storage::PgFleetEmbeddingIndex;
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use sqlx::PgPool;
use uuid::Uuid;

const DIM: usize = 1536;
const MIGRATION_131: &str = include_str!("../../../migrations/131_spec091_fleet_vector_drop.sql");
/// Must match advisor `vector_verify_fleet` default model env.
const VERIFY_MODEL: &str = "text-embedding-3-small";

async fn ensure_143(pool: &PgPool) {
    let _ = sqlx::raw_sql(include_str!(
        "../../../migrations/143_spec111_legacy_vector_id.sql"
    ))
    .execute(pool)
    .await;
}

async fn seed_entity_display(pool: &PgPool, ws: Uuid, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO public.entities (id, name, workspace_id, entity_type, description) \
         VALUES ($1, $2, $3, 'ORG', '') ON CONFLICT DO NOTHING",
    )
    .bind(id)
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

async fn set_backend_typed(pool: &PgPool) {
    // Advisor reads EDGEQUAKE_VECTOR_BACKEND env — set for retirable checks.
    std::env::set_var("EDGEQUAKE_VECTOR_BACKEND", "typed_embeddings");
    std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", VERIFY_MODEL);
    let _ = pool; // posture also probes schema
}

async fn run_stamp_to_completion(pool: &PgPool) {
    let job = FleetProvenanceStampJob::new();
    let mut cursor = job.initial_cursor();
    loop {
        let mut tx = pool.begin().await.expect("begin");
        let outcome = job
            .run_batch(&mut tx, &cursor, 16)
            .await
            .expect("stamp batch");
        tx.commit().await.expect("commit");
        match outcome.next_cursor {
            Some(next) => cursor = next,
            None => break,
        }
    }
}

/// Count entity rows without provenance — mirrors 131 provenance-only guard.
async fn guard_uncovered_entity(pool: &PgPool, table: &str) -> i64 {
    let sql = format!(
        "SELECT count(*) FROM public.{table} v \
         WHERE v.id LIKE 'entity:%' \
           AND NOT EXISTS ( \
                SELECT 1 FROM public.entity_embeddings ee \
                WHERE ee.legacy_vector_id = v.id)"
    );
    sqlx::query_scalar(&sql).fetch_one(pool).await.unwrap_or(0)
}

/// Workspace-scoped uncovered count (avoids unique-legacy_vector_id pollution).
async fn guard_uncovered_entity_ws(pool: &PgPool, table: &str, workspace_id: Uuid) -> i64 {
    let sql = format!(
        "SELECT count(*) FROM public.{table} v \
         WHERE v.id LIKE 'entity:%' \
           AND v.metadata->>'workspace_id' = $1 \
           AND NOT EXISTS ( \
                SELECT 1 FROM public.entity_embeddings ee \
                WHERE ee.legacy_vector_id = v.id \
                  AND ee.workspace_id = $2::uuid)"
    );
    sqlx::query_scalar(&sql)
        .bind(workspace_id.to_string())
        .bind(workspace_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

async fn table_exists(pool: &PgPool, table: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = $1
        )",
    )
    .bind(table)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

/// Sole-table fleet fixture: typed + legacy entity, no chunk rows, no provenance yet.
async fn seed_fleet_green_base(
    pool: &PgPool,
    ns: &str,
    display: &str,
    emb_seed: u32,
) -> (Uuid, Uuid, String, String, Vec<f32>) {
    let ws = w3::seed_workspace(pool, ns).await;
    let eid = seed_entity_display(pool, ws, display).await;
    let table = w3::create_vectors_table(pool, ns).await;
    w3::drop_all_vector_tables_except(pool, &table).await;
    let legacy_id = format!("entity:{}", normalize_entity_name(display));
    let emb = w3::make_embedding(DIM, emb_seed);
    sqlx::query(&format!(
        "INSERT INTO public.{table} (id, embedding, metadata) VALUES ($1, $2::vector, $3)"
    ))
    .bind(&legacy_id)
    .bind(w3::vector_to_text(&emb))
    .bind(serde_json::json!({"workspace_id": ws.to_string()}))
    .execute(pool)
    .await
    .expect("legacy");

    let model_id: Uuid = sqlx::query_scalar(
        "INSERT INTO embedding_models (name, dimensions) VALUES ($1, $2) \
         ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(VERIFY_MODEL)
    .bind(DIM as i32)
    .fetch_one(pool)
    .await
    .expect("model");
    sqlx::query(
        "INSERT INTO entity_embeddings (model_id, entity_id, workspace_id, embedding, dimensions) \
         VALUES ($1, $2, $3, $4::halfvec, $5) \
         ON CONFLICT (model_id, entity_id) DO UPDATE SET embedding = EXCLUDED.embedding",
    )
    .bind(model_id)
    .bind(eid)
    .bind(ws)
    .bind(w3::vector_to_text(&emb))
    .bind(DIM as i32)
    .execute(pool)
    .await
    .expect("typed");

    (ws, eid, table, legacy_id, emb)
}

/// E2E-111-10: display-name + typed embed without provenance → not fleet_retirable.
#[tokio::test]
async fn e2e_spec111_10_no_provenance_not_fleet_retirable() {
    let Some(cfg) = require_or_skip_postgres("spec111_p10") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    w3::clear_w3_job(&pool).await;
    ensure_143(&pool).await;
    set_backend_typed(&pool).await;

    let ws = w3::seed_workspace(&pool, "p10").await;
    let display = "Acme Corp Ltd";
    let eid = seed_entity_display(&pool, ws, display).await;
    assert_eq!(normalize_entity_name(display), "ACME_CORP_LTD");

    let table = w3::create_vectors_table(&pool, "p10").await;
    let legacy_id = "entity:ACME_CORP_LTD";
    let emb = w3::make_embedding(DIM, 11);
    sqlx::query(&format!(
        "INSERT INTO public.{table} (id, embedding, metadata) VALUES ($1, $2::vector, $3)"
    ))
    .bind(legacy_id)
    .bind(w3::vector_to_text(&emb))
    .bind(serde_json::json!({"workspace_id": ws.to_string()}))
    .execute(&pool)
    .await
    .expect("legacy");

    // Typed embed WITHOUT provenance (simulates pre-stamp / old serving upsert).
    let model_id: Uuid = sqlx::query_scalar(
        "INSERT INTO embedding_models (name, dimensions) VALUES ($1, $2) \
         ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind("spec111-p10")
    .bind(DIM as i32)
    .fetch_one(&pool)
    .await
    .expect("model");
    sqlx::query(
        "INSERT INTO entity_embeddings (model_id, entity_id, workspace_id, embedding, dimensions) \
         VALUES ($1, $2, $3, $4::halfvec, $5) \
         ON CONFLICT (model_id, entity_id) DO NOTHING",
    )
    .bind(model_id)
    .bind(eid)
    .bind(ws)
    .bind(w3::vector_to_text(&emb))
    .bind(DIM as i32)
    .execute(&pool)
    .await
    .expect("typed without provenance");

    let uncovered = count_uncovered_fleet_rows(&pool).await.expect("uncovered");
    assert!(
        uncovered >= 1,
        "normalize-without-provenance must count uncovered"
    );
    assert!(
        guard_uncovered_entity_ws(&pool, &table, ws).await >= 1,
        "131 provenance-only guard must also see uncovered"
    );

    // Force verify pass for retirable gate if needed — still must fail on uncovered.
    std::env::set_var("EDGEQUAKE_MIGRATION_VERIFY_EQUALITY", "0");
    let posture = advisor::posture(&pool).await.expect("posture");
    assert!(
        !posture.vector.fleet_retirable(),
        "fleet_retirable must be false without provenance (uncovered={})",
        posture.vector.uncovered_fleet_rows
    );
    assert!(posture.vector.uncovered_fleet_rows >= 1);

    w3::drop_table(&pool, &table).await;
    w3::cleanup_workspace(&pool, ws).await;
}

/// E2E-111-11: after provenance-stamp → fleet_retirable with legacy_fleet_rows>0.
#[tokio::test]
async fn e2e_spec111_11_stamp_makes_fleet_retirable() {
    let Some(cfg) = require_or_skip_postgres("spec111_p11") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    w3::clear_w3_job(&pool).await;
    ensure_143(&pool).await;
    set_backend_typed(&pool).await;

    let display = format!(
        "Beta Industries Inc T{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let (ws, _eid, table, _legacy_id, _emb) =
        seed_fleet_green_base(&pool, "p11", &display, 22).await;

    assert!(guard_uncovered_entity_ws(&pool, &table, ws).await >= 1);
    run_stamp_to_completion(&pool).await;
    assert_eq!(
        guard_uncovered_entity_ws(&pool, &table, ws).await,
        0,
        "131 guard must be 0 after stamp"
    );

    std::env::set_var("EDGEQUAKE_MIGRATION_VERIFY_EQUALITY", "0");
    let posture = advisor::posture(&pool).await.expect("posture");
    assert_eq!(posture.vector.uncovered_fleet_rows, 0);
    assert_eq!(posture.vector.uncovered_chunk_rows, 0);
    assert!(
        posture.vector.legacy_fleet_rows > 0,
        "legacy rows must still exist pre-drop"
    );
    assert!(
        posture.vector.fleet_retirable(),
        "sole-table + stamp + typed backend must make fleet_retirable (verify_fleet={:?})",
        posture.vector.verify_fleet
    );

    w3::drop_table(&pool, &table).await;
    w3::cleanup_workspace(&pool, ws).await;
}

/// E2E-111-12: mirror INSERT stamps legacy_vector_id.
#[tokio::test]
async fn e2e_spec111_12_mirror_stamps_provenance() {
    let Some(cfg) = require_or_skip_postgres("spec111_p12") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    ensure_143(&pool).await;

    let ws = w3::seed_workspace(&pool, "p12").await;
    let display = "Gamma LLC";
    let eid = seed_entity_display(&pool, ws, display).await;
    let legacy_id = "entity:GAMMA_LLC";
    let emb = w3::make_embedding(DIM, 33);
    let index = PgFleetEmbeddingIndex::new(pool.clone(), "spec111-p12");
    let report = index
        .mirror_legacy_batch(
            &[(
                legacy_id.to_string(),
                emb,
                serde_json::json!({"workspace_id": ws.to_string()}),
            )],
            true,
            None,
        )
        .await
        .expect("mirror");
    assert!(report.resolved >= 1, "mirror must resolve normalize join");

    let stamped: Option<String> = sqlx::query_scalar(
        "SELECT legacy_vector_id FROM entity_embeddings WHERE entity_id = $1 LIMIT 1",
    )
    .bind(eid)
    .fetch_optional(&pool)
    .await
    .expect("select");
    assert_eq!(
        stamped.as_deref(),
        Some(legacy_id),
        "mirror INSERT must stamp legacy_vector_id"
    );

    w3::cleanup_workspace(&pool, ws).await;
}

/// E2E-111-13: missing workspace_id → uncovered (not false-GREEN).
#[tokio::test]
async fn e2e_spec111_13_missing_workspace_uncovered() {
    let Some(cfg) = require_or_skip_postgres("spec111_p13") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    ensure_143(&pool).await;

    let ws = w3::seed_workspace(&pool, "p13").await;
    let _eid = seed_entity_display(&pool, ws, "Delta Co").await;
    let table = w3::create_vectors_table(&pool, "p13").await;
    let emb = w3::make_embedding(DIM, 44);
    // No workspace_id in metadata — and no provenance.
    sqlx::query(&format!(
        "INSERT INTO public.{table} (id, embedding, metadata) VALUES ($1, $2::vector, '{{}}')"
    ))
    .bind(format!(
        "entity:DELTA_CO_T{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
    .bind(w3::vector_to_text(&emb))
    .execute(&pool)
    .await
    .expect("legacy");

    let uncovered = count_uncovered_fleet_rows(&pool).await.expect("count");
    assert!(
        uncovered >= 1,
        "missing workspace + no provenance must stay uncovered (got {uncovered})"
    );
    // 131 provenance-only: also uncovered (no exact-name false-GREEN).
    assert!(guard_uncovered_entity(&pool, &table).await >= 1);

    w3::drop_table(&pool, &table).await;
    w3::cleanup_workspace(&pool, ws).await;
}

/// E2E-111-14: dual legacy → one entity; second remains uncovered / stamp failed.
#[tokio::test]
async fn e2e_spec111_14_dual_legacy_second_uncovered() {
    let Some(cfg) = require_or_skip_postgres("spec111_p14") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    ensure_143(&pool).await;

    let ws = w3::seed_workspace(&pool, "p14").await;
    let eid = seed_entity_display(&pool, ws, "Epsilon Corp").await;
    let table = w3::create_vectors_table(&pool, "p14").await;
    w3::drop_all_vector_tables_except(&pool, &table).await;
    let emb = w3::make_embedding(DIM, 55);
    // Both keys normalize-resolve to "Epsilon Corp" → true dual-legacy stall.
    for legacy_id in ["entity:EPSILON_CORP", "entity:The Epsilon Corp"] {
        sqlx::query(&format!(
            "INSERT INTO public.{table} (id, embedding, metadata) VALUES ($1, $2::vector, $3)"
        ))
        .bind(legacy_id)
        .bind(w3::vector_to_text(&emb))
        .bind(serde_json::json!({"workspace_id": ws.to_string()}))
        .execute(&pool)
        .await
        .expect("legacy");
    }
    // One typed row
    let model_id: Uuid = sqlx::query_scalar(
        "INSERT INTO embedding_models (name, dimensions) VALUES ($1, $2) \
         ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(VERIFY_MODEL)
    .bind(DIM as i32)
    .fetch_one(&pool)
    .await
    .expect("model");
    sqlx::query(
        "INSERT INTO entity_embeddings (model_id, entity_id, workspace_id, embedding, dimensions) \
         VALUES ($1, $2, $3, $4::halfvec, $5)",
    )
    .bind(model_id)
    .bind(eid)
    .bind(ws)
    .bind(w3::vector_to_text(&emb))
    .bind(DIM as i32)
    .execute(&pool)
    .await
    .expect("typed");

    let job = FleetProvenanceStampJob::new();
    let mut cursor = job.initial_cursor();
    let mut failed = 0i64;
    loop {
        let mut tx = pool.begin().await.expect("begin");
        let outcome = job.run_batch(&mut tx, &cursor, 16).await.expect("batch");
        tx.commit().await.expect("commit");
        failed += outcome.failed;
        match outcome.next_cursor {
            Some(n) => cursor = n,
            None => break,
        }
    }
    let uncovered = guard_uncovered_entity_ws(&pool, &table, ws).await;
    assert!(
        uncovered >= 1 || failed >= 1,
        "second legacy key must remain uncovered or stamp-failed (uncovered={uncovered}, failed={failed})"
    );

    let stalls = count_provenance_stall_rows(&pool).await.expect("stalls");
    let stall_ids = sample_provenance_stall_ids(&pool, 32)
        .await
        .expect("stall sample");
    assert!(
        stalls >= 1,
        "dual-legacy must surface provenance_stall_rows>=1 (got {stalls}, failed={failed}, ids={stall_ids:?})"
    );
    assert!(!stall_ids.is_empty(), "stall sample must be non-empty");

    std::env::set_var("EDGEQUAKE_VECTOR_BACKEND", "typed_embeddings");
    let posture = advisor::posture(&pool).await.expect("posture");
    assert!(
        posture.vector.provenance_stall_rows >= 1,
        "posture must expose stall count"
    );
    let actions = advisor::derive_actions(&posture);
    let fleet_gate = actions
        .iter()
        .find(|a| a.verb == "drop" && a.target == "vector-fleet")
        .expect("vector-fleet action");
    assert!(!fleet_gate.enabled);
    let reason = fleet_gate.gate_reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("stall"),
        "advisor must mention dual-legacy stall (got {reason})"
    );

    let report = FleetProvenanceStampJob::new()
        .verify(&pool)
        .await
        .expect("stamp verify");
    assert!(
        report.metric.contains("stalls="),
        "stamp verify metric must include stalls= (got {})",
        report.metric
    );
    assert!(report.mismatches >= 1);

    w3::drop_table(&pool, &table).await;
    w3::cleanup_workspace(&pool, ws).await;
}

/// E2E-111-15: chunk retirable can differ from fleet retirable.
#[tokio::test]
async fn e2e_spec111_15_chunk_vs_fleet_retirable_distinct() {
    let types = include_str!("../src/migration_engine/advisor/types.rs");
    assert!(types.contains("fn fleet_retirable"));
    assert!(types.contains("fn chunk_retirable"));
    assert!(types.contains("uncovered_fleet_rows == 0"));

    let console = include_str!("../../../src/migrate_console.rs");
    assert!(
        console.contains("vector-chunk drop-readiness")
            && console.contains("vector-fleet drop-readiness"),
        "console must separate chunk vs fleet GREEN"
    );
    assert!(
        console.contains("iw2-fleet-provenance-stamp"),
        "console NEXT must mention provenance-stamp"
    );

    let rules = include_str!("../src/migration_engine/advisor/rules.rs");
    assert!(rules.contains("iw2-fleet-provenance-stamp"));
    assert!(rules.contains("dual-legacy stall"));
    assert!(types.contains("verify_fleet"));
    assert!(types.contains("provenance_stall_rows"));

    let stamp = include_str!("../src/migration_engine/fleet_provenance_stamp.rs");
    assert!(
        stamp.contains("count_stamp_verify_coverage")
            || stamp.contains("stampable_provenance_coverage"),
        "stamp verify must be stampable-only (not all fleet rows)"
    );
    let vec_src = include_str!("../src/adapters/postgres/vector/storage_impl.rs");
    assert!(
        !vec_src.contains(
            "async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {\n        if crate::legacy_vector_writes_stopped() {\n            return Ok(0);"
        ),
        "clear_workspace must not no-op under typed write-stop"
    );

    let _ = EmbeddingFamily::Entity;
    let _ = (
        ModelId(Uuid::nil()),
        WorkspaceId::new(Uuid::nil()),
        FleetEmbeddingKey::Entity(Uuid::nil()),
    );
    let _ = std::mem::size_of::<FleetEmbeddingRow>();
}

/// E2E-111-16: GREEN fleet fixture → migration 131 DROP succeeds; typed survives.
#[tokio::test]
async fn e2e_spec111_16_confirm_drop_fleet_after_provenance() {
    let Some(cfg) = require_or_skip_postgres("spec111_p16") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    w3::clear_w3_job(&pool).await;
    ensure_143(&pool).await;
    set_backend_typed(&pool).await;

    let display = format!(
        "Zeta Holdings PLC T{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let (ws, eid, table, _legacy_id, _emb) =
        seed_fleet_green_base(&pool, "p16", &display, 66).await;
    run_stamp_to_completion(&pool).await;
    assert_eq!(guard_uncovered_entity_ws(&pool, &table, ws).await, 0);

    std::env::set_var("EDGEQUAKE_MIGRATION_VERIFY_EQUALITY", "0");
    let posture = advisor::posture(&pool).await.expect("posture");
    assert!(
        posture.vector.fleet_retirable(),
        "must be fleet_retirable before 131 (verify_fleet={:?})",
        posture.vector.verify_fleet
    );

    sqlx::raw_sql(MIGRATION_131)
        .execute(&pool)
        .await
        .expect("migration 131 applies once provenance-covered");

    assert!(
        !table_exists(&pool, &table).await,
        "fleet table must be dropped after 131"
    );
    let typed_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entity_embeddings WHERE entity_id = $1")
            .bind(eid)
            .fetch_one(&pool)
            .await
            .expect("typed");
    assert_eq!(typed_after, 1, "typed SSOT must survive fleet drop");
    let uncovered = count_uncovered_fleet_rows(&pool).await.expect("uncovered");
    assert_eq!(uncovered, 0);

    w3::cleanup_workspace(&pool, ws).await;
}

/// E2E-111-17: without stamp, migration 131 ABORTs on missing provenance.
#[tokio::test]
async fn e2e_spec111_17_abort_without_provenance() {
    let Some(cfg) = require_or_skip_postgres("spec111_p17") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    w3::clear_w3_job(&pool).await;
    ensure_143(&pool).await;
    set_backend_typed(&pool).await;

    let (ws, _eid, table, _legacy_id, _emb) =
        seed_fleet_green_base(&pool, "p17", "Eta Partners LLC", 77).await;
    assert!(guard_uncovered_entity_ws(&pool, &table, ws).await >= 1);

    let err = sqlx::raw_sql(MIGRATION_131)
        .execute(&pool)
        .await
        .expect_err("131 must ABORT without provenance");
    let msg = err.to_string();
    assert!(
        msg.contains("SPEC-091 IW2 ABORT") || msg.contains("legacy_vector_id"),
        "ABORT must cite IW2 / provenance (got {msg})"
    );
    assert!(
        table_exists(&pool, &table).await,
        "table must remain after ABORT"
    );

    w3::drop_table(&pool, &table).await;
    w3::cleanup_workspace(&pool, ws).await;
}
