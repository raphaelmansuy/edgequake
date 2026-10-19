//! SPEC-130 e2e: sink RETURNING id map → fleet mirror by UUID.
#![cfg(feature = "postgres")]

#[path = "common/test_db.rs"]
mod test_db;

use edgequake_api::postgres_entity_sink::PostgresEntitySink;
use edgequake_pipeline::{EntitySinkRow, RelationalEntitySink, RelationshipSinkRow};
use edgequake_storage::format_relationship_legacy_key;
use edgequake_storage::traits::FleetEmbeddingIndex;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

fn require_db() -> Option<String> {
    let base = std::env::var("DATABASE_URL").ok()?;
    if base.trim().is_empty() {
        return None;
    }
    Some(test_db::isolated_test_url(&base))
}

#[tokio::test]
async fn e2e_spec130_sink_returning_mirror() {
    let Some(url) = require_db() else {
        eprintln!("skip: DATABASE_URL unset");
        return;
    };
    let pool = PgPool::connect(&url).await.expect("connect");
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

    let sink = PostgresEntitySink::new_fail_closed(Arc::new(pool.clone()));
    sink.upsert_entities_batch(&[
        EntitySinkRow {
            name: "MELISSA_BOTHA".into(),
            entity_type: "PERSON".into(),
            description: "m".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        },
        EntitySinkRow {
            name: "FLAT_4".into(),
            entity_type: "PLACE".into(),
            description: "f".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        },
    ])
    .await
    .expect("entities");

    let report = sink
        .upsert_relationships_batch(&[RelationshipSinkRow {
            source_name: "MELISSA_BOTHA".into(),
            target_name: "FLAT_4".into(),
            relation_type: "owner_of".into(),
            description: "owns".into(),
            weight: 1.0,
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        }])
        .await
        .expect("rel sink");

    let expected_key = format_relationship_legacy_key("MELISSA_BOTHA", "FLAT_4", "owner_of");
    assert_eq!(expected_key, "MELISSA_BOTHA->FLAT_4:OWNER_OF");
    assert_eq!(report.missing_fk, 0);
    assert_eq!(report.ids.len(), 1, "{report:?}");
    let rid = *report.ids.get(&expected_key).expect("legacy key in map");

    let db_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM relationships WHERE workspace_id = $1 AND relation_type = 'OWNER_OF'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("db id");
    assert_eq!(rid, db_id);

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "e2e-spec130-sink");
    let emb = vec![0.08f32; 1024];
    let mirror = index
        .mirror_legacy_batch(
            &[(
                expected_key.clone(),
                emb,
                json!({ "workspace_id": workspace.to_string() }),
            )],
            false,
            Some(&report.ids),
        )
        .await
        .expect("mirror");

    assert_eq!(mirror.resolved, 1, "{mirror:?}");
    assert!(mirror.is_complete());
}
