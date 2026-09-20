//! SPEC-136 / #377 — durable NULL-lid loser PK: sequential merger retry absorbs stamp UPDATE 23505.
//!
//! Fixture (cannot fake):
//! - unique index `idx_entity_embeddings_legacy_vector_id` still exists
//! - winner display name `"John Smith"` owns lid `entity:JOHN_SMITH`
//! - loser exact name `"JOHN_SMITH"` has a typed PK row with `legacy_vector_id IS NULL`
//! - merge of `"John Smith"` resolves to the **losing** FK (exact-name UNIQUE + resolve)
//!
//! Bound: `KnowledgeGraphMerger::merge` twice, `errors == 0`, one lid owner, loser does
//! not steal the lid. HTTP worker dual-doc soak is **not** claimed.
#![cfg(feature = "postgres")]

#[path = "common/test_db.rs"]
mod test_db;

use edgequake_api::postgres_entity_sink::PostgresEntitySink;
use edgequake_pipeline::{
    ExtractedEntity, ExtractionResult, KnowledgeGraphMerger, MergerConfig, RelationalEntitySink,
};
use edgequake_storage::embedding_family::EmbeddingFamily;
use edgequake_storage::traits::{
    FleetEmbeddingIndex, FleetEmbeddingKey, FleetEmbeddingRow, ModelId, WorkspaceId,
};
use edgequake_storage::{
    MemoryGraphStorage, MemoryVectorStorage, PgFleetEmbeddingIndex, VectorStorage,
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

const DIM: usize = 1024;
const LID: &str = "entity:JOHN_SMITH";

fn postgres_tests_required() -> bool {
    matches!(
        std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

fn require_db() -> Option<String> {
    let required = postgres_tests_required();
    let Some(base) = std::env::var("DATABASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
    else {
        if required {
            panic!(
                "DATABASE_URL required when EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 (SPEC-136 / #377)"
            );
        }
        return None;
    };
    Some(test_db::isolated_test_url(&base))
}

async fn connect_pool(url: &str) -> Option<PgPool> {
    match PgPool::connect(url).await {
        Ok(pool) => Some(pool),
        Err(e) if postgres_tests_required() => {
            panic!(
                "postgres connect failed (SPEC-136 / #377, EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1): {e}"
            );
        }
        Err(e) => {
            eprintln!("SKIP: cannot connect to DATABASE_URL: {e}");
            None
        }
    }
}

fn extraction(doc: &str, chunk: &str, seed: f32) -> ExtractionResult {
    let mut entity = ExtractedEntity::new("John Smith", "PERSON", format!("from {doc}"))
        .with_importance(0.9)
        .with_source_chunk_id(chunk)
        .with_source_document_id(doc);
    entity.embedding = Some(vec![seed; DIM]);
    let mut result = ExtractionResult::new(chunk);
    result.add_entity(entity);
    result
}

async fn insert_entity(pool: &PgPool, id: Uuid, name: &str, tenant: Uuid, workspace: Uuid) {
    sqlx::query(
        "INSERT INTO entities (id, name, entity_type, description, tenant_id, workspace_id, sync_status) \
         VALUES ($1, $2, 'PERSON', 'x', $3, $4, 'synced')",
    )
    .bind(id)
    .bind(name)
    .bind(tenant)
    .bind(workspace)
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("insert entity {name}: {e}"));
}

async fn insert_null_lid_clone(pool: &PgPool, winner_fk: Uuid, loser_fk: Uuid) {
    let inserted = sqlx::query(
        "INSERT INTO entity_embeddings \
         (model_id, entity_id, workspace_id, embedding, dimensions, legacy_vector_id) \
         SELECT model_id, $1, workspace_id, embedding, dimensions, NULL \
         FROM entity_embeddings WHERE entity_id = $2 LIMIT 1",
    )
    .bind(loser_fk)
    .bind(winner_fk)
    .execute(pool)
    .await
    .expect("NULL-lid clone")
    .rows_affected();
    assert_eq!(inserted, 1, "expected one NULL-lid entity_embeddings clone");
}

async fn assert_legacy_unique_index(pool: &PgPool) {
    let def: Option<String> = sqlx::query_scalar(
        "SELECT pg_get_indexdef('idx_entity_embeddings_legacy_vector_id'::regclass)",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or_else(|e| panic!("idx_entity_embeddings_legacy_vector_id lookup failed: {e}"));
    let def = def.unwrap_or_else(|| {
        panic!(
            "idx_entity_embeddings_legacy_vector_id must exist (dropped index cannot fake green)"
        )
    });
    let lower = def.to_lowercase();
    assert!(
        lower.contains("legacy_vector_id"),
        "unique index must still cover legacy_vector_id: {def}"
    );
    assert!(
        lower.contains("workspace_id"),
        "unique index must remain workspace-scoped (migration 144): {def}"
    );
}

#[tokio::test]
async fn e2e_spec136_durable_lid_retry_sequential_merger_absorbs() {
    let Some(url) = require_db() else {
        eprintln!("SKIP: DATABASE_URL unset");
        return;
    };
    std::env::set_var("EDGEQUAKE_VECTOR_BACKEND", "typed_embeddings");

    let Some(pool) = connect_pool(&url).await else {
        return;
    };
    assert_legacy_unique_index(&pool).await;

    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant)
        .bind(format!("t-{tenant}"))
        .bind(format!("t-{tenant}"))
        .execute(&pool)
        .await
        .expect("tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind(format!("w-{workspace}"))
    .bind(format!("w-{workspace}"))
    .execute(&pool)
    .await
    .expect("workspace");

    // Loser FIRST so EntityNameIndex::resolve("JOHN_SMITH") binds the losing FK
    // (or_insert keeps oldest; winner's normalized key must not occupy JOHN_SMITH).
    let loser = Uuid::new_v4();
    let winner = Uuid::new_v4();
    insert_entity(&pool, loser, "JOHN_SMITH", tenant, workspace).await;
    insert_entity(&pool, winner, "John Smith", tenant, workspace).await;

    let graph = Arc::new(MemoryGraphStorage::new(format!(
        "spec136-merger-{workspace}"
    )));
    let vector = Arc::new(MemoryVectorStorage::new(
        format!("spec136-merger-{workspace}"),
        DIM,
    ));
    vector.initialize().await.expect("vector init");

    let sink: Arc<dyn RelationalEntitySink> =
        Arc::new(PostgresEntitySink::new_fail_closed(Arc::new(pool.clone())));
    let fleet: Arc<dyn FleetEmbeddingIndex> = Arc::new(PgFleetEmbeddingIndex::new(
        pool.clone(),
        format!("e2e-spec136-merger-{workspace}"),
    ));

    fleet
        .upsert_batch(
            EmbeddingFamily::Entity,
            ModelId(Uuid::nil()),
            &[FleetEmbeddingRow {
                workspace_id: WorkspaceId::new(workspace),
                embedding: vec![0.11; DIM],
                dimensions: DIM as i32,
                key: FleetEmbeddingKey::Entity(winner),
                legacy_vector_id: Some(LID.to_string()),
            }],
        )
        .await
        .expect("winner lid stamp");
    insert_null_lid_clone(&pool, winner, loser).await;

    let mk_merger = || {
        KnowledgeGraphMerger::new(MergerConfig::default(), graph.clone(), vector.clone())
            .with_tenant_context(Some(tenant.to_string()), Some(workspace.to_string()))
            .with_relational_sink(sink.clone())
            .with_fleet_embedding_index(fleet.clone())
    };

    let first = mk_merger()
        .merge(vec![extraction("doc-retry-1", "doc-retry-1-chunk-0", 0.31)])
        .await
        .expect("first merge must not return Err");
    assert_eq!(
        first.errors, 0,
        "first merge must absorb stamp 23505, not GraphMerge: first_error={:?}",
        first.first_error
    );
    if let Some(err) = first.first_error.as_deref() {
        assert!(
            !err.contains("23505")
                && !err.contains("GraphMerge")
                && !err.contains("legacy_vector_id"),
            "first_error must not name lid unique / GraphMerge: {err}"
        );
    }

    let second = mk_merger()
        .merge(vec![extraction("doc-retry-2", "doc-retry-2-chunk-0", 0.32)])
        .await
        .expect("retry merge must not return Err");
    assert_eq!(
        second.errors, 0,
        "retry must absorb again (durable, not a one-shot race): first_error={:?}",
        second.first_error
    );

    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM entity_embeddings \
         WHERE workspace_id = $1 AND legacy_vector_id = $2",
    )
    .bind(workspace)
    .bind(LID)
    .fetch_one(&pool)
    .await
    .expect("lid owners");
    assert_eq!(owners, 1, "exactly one legacy_vector_id owner after retry");

    let winner_owns: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM entity_embeddings \
         WHERE entity_id = $1 AND legacy_vector_id = $2)",
    )
    .bind(winner)
    .bind(LID)
    .fetch_one(&pool)
    .await
    .expect("winner owns");
    assert!(winner_owns, "winner must keep the lid (LAW-120-2)");

    let loser_stole: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM entity_embeddings \
         WHERE entity_id = $1 AND legacy_vector_id = $2)",
    )
    .bind(loser)
    .bind(LID)
    .fetch_one(&pool)
    .await
    .expect("loser stole");
    assert!(!loser_stole, "loser PK must not steal the lid");
}
