//! SPEC-091 IP1 — CQRS batch sink (LAW-IP2 / IP-AC-03).
#![cfg(feature = "postgres")]

#[path = "common/test_db.rs"]
mod test_db;

use edgequake_api::postgres_entity_sink::PostgresEntitySink;
use edgequake_pipeline::{EntitySinkRow, RelationalEntitySink, RelationshipSinkRow};
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

async fn pool(url: &str) -> PgPool {
    PgPool::connect(url).await.expect("connect test db")
}

async fn seed_tenant_workspace(pool: &PgPool) -> (Uuid, Uuid) {
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

#[tokio::test]
async fn contract_spec091_cqrs_batch_entities_and_rels() {
    let Some(url) = require_db() else {
        eprintln!("skip: DATABASE_URL unset");
        return;
    };
    let pool = pool(&url).await;
    let (tenant, workspace) = seed_tenant_workspace(&pool).await;
    let sink = PostgresEntitySink::new_fail_closed(Arc::new(pool.clone()));

    let rows = vec![
        EntitySinkRow {
            name: "ALPHA_CO".into(),
            entity_type: "ORG".into(),
            description: "a".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec!["c1".into()],
        },
        EntitySinkRow {
            name: "BETA_CO".into(),
            entity_type: "ORG".into(),
            description: "b".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec!["c2".into(), "c3".into()],
        },
        // Duplicate name in same batch — second description wins via ON CONFLICT.
        EntitySinkRow {
            name: "ALPHA_CO".into(),
            entity_type: "ORG".into(),
            description: "a-updated".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec!["c4".into()],
        },
    ];
    sink.upsert_entities_batch(&rows)
        .await
        .expect("entity batch");

    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM entities WHERE workspace_id = $1 AND name = ANY($2)",
    )
    .bind(workspace)
    .bind(&["ALPHA_CO".to_string(), "BETA_CO".to_string()][..])
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(n, 2);

    let desc: String = sqlx::query_scalar(
        "SELECT description FROM entities WHERE workspace_id = $1 AND name = 'ALPHA_CO'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("desc");
    assert_eq!(desc, "a-updated");

    let rels = vec![RelationshipSinkRow {
        source_name: "ALPHA_CO".into(),
        target_name: "BETA_CO".into(),
        relation_type: "PARTNERS_WITH".into(),
        description: "p".into(),
        weight: 1.0,
        tenant_id: Some(tenant.to_string()),
        workspace_id: Some(workspace.to_string()),
        source_chunk_ids: vec![],
    }];
    sink.upsert_relationships_batch(&rels)
        .await
        .expect("rel batch");

    let rn: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM relationships WHERE workspace_id = $1 AND relation_type = 'PARTNERS_WITH'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("rel count");
    assert_eq!(rn, 1);
}

#[tokio::test]
async fn contract_spec091_cqrs_batch_empty_and_missing_fk() {
    let Some(url) = require_db() else {
        eprintln!("skip: DATABASE_URL unset");
        return;
    };
    let pool = pool(&url).await;
    let (tenant, workspace) = seed_tenant_workspace(&pool).await;
    let sink = PostgresEntitySink::new_fail_closed(Arc::new(pool.clone()));

    sink.upsert_entities_batch(&[]).await.expect("empty ok");
    sink.upsert_relationships_batch(&[])
        .await
        .expect("empty rels ok");

    // Missing FK endpoints → fail-closed Err.
    let err = sink
        .upsert_relationships_batch(&[RelationshipSinkRow {
            source_name: "MISSING_A".into(),
            target_name: "MISSING_B".into(),
            relation_type: "RELATED_TO".into(),
            description: "x".into(),
            weight: 0.5,
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        }])
        .await
        .expect_err("missing FK must fail closed");
    assert!(
        err.to_string().contains("missing entity FK") || err.to_string().contains("relational"),
        "unexpected: {err}"
    );
}
