//! SPEC-098 LAW-098-8: relationship sink dedupes arbiter keys before ON CONFLICT.
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

#[tokio::test]
async fn e2e_spec098_rel_sink_batch_dedupe() {
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
            name: "SRC_DEDUP".into(),
            entity_type: "ORG".into(),
            description: "s".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        },
        EntitySinkRow {
            name: "TGT_DEDUP".into(),
            entity_type: "ORG".into(),
            description: "t".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        },
    ])
    .await
    .expect("entities");

    // Duplicate arbiter key with mixed-case relation_type — must not raise 21000.
    let rels = vec![
        RelationshipSinkRow {
            source_name: "SRC_DEDUP".into(),
            target_name: "TGT_DEDUP".into(),
            relation_type: "knows".into(),
            description: "first".into(),
            weight: 0.5,
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        },
        RelationshipSinkRow {
            source_name: "SRC_DEDUP".into(),
            target_name: "TGT_DEDUP".into(),
            relation_type: "KNOWS".into(),
            description: "second-wins".into(),
            weight: 1.0,
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        },
    ];
    sink.upsert_relationships_batch(&rels)
        .await
        .expect("duplicate rel sink batch must not fail ON CONFLICT cardinality");

    let (desc, weight, rel_type): (String, f32, String) = sqlx::query_as(
        "SELECT description, weight, relation_type FROM relationships \
         WHERE workspace_id = $1 AND relation_type = 'KNOWS'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("one KNOWS row");
    assert_eq!(desc, "second-wins");
    assert!((weight - 1.0).abs() < f32::EPSILON);
    assert_eq!(rel_type, "KNOWS");

    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM relationships WHERE workspace_id = $1 AND relation_type = 'KNOWS'",
    )
    .bind(workspace)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(n, 1);
}

/// Document lineage reaches `relationships.source_chunk_keys` in the same
/// statement as the row, per row, and unions across writes. Migration 211's
/// column was empty on every row because no writer set it.
#[tokio::test]
async fn e2e_relationship_sink_records_and_unions_chunk_keys() {
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

    let scope = (Some(tenant.to_string()), Some(workspace.to_string()));
    let entity = |name: &str| EntitySinkRow {
        name: name.into(),
        entity_type: "ORG".into(),
        description: "e".into(),
        tenant_id: scope.0.clone(),
        workspace_id: scope.1.clone(),
        source_chunk_ids: vec![],
    };
    let rel = |rel_type: &str, keys: &[&str]| RelationshipSinkRow {
        source_name: "LIN_SRC".into(),
        target_name: "LIN_TGT".into(),
        relation_type: rel_type.into(),
        description: "r".into(),
        weight: 1.0,
        tenant_id: scope.0.clone(),
        workspace_id: scope.1.clone(),
        source_chunk_ids: keys.iter().map(|k| k.to_string()).collect(),
    };
    let keys_of = |rel_type: &'static str| {
        let pool = pool.clone();
        async move {
            let keys: Option<Vec<String>> = sqlx::query_scalar(
                "SELECT source_chunk_keys FROM relationships \
                 WHERE workspace_id = $1 AND relation_type = $2",
            )
            .bind(workspace)
            .bind(rel_type)
            .fetch_one(&pool)
            .await
            .expect("relationship row");
            let mut keys = keys.unwrap_or_default();
            keys.sort();
            keys
        }
    };

    let sink = PostgresEntitySink::new_fail_closed(Arc::new(pool.clone()));
    sink.upsert_entities_batch(&[entity("LIN_SRC"), entity("LIN_TGT")])
        .await
        .expect("entities");

    // One batch: rows with different key sets (including none) must not bleed
    // into each other, and the two KNOWS rows merge into one.
    sink.upsert_relationships_batch(&[
        rel("KNOWS", &["d1-chunk-0"]),
        rel("LIKES", &["d2-chunk-0", "d2-chunk-1"]),
        rel("KNOWS", &["d3-chunk-0", "d1-chunk-0"]),
        rel("EMPTY", &[]),
    ])
    .await
    .expect("batch");
    assert_eq!(keys_of("KNOWS").await, ["d1-chunk-0", "d3-chunk-0"]);
    assert_eq!(keys_of("LIKES").await, ["d2-chunk-0", "d2-chunk-1"]);
    assert!(keys_of("EMPTY").await.is_empty());

    // A later batch adds to the stored keys instead of replacing them.
    sink.upsert_relationships_batch(&[rel("LIKES", &["d4-chunk-0", "d2-chunk-0"])])
        .await
        .expect("second batch");
    assert_eq!(
        keys_of("LIKES").await,
        ["d2-chunk-0", "d2-chunk-1", "d4-chunk-0"]
    );

    // The single-row path writes and unions the same way.
    sink.upsert_relationship(
        "LIN_SRC",
        "LIN_TGT",
        "KNOWS",
        "single",
        1.0,
        scope.0.as_deref(),
        scope.1.as_deref(),
        &["d5-chunk-0".to_string()],
    )
    .await
    .expect("single");
    assert_eq!(
        keys_of("KNOWS").await,
        ["d1-chunk-0", "d3-chunk-0", "d5-chunk-0"]
    );
}
