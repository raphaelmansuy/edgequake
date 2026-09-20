//! SPEC-120: same-workspace dual-FK `legacy_vector_id` collisions absorb (LAW-120-1..3).
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::embedding_family::EmbeddingFamily;
use edgequake_storage::traits::{
    FleetEmbeddingIndex, FleetEmbeddingKey, FleetEmbeddingRow, ModelId, WorkspaceId,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use uuid::Uuid;

const DIM: usize = 1024;

fn emb(seed: f32) -> Vec<f32> {
    vec![seed; DIM]
}

async fn seed_tenant_ws(pool: &sqlx::PgPool) -> (Uuid, Uuid) {
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant)
        .bind(format!("t-{tenant}"))
        .bind(format!("t-{tenant}"))
        .execute(pool)
        .await
        .expect("tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4)",
    )
    .bind(workspace)
    .bind(tenant)
    .bind(format!("w-{workspace}"))
    .bind(format!("w-{workspace}"))
    .execute(pool)
    .await
    .expect("workspace");
    (tenant, workspace)
}

async fn insert_entity(pool: &sqlx::PgPool, id: Uuid, name: &str, tenant: Uuid, workspace: Uuid) {
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
    .expect("entity");
}

fn entity_row(ws: Uuid, eid: Uuid, lid: &str, seed: f32) -> FleetEmbeddingRow {
    FleetEmbeddingRow {
        workspace_id: WorkspaceId::new(ws),
        embedding: emb(seed),
        dimensions: DIM as i32,
        key: FleetEmbeddingKey::Entity(eid),
        legacy_vector_id: Some(lid.to_string()),
    }
}

/// T1 / EC-01 / EC-02: two FKs, same lid, concurrent upsert → both Ok; one owner.
#[tokio::test]
async fn contract_spec120_dual_fk_same_lid_concurrent_absorb() {
    let Some(cfg) = require_or_skip_postgres("spec120_dual_fk") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let (tenant, workspace) = seed_tenant_ws(&pool).await;
    let e1 = Uuid::new_v4();
    let e2 = Uuid::new_v4();
    insert_entity(&pool, e1, "JOHN_SMITH", tenant, workspace).await;
    insert_entity(&pool, e2, "John Smith", tenant, workspace).await;

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "spec120-race");
    let lid = "entity:JOHN_SMITH";
    let rows_a = [entity_row(workspace, e1, lid, 0.01)];
    let rows_b = [entity_row(workspace, e2, lid, 0.02)];

    let (r1, r2) = tokio::join!(
        index.upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &rows_a),
        index.upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &rows_b),
    );
    let r1 = r1.expect("winner/loser upsert A must not 23505");
    let r2 = r2.expect("winner/loser upsert B must not 23505");
    assert!(
        r1.absorbed_legacy_collisions + r2.absorbed_legacy_collisions >= 1
            || (r1.upserted >= 1 && r2.upserted >= 1),
        "expected absorb or both stamped; got r1={r1:?} r2={r2:?}"
    );

    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM entity_embeddings \
         WHERE workspace_id = $1 AND legacy_vector_id = $2",
    )
    .bind(workspace)
    .bind(lid)
    .fetch_one(&pool)
    .await
    .expect("count owners");
    assert_eq!(owners, 1, "exactly one typed owner for lid");
}

/// T4: stamp-once — existing non-null lid is not overwritten.
#[tokio::test]
async fn contract_spec120_stamp_once_preserves_existing_lid() {
    let Some(cfg) = require_or_skip_postgres("spec120_stamp_once") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let (tenant, workspace) = seed_tenant_ws(&pool).await;
    let eid = Uuid::new_v4();
    insert_entity(&pool, eid, "STAMP_ONCE", tenant, workspace).await;
    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "spec120-stamp");

    let first = entity_row(workspace, eid, "entity:STAMP_ONCE", 0.03);
    index
        .upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &[first])
        .await
        .expect("first stamp");

    let second = entity_row(workspace, eid, "entity:OTHER_LID", 0.04);
    index
        .upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &[second])
        .await
        .expect("second upsert");

    let lid: Option<String> =
        sqlx::query_scalar("SELECT legacy_vector_id FROM entity_embeddings WHERE entity_id = $1")
            .bind(eid)
            .fetch_one(&pool)
            .await
            .expect("lid");
    assert_eq!(lid.as_deref(), Some("entity:STAMP_ONCE"));
}

