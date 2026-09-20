//! SPEC-120 e2e: concurrent `mirror_legacy_batch` with alias entities sharing one lid.
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::traits::FleetEmbeddingIndex;
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn e2e_spec120_concurrent_mirror_alias_entities_absorb() {
    let Some(cfg) = require_or_skip_postgres("e2e_spec120_mirror") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
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

    // Alias pair: exact names differ; both resolve toward entity:JOHN_SMITH lid.
    sqlx::query(
        "INSERT INTO entities (name, entity_type, description, tenant_id, workspace_id, sync_status) \
         VALUES ('JOHN_SMITH', 'PERSON', 'a', $1, $2, 'synced'), \
                ('John Smith', 'PERSON', 'b', $1, $2, 'synced')",
    )
    .bind(tenant)
    .bind(workspace)
    .execute(&pool)
    .await
    .expect("entities");

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "e2e-spec120");
    let emb = vec![0.11f32; 1024];
    let meta = json!({ "workspace_id": workspace.to_string() });
    let batch_a = [("entity:JOHN_SMITH".to_string(), emb.clone(), meta.clone())];
    let batch_b = [("entity:JOHN_SMITH".to_string(), emb, meta)];

    // Concurrent mirrors: each loads EntityNameIndex independently.
    // Neither must return Err (LAW-120-1); exactly one lid owner.
    let (a, b) = tokio::join!(
        index.mirror_legacy_batch(&batch_a, true, None),
        index.mirror_legacy_batch(&batch_b, true, None),
    );
    let a = a.expect("mirror A must not fail on legacy unique");
    let b = b.expect("mirror B must not fail on legacy unique");
    assert!(a.resolved + b.resolved >= 1, "a={a:?} b={b:?}");
    assert!(a.is_complete() && b.is_complete(), "a={a:?} b={b:?}");

    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM entity_embeddings \
         WHERE workspace_id = $1 AND legacy_vector_id = 'entity:JOHN_SMITH'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("owners");
    assert_eq!(owners, 1, "one lid owner after concurrent mirror");
}

/// Dual-FK forced through mirror resolve: two entities where only normalized
/// keys collide — winner stamps; loser absorb when we upsert the other FK with
/// the same lid via a second index call after deleting the first entity's exact
/// map is unnecessary: use upsert_batch after mirror for the second FK.
#[tokio::test]
async fn e2e_spec120_mirror_then_losing_fk_absorb() {
    let Some(cfg) = require_or_skip_postgres("e2e_spec120_lose") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;
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

    let e1 = Uuid::new_v4();
    let e2 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO entities (id, name, entity_type, description, tenant_id, workspace_id, sync_status) \
         VALUES ($1, 'JOHN_SMITH', 'PERSON', 'a', $3, $4, 'synced'), \
                ($2, 'John Smith', 'PERSON', 'b', $3, $4, 'synced')",
    )
    .bind(e1)
    .bind(e2)
    .bind(tenant)
    .bind(workspace)
    .execute(&pool)
    .await
    .expect("entities");

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "e2e-spec120-lose");
    let emb = vec![0.12f32; 1024];
    let report = index
        .mirror_legacy_batch(
            &[(
                "entity:JOHN_SMITH".into(),
                emb.clone(),
                json!({ "workspace_id": workspace.to_string() }),
            )],
            true,
            None,
        )
        .await
        .expect("mirror winner");
    assert_eq!(report.resolved, 1);

    // Losing FK: write same lid under the other entity id (simulates racing resolve).
    use edgequake_storage::embedding_family::EmbeddingFamily;
    use edgequake_storage::traits::{FleetEmbeddingKey, FleetEmbeddingRow, ModelId, WorkspaceId};
    let loser = FleetEmbeddingRow {
        workspace_id: WorkspaceId::new(workspace),
        embedding: emb,
        dimensions: 1024,
        key: FleetEmbeddingKey::Entity(e2),
        legacy_vector_id: Some("entity:JOHN_SMITH".into()),
    };
    // Ensure e2 is the one not already stamped
    let stamped_on: Option<Uuid> = sqlx::query_scalar(
        "SELECT entity_id FROM entity_embeddings WHERE workspace_id = $1 \
         AND legacy_vector_id = 'entity:JOHN_SMITH'",
    )
    .bind(workspace)
    .fetch_optional(&pool)
    .await
    .expect("stamped");
    let loser_id = if stamped_on == Some(e1) { e2 } else { e1 };
    let loser = FleetEmbeddingRow {
        key: FleetEmbeddingKey::Entity(loser_id),
        ..loser
    };

    let ur = index
        .upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &[loser])
        .await
        .expect("loser must absorb, not Err");
    assert!(
        ur.absorbed_legacy_collisions >= 1,
        "expected absorb: {ur:?}"
    );

    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM entity_embeddings \
         WHERE workspace_id = $1 AND legacy_vector_id = 'entity:JOHN_SMITH'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("owners");
    assert_eq!(owners, 1);
}
