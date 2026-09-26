//! SPEC-091: typed-only ingest + query without workspace `eq_*_vectors`.
//!
//! Proves chunk + entity embeddings land in typed SSOT tables, legacy table
//! stays absent, and `query_filtered` (chunk) returns hits without 42P01.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec091_typed_only_ingest -- --nocapture
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::adapters::postgres::{PgVectorStorage, PostgresPool};
use edgequake_storage::embedding_family::EmbeddingFamily;
use edgequake_storage::traits::domain::{
    EmbeddingIndex, EmbeddingRow, FleetEmbeddingIndex, FleetEmbeddingKey, FleetEmbeddingRow,
    ModelId, WorkspaceId,
};
use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::VECTOR_BACKEND_ENV;
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use uuid::Uuid;

const DIM: usize = 1536;

async fn table_exists(pool: &sqlx::PgPool, table: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = $1)",
    )
    .bind(table)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn e2e_spec091_typed_only_ingest_and_query() {
    let Some(cfg) = require_or_skip_postgres("spec091_typed_only") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;

    // Fleet tables from migration 130 must exist for entity embeddings.
    for table in [
        "chunk_embeddings",
        "entity_embeddings",
        "relationship_embeddings",
        "report_embeddings",
    ] {
        if !table_exists(&pool, table).await {
            eprintln!("skip: typed table {table} missing — run migrations through 130");
            return;
        }
    }

    let ns = format!("toi_{}", Uuid::new_v4().as_simple());
    std::env::set_var(VECTOR_BACKEND_ENV, "typed_embeddings");
    std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", "typed-only-e2e");

    let mut pg_cfg = cfg.clone();
    pg_cfg.namespace = ns;
    let storage = PgVectorStorage::with_pool(
        PostgresPool::from_existing(pool.clone(), pg_cfg.clone()),
        pg_cfg,
        DIM,
    );
    let legacy_table = storage
        .vectors_table_name()
        .trim_start_matches("public.")
        .to_string();
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "precondition: {legacy_table} must be absent"
    );

    storage
        .initialize()
        .await
        .expect("typed initialize must not CREATE legacy table");
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "typed initialize must leave legacy table absent"
    );

    let ws = w3::seed_workspace(&pool, "typed-only").await;
    let doc = w3::seed_document(&pool, ws).await;
    let chunk_id = w3::seed_chunk(&pool, doc, ws, 0, "typed only chunk").await;
    let emb = w3::make_embedding(DIM, 42);

    let chunk_index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "typed-only-e2e");
    chunk_index
        .upsert_batch(
            ModelId(Uuid::nil()),
            &[EmbeddingRow {
                chunk_id: chunk_id.into(),
                workspace_id: WorkspaceId::new(ws),
                dimensions: DIM as i32,
                embedding: emb.clone(),
            }],
        )
        .await
        .expect("typed chunk upsert");
    // SPEC-149 typed reads apply the serving fence; direct upserts must open it.
    sqlx::query(
        "INSERT INTO public.chunk_serving_state (chunk_id, state) VALUES ($1, 'ready') \
         ON CONFLICT (chunk_id) DO UPDATE SET state = EXCLUDED.state",
    )
    .bind(chunk_id)
    .execute(&pool)
    .await
    .expect("mark typed chunk serving-ready");

    let entity_name = format!("ENTITY_{}", Uuid::new_v4().as_simple());
    let entity_id: Uuid = sqlx::query_scalar(
        "INSERT INTO entities (name, entity_type, workspace_id) VALUES ($1, 'concept', $2) \
         RETURNING id",
    )
    .bind(&entity_name)
    .bind(ws)
    .fetch_one(&pool)
    .await
    .expect("seed entity");

    let fleet = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "typed-only-e2e");
    fleet
        .upsert_batch(
            EmbeddingFamily::Entity,
            ModelId(Uuid::nil()),
            &[FleetEmbeddingRow {
                key: FleetEmbeddingKey::Entity(entity_id),
                workspace_id: WorkspaceId::new(ws),
                dimensions: DIM as i32,
                embedding: emb.clone(),
                legacy_vector_id: None,
            }],
        )
        .await
        .expect("typed entity fleet upsert");

    let chunk_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chunk_embeddings WHERE workspace_id = $1")
            .bind(ws)
            .fetch_one(&pool)
            .await
            .expect("chunk_embeddings count");
    let entity_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entity_embeddings WHERE workspace_id = $1")
            .bind(ws)
            .fetch_one(&pool)
            .await
            .expect("entity_embeddings count");
    assert!(chunk_count > 0, "chunk_embeddings must have rows");
    assert!(entity_count > 0, "entity_embeddings must have rows");
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "legacy {legacy_table} must still be absent after typed ingest"
    );

    let mf = MetadataFilter::from_tenant_workspace_type(None, Some(ws.to_string()), "chunk")
        .expect("filter");
    let hits = storage
        .query_filtered(&emb, 5, None, Some(&mf))
        .await
        .expect("query_filtered must not 42P01 under typed");
    assert!(
        !hits.is_empty(),
        "typed chunk query_filtered must return hits"
    );

    // Mutate wipe path also soft-succeeds without legacy table.
    storage
        .delete_by_document(&doc.to_string())
        .await
        .expect("delete_by_document write-stop");
    storage
        .clear_workspace(&ws)
        .await
        .expect("clear_workspace write-stop");

    let _ = sqlx::query("DELETE FROM entity_embeddings WHERE workspace_id = $1")
        .bind(ws)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM entities WHERE workspace_id = $1")
        .bind(ws)
        .execute(&pool)
        .await;
    w3::cleanup_workspace(&pool, ws).await;
    std::env::remove_var(VECTOR_BACKEND_ENV);
    std::env::remove_var("EDGEQUAKE_EMBEDDING_MODEL");
}