/// T3: multi-workspace same lid still allowed (migration 144).
#[tokio::test]
async fn contract_spec120_multi_ws_same_lid_allowed() {
    let Some(cfg) = require_or_skip_postgres("spec120_multi_ws") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let tenant = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant)
        .bind(format!("t-{tenant}"))
        .bind(format!("t-{tenant}"))
        .execute(&pool)
        .await
        .expect("tenant");

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "spec120-multi-ws");
    let lid = "entity:SHARED_NAME";
    for _ in 0..2 {
        let workspace = Uuid::new_v4();
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
        let eid = Uuid::new_v4();
        insert_entity(&pool, eid, "SHARED_NAME", tenant, workspace).await;
        let report = index
            .upsert_batch(
                EmbeddingFamily::Entity,
                ModelId(Uuid::nil()),
                &[entity_row(workspace, eid, lid, 0.05)],
            )
            .await
            .expect("cross-ws lid must succeed");
        assert!(report.upserted >= 1, "{report:?}");
        assert_eq!(report.absorbed_legacy_collisions, 0);
    }
}

/// T5: relationship family absorb.
#[tokio::test]
async fn contract_spec120_relationship_dual_fk_absorb() {
    let Some(cfg) = require_or_skip_postgres("spec120_rel_dual") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let (tenant, workspace) = seed_tenant_ws(&pool).await;
    let src = Uuid::new_v4();
    let tgt = Uuid::new_v4();
    insert_entity(&pool, src, "SRC_A", tenant, workspace).await;
    insert_entity(&pool, tgt, "TGT_A", tenant, workspace).await;

    let r1 = Uuid::new_v4();
    let r2 = Uuid::new_v4();
    // Two relationship rows: distinct relation_type so UNIQUE allows both FKs.
    for (rid, rel_type) in [(r1, "LINKS"), (r2, "RELATED")] {
        sqlx::query(
            "INSERT INTO relationships \
             (id, source_id, target_id, tenant_id, workspace_id, relation_type, description, weight, sync_status) \
             VALUES ($1, $2, $3, $4, $5, $6, 'x', 1.0, 'synced')",
        )
        .bind(rid)
        .bind(src)
        .bind(tgt)
        .bind(tenant)
        .bind(workspace)
        .bind(rel_type)
        .execute(&pool)
        .await
        .expect("rel");
    }

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "spec120-rel");
    let lid = "SRC_A->TGT_A:LINKS";
    let mk = |rid: Uuid, seed: f32| FleetEmbeddingRow {
        workspace_id: WorkspaceId::new(workspace),
        embedding: emb(seed),
        dimensions: DIM as i32,
        key: FleetEmbeddingKey::Relationship(rid),
        legacy_vector_id: Some(lid.to_string()),
    };
    let rows_a = [mk(r1, 0.1)];
    let rows_b = [mk(r2, 0.2)];

    let (a, b) = tokio::join!(
        index.upsert_batch(EmbeddingFamily::Relationship, ModelId(Uuid::nil()), &rows_a),
        index.upsert_batch(EmbeddingFamily::Relationship, ModelId(Uuid::nil()), &rows_b),
    );
    a.expect("rel upsert A");
    b.expect("rel upsert B");

    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM relationship_embeddings \
         WHERE workspace_id = $1 AND legacy_vector_id = $2",
    )
    .bind(workspace)
    .bind(lid)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(owners, 1);
}

