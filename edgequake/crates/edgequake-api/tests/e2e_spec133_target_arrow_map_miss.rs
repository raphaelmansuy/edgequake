//! SPEC-133 e2e: target-arrow legacy keys still mirror when known map misses;
//! fail-closed when spine endpoints are absent.
#![cfg(feature = "postgres")]

#[path = "common/test_db.rs"]
mod test_db;

use edgequake_api::postgres_entity_sink::PostgresEntitySink;
use edgequake_pipeline::{EntitySinkRow, RelationalEntitySink, RelationshipSinkRow};
use edgequake_storage::format_relationship_legacy_key;
use edgequake_storage::parse_relationship_legacy_key;
use edgequake_storage::traits::FleetEmbeddingIndex;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

fn require_db() -> Option<String> {
    let base = std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| !u.trim().is_empty())
        .or_else(|| {
            std::fs::read_to_string("/tmp/edgequake-db-url")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|u| !u.is_empty())
        })?;
    Some(test_db::isolated_test_url(&base))
}

const SRC: &str = "FLOW_DIRECTION";
const TGT: &str = "ARROW_1_(SHADED_BOX_->CIRCULAR_TARGET)";
const REL: &str = "RELATED_TO";

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

/// T3: incomplete known map + target-arrow name → index-guided parse still mirrors.
#[tokio::test]
async fn e2e_spec133_target_arrow_map_miss_still_mirrors() {
    let Some(url) = require_db() else {
        eprintln!("skip: DATABASE_URL unset");
        return;
    };
    let pool = PgPool::connect(&url).await.expect("connect");
    let (tenant, workspace) = seed_tenant_workspace(&pool).await;

    let legacy_id = format_relationship_legacy_key(SRC, TGT, REL);
    let naive = parse_relationship_legacy_key(&legacy_id).expect("naive");
    assert_ne!(
        (naive.0.as_str(), naive.1.as_str()),
        (SRC, TGT),
        "fixture must be target-arrow ambiguous under naive rsplit"
    );

    let sink = PostgresEntitySink::new_fail_closed(Arc::new(pool.clone()));
    sink.upsert_entities_batch(&[
        EntitySinkRow {
            name: SRC.into(),
            entity_type: "CONCEPT".into(),
            description: "flow".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        },
        EntitySinkRow {
            name: TGT.into(),
            entity_type: "CONCEPT".into(),
            description: "arrow".into(),
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        },
    ])
    .await
    .expect("entities");

    let report = sink
        .upsert_relationships_batch(&[RelationshipSinkRow {
            source_name: SRC.into(),
            target_name: TGT.into(),
            relation_type: REL.into(),
            description: "diagram edge".into(),
            weight: 1.0,
            tenant_id: Some(tenant.to_string()),
            workspace_id: Some(workspace.to_string()),
            source_chunk_ids: vec![],
        }])
        .await
        .expect("rel sink");
    assert_eq!(report.missing_fk, 0, "{report:?}");
    assert!(
        report.ids.contains_key(&legacy_id),
        "sink must key by format SSOT: {report:?}"
    );

    // Simulate incomplete / empty known map (SPEC-130 miss path → SPEC-133 parse).
    let empty: HashMap<String, Uuid> = HashMap::new();
    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "e2e-spec133-miss");
    let emb = vec![0.08f32; 1024];
    let mirror = index
        .mirror_legacy_batch(
            &[(
                legacy_id.clone(),
                emb.clone(),
                json!({ "workspace_id": workspace.to_string() }),
            )],
            false,
            Some(&empty),
        )
        .await
        .expect("mirror empty-map");

    assert_eq!(mirror.eligible, 1, "{mirror:?}");
    assert_eq!(
        mirror.resolved, 1,
        "map-miss + target-arrow must resolve via index parse: {mirror:?}"
    );
    assert!(mirror.misses.is_empty(), "{mirror:?}");
    assert!(mirror.is_complete());

    // Also: known map with *wrong* key only — still recover by parse.
    let mut wrong = HashMap::new();
    wrong.insert("UNRELATED->PAIR:RELATED_TO".to_string(), Uuid::new_v4());
    let mirror2 = index
        .mirror_legacy_batch(
            &[(
                legacy_id,
                emb,
                json!({ "workspace_id": workspace.to_string() }),
            )],
            false,
            Some(&wrong),
        )
        .await
        .expect("mirror wrong-map");
    assert_eq!(mirror2.resolved, 1, "{mirror2:?}");
    assert!(mirror2.is_complete());
}

/// T3 control: no spine endpoints → fail-closed miss (naive wrong names also miss).
#[tokio::test]
async fn e2e_spec133_target_arrow_absent_spine_fail_closed() {
    let Some(url) = require_db() else {
        eprintln!("skip: DATABASE_URL unset");
        return;
    };
    let pool = PgPool::connect(&url).await.expect("connect");
    let (_tenant, workspace) = seed_tenant_workspace(&pool).await;

    let legacy_id = format_relationship_legacy_key(SRC, TGT, REL);
    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool, "e2e-spec133-fail");
    let emb = vec![0.08f32; 1024];
    let mirror = index
        .mirror_legacy_batch(
            &[(
                legacy_id.clone(),
                emb,
                json!({ "workspace_id": workspace.to_string() }),
            )],
            false,
            None,
        )
        .await
        .expect("mirror");

    assert_eq!(mirror.eligible, 1, "{mirror:?}");
    assert_eq!(mirror.resolved, 0, "{mirror:?}");
    assert!(!mirror.is_complete());
    assert!(
        mirror.misses.iter().any(|m| m == &legacy_id),
        "expected miss sample {legacy_id}: {mirror:?}"
    );
}