/// T5: report family absorb.
#[tokio::test]
async fn contract_spec120_report_dual_fk_absorb() {
    let Some(cfg) = require_or_skip_postgres("spec120_report_dual") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let (_tenant, workspace) = seed_tenant_ws(&pool).await;
    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "spec120-report");
    // report_id is part of the global PK (model_id, report_id) — must be unique per run.
    let run = Uuid::new_v4();
    let lid = format!("community_report:cluster-{run}");
    let mk = |report_id: String, seed: f32| FleetEmbeddingRow {
        workspace_id: WorkspaceId::new(workspace),
        embedding: emb(seed),
        dimensions: DIM as i32,
        key: FleetEmbeddingKey::Report(report_id),
        legacy_vector_id: Some(lid.clone()),
    };
    let rows_a = [mk(format!("community_report:cluster-{run}#a"), 0.3)];
    let rows_b = [mk(format!("community_report:cluster-{run}#b"), 0.4)];

    let (a, b) = tokio::join!(
        index.upsert_batch(EmbeddingFamily::Report, ModelId(Uuid::nil()), &rows_a),
        index.upsert_batch(EmbeddingFamily::Report, ModelId(Uuid::nil()), &rows_b),
    );
    a.expect("report A");
    b.expect("report B");

    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM report_embeddings \
         WHERE workspace_id = $1 AND legacy_vector_id = $2",
    )
    .bind(workspace)
    .bind(&lid)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(owners, 1);
}

/// Mixed batch: colliding lid + fresh lid in one unnest (EC-07).
#[tokio::test]
async fn contract_spec120_mixed_collide_and_fresh() {
    let Some(cfg) = require_or_skip_postgres("spec120_mixed") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    let (tenant, workspace) = seed_tenant_ws(&pool).await;
    let e1 = Uuid::new_v4();
    let e2 = Uuid::new_v4();
    let e3 = Uuid::new_v4();
    insert_entity(&pool, e1, "MIX_A", tenant, workspace).await;
    insert_entity(&pool, e2, "Mix A", tenant, workspace).await;
    insert_entity(&pool, e3, "MIX_B", tenant, workspace).await;

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "spec120-mixed");
    // Pre-stamp winner for MIX_A lid
    index
        .upsert_batch(
            EmbeddingFamily::Entity,
            ModelId(Uuid::nil()),
            &[entity_row(workspace, e1, "entity:MIX_A", 0.1)],
        )
        .await
        .expect("prestamp");

    let report = index
        .upsert_batch(
            EmbeddingFamily::Entity,
            ModelId(Uuid::nil()),
            &[
                entity_row(workspace, e2, "entity:MIX_A", 0.2),
                entity_row(workspace, e3, "entity:MIX_B", 0.3),
            ],
        )
        .await
        .expect("mixed batch");
    assert!(
        report.absorbed_legacy_collisions >= 1,
        "loser MIX_A must absorb: {report:?}"
    );
    assert!(report.upserted >= 1, "fresh MIX_B must insert: {report:?}");

    let mix_b: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM entity_embeddings \
         WHERE entity_id = $1 AND legacy_vector_id = 'entity:MIX_B'",
    )
    .bind(e3)
    .fetch_one(&pool)
    .await
    .expect("mix_b");
    assert_eq!(mix_b, 1);
}

/// SPEC-136 / #377: unique index is still the arbiter (unfakable control).
async fn assert_legacy_unique_index(pool: &sqlx::PgPool, index_name: &str) {
    let def: Option<String> = sqlx::query_scalar("SELECT pg_get_indexdef($1::regclass)")
        .bind(index_name)
        .fetch_optional(pool)
        .await
        .unwrap_or_else(|e| panic!("{index_name} lookup failed: {e}"));
    let def = def.unwrap_or_else(|| panic!("{index_name} must exist"));
    assert!(
        def.to_lowercase().contains("legacy_vector_id"),
        "{index_name} must still unique-index legacy_vector_id: {def}"
    );
    assert!(
        def.to_lowercase().contains("workspace_id"),
        "{index_name} must remain workspace-scoped (migration 144): {def}"
    );
}

async fn insert_null_lid_clone(
    pool: &sqlx::PgPool,
    table: &str,
    fk_column: &str,
    winner_fk: Uuid,
    loser_fk: Uuid,
) {
    let sql = format!(
        "INSERT INTO {table} (model_id, {fk_column}, workspace_id, embedding, dimensions, legacy_vector_id) \
         SELECT model_id, $1, workspace_id, embedding, dimensions, NULL \
         FROM {table} WHERE {fk_column} = $2 LIMIT 1"
    );
    let inserted = sqlx::query(&sql)
        .bind(loser_fk)
        .bind(winner_fk)
        .execute(pool)
        .await
        .unwrap_or_else(|e| panic!("NULL-lid clone into {table} failed: {e}"))
        .rows_affected();
    assert_eq!(inserted, 1, "expected one NULL-lid row cloned into {table}");
}

fn assert_sqlstate_23505_legacy(err: sqlx::Error, what: &str) {
    match err {
        sqlx::Error::Database(db) => {
            assert_eq!(
                db.code().as_deref(),
                Some("23505"),
                "{what}: expected unique_violation, got code={:?}",
                db.code()
            );
            let constraint = db.constraint().unwrap_or("");
            let message = db.message();
            assert!(
                constraint.contains("legacy_vector_id") || message.contains("legacy_vector_id"),
                "{what}: 23505 must name legacy_vector_id (constraint={constraint:?} message={message})"
            );
        }
        other => panic!("{what}: expected Database(23505), got {other:?}"),
    }
}

/// T-377-0 / T-377-1: NULL-lid loser PK + committed lid → absorb, retry Ok, one owner.
#[tokio::test]
async fn contract_spec136_null_lid_loser_pk_entity_absorb_and_retry() {
    let Some(cfg) = require_or_skip_postgres("spec136_entity_null_lid") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    assert_legacy_unique_index(&pool, "idx_entity_embeddings_legacy_vector_id").await;

    let (tenant, workspace) = seed_tenant_ws(&pool).await;
    let winner = Uuid::new_v4();
    let loser = Uuid::new_v4();
    insert_entity(&pool, winner, "John Smith", tenant, workspace).await;
    insert_entity(&pool, loser, "JOHN_SMITH", tenant, workspace).await;

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "spec136-entity");
    let lid = "entity:JOHN_SMITH";
    index
        .upsert_batch(
            EmbeddingFamily::Entity,
            ModelId(Uuid::nil()),
            &[entity_row(workspace, winner, lid, 0.11)],
        )
        .await
        .expect("winner stamp");

    insert_null_lid_clone(&pool, "entity_embeddings", "entity_id", winner, loser).await;

    let raw =
        sqlx::query("UPDATE entity_embeddings SET legacy_vector_id = $1 WHERE entity_id = $2")
            .bind(lid)
            .bind(loser)
            .execute(&pool)
            .await
            .expect_err("T-377-0: raw stamp of loser PK must 23505");
    assert_sqlstate_23505_legacy(raw, "T-377-0 entity control UPDATE");

    let first = index
        .upsert_batch(
            EmbeddingFamily::Entity,
            ModelId(Uuid::nil()),
            &[entity_row(workspace, loser, lid, 0.12)],
        )
        .await
        .expect("loser upsert must absorb, not 23505");
    assert!(
        first.absorbed_legacy_collisions >= 1,
        "NULL-lid loser must count as absorbed: {first:?}"
    );

    let second = index
        .upsert_batch(
            EmbeddingFamily::Entity,
            ModelId(Uuid::nil()),
            &[entity_row(workspace, loser, lid, 0.13)],
        )
        .await
        .expect("retry must absorb again");
    assert!(
        second.absorbed_legacy_collisions >= 1,
        "retry must still absorb: {second:?}"
    );

    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM entity_embeddings \
         WHERE workspace_id = $1 AND legacy_vector_id = $2",
    )
    .bind(workspace)
    .bind(lid)
    .fetch_one(&pool)
    .await
    .expect("owners");
    assert_eq!(owners, 1, "exactly one lid owner after absorb+retry");

    let winner_owns: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM entity_embeddings \
         WHERE entity_id = $1 AND legacy_vector_id = $2)",
    )
    .bind(winner)
    .bind(lid)
    .fetch_one(&pool)
    .await
    .expect("winner owns");
    assert!(winner_owns, "winner must keep the lid");

    let loser_stole: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM entity_embeddings \
         WHERE entity_id = $1 AND legacy_vector_id = $2)",
    )
    .bind(loser)
    .bind(lid)
    .fetch_one(&pool)
    .await
    .expect("loser stole");
    assert!(!loser_stole, "loser PK must not steal the lid");
}

/// T-377-2: relationship twin of the durable NULL-lid PK collision.
#[tokio::test]
async fn contract_spec136_null_lid_loser_pk_relationship_absorb_and_retry() {
    let Some(cfg) = require_or_skip_postgres("spec136_rel_null_lid") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
    assert_legacy_unique_index(&pool, "idx_relationship_embeddings_legacy_vector_id").await;

    let (tenant, workspace) = seed_tenant_ws(&pool).await;
    let src = Uuid::new_v4();
    let tgt = Uuid::new_v4();
    insert_entity(&pool, src, "SRC_B", tenant, workspace).await;
    insert_entity(&pool, tgt, "TGT_B", tenant, workspace).await;

    let winner = Uuid::new_v4();
    let loser = Uuid::new_v4();
    for (rid, rel_type) in [(winner, "LINKS"), (loser, "RELATED")] {
        sqlx::query(
            "INSERT INTO relationships \
             (id, source_id, target_id, tenant_id, workspace_id, relation_type, description, weight, sync_status) \
             VALUES ($1, $2, $3, $4, $5, $6, 'x', 1.0, 'synced')",
        )
        .bind(rid)
        .bind(src)
        .bind(tgt)
        .bind(tenant)
        .bind(workspace)
        .bind(rel_type)
        .execute(&pool)
        .await
        .expect("rel");
    }

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "spec136-rel");
    let lid = "SRC_B->TGT_B:LINKS";
    let mk = |rid: Uuid, seed: f32| FleetEmbeddingRow {
        workspace_id: WorkspaceId::new(workspace),
        embedding: emb(seed),
        dimensions: DIM as i32,
        key: FleetEmbeddingKey::Relationship(rid),
        legacy_vector_id: Some(lid.to_string()),
    };
    index
        .upsert_batch(
            EmbeddingFamily::Relationship,
            ModelId(Uuid::nil()),
            &[mk(winner, 0.21)],
        )
        .await
        .expect("winner rel stamp");

    insert_null_lid_clone(
        &pool,
        "relationship_embeddings",
        "relationship_id",
        winner,
        loser,
    )
    .await;

    let raw = sqlx::query(
        "UPDATE relationship_embeddings SET legacy_vector_id = $1 WHERE relationship_id = $2",
    )
    .bind(lid)
    .bind(loser)
    .execute(&pool)
    .await
    .expect_err("T-377-0 rel: raw stamp must 23505");
    assert_sqlstate_23505_legacy(raw, "T-377-0 relationship control UPDATE");

    let first = index
        .upsert_batch(
            EmbeddingFamily::Relationship,
            ModelId(Uuid::nil()),
            &[mk(loser, 0.22)],
        )
        .await
        .expect("rel loser upsert must absorb");
    assert!(
        first.absorbed_legacy_collisions >= 1,
        "rel NULL-lid loser must absorb: {first:?}"
    );

    let second = index
        .upsert_batch(
            EmbeddingFamily::Relationship,
            ModelId(Uuid::nil()),
            &[mk(loser, 0.23)],
        )
        .await
        .expect("rel retry must absorb");
    assert!(
        second.absorbed_legacy_collisions >= 1,
        "rel retry must absorb: {second:?}"
    );

    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM relationship_embeddings \
         WHERE workspace_id = $1 AND legacy_vector_id = $2",
    )
    .bind(workspace)
    .bind(lid)
    .fetch_one(&pool)
    .await
    .expect("rel owners");
    assert_eq!(owners, 1, "exactly one relationship lid owner");
}
