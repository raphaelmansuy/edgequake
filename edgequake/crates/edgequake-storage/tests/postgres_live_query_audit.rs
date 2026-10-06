//! Opt-in read-only measurements on a real workspace, never a fixture substitute.
//! Requires EDGEQUAKE_AUDIT_{WORKSPACE,DOCUMENT,MODEL,OUTPUT}; DATABASE_URL (or
//! make dev's /tmp/edgequake-db-url) selects the live database. Run with --ignored.
#![cfg(feature = "postgres")]

#[path = "support/typed_ann_plan.rs"]
mod typed_ann_plan;

use edgequake_storage::traits::domain::{
    EmbeddingIndex, FleetEmbeddingIndex, ModelId, TenantId, VectorQuery, WorkspaceId,
};
use edgequake_storage::{EmbeddingFamily, PgChunkEmbeddingIndex, PgFleetEmbeddingIndex};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use std::time::Instant;
use uuid::Uuid;

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("explicit audit option {name} is required"))
}

fn plan_structure(value: &Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .filter(|(key, _)| {
                    !matches!(
                        key.as_str(),
                        "Filter" | "One-Time Filter" | "Index Cond" | "Order By" | "Sort Key"
                    )
                })
                .map(|(key, value)| (key.clone(), plan_structure(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(plan_structure).collect()),
        _ => value.clone(),
    }
}

#[tokio::test]
#[ignore = "requires explicit live workspace and output; performs only read-only transactions"]
async fn live_typed_queries_measure_plans_scopes_and_ordering() {
    let workspace = Uuid::parse_str(&required("EDGEQUAKE_AUDIT_WORKSPACE")).unwrap();
    let document = Uuid::parse_str(&required("EDGEQUAKE_AUDIT_DOCUMENT")).unwrap();
    let model_name = required("EDGEQUAKE_AUDIT_MODEL");
    let output = required("EDGEQUAKE_AUDIT_OUTPUT");
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| std::fs::read_to_string("/tmp/edgequake-db-url").unwrap());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url.trim())
        .await
        .unwrap();
    // Enforced by PostgreSQL, including all subsequent adapter transactions.
    sqlx::query("SET default_transaction_read_only=on")
        .execute(&pool)
        .await
        .unwrap();
    let tenant: Uuid =
        sqlx::query_scalar("SELECT tenant_id FROM public.workspaces WHERE workspace_id=$1")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .unwrap();
    let owned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM public.documents WHERE id=$1 AND workspace_id=$2 AND tenant_id=$3)",
    )
    .bind(document)
    .bind(workspace)
    .bind(tenant)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        owned,
        "audit document must belong to the selected workspace"
    );
    let (model, dimension): (Uuid, i32) = sqlx::query_as(
        "SELECT id,dimensions FROM public.embedding_models WHERE name=$1 ORDER BY dimensions DESC LIMIT 1",
    )
    .bind(&model_name)
    .fetch_one(&pool)
    .await
    .unwrap();
    let vector: String = sqlx::query_scalar(
        "SELECT fe.embedding::text FROM public.chunk_embeddings fe \
         JOIN public.chunks c ON c.id=fe.chunk_id \
         WHERE fe.model_id=$1 AND fe.workspace_id=$2 AND c.document_id=$3 LIMIT 1",
    )
    .bind(model)
    .bind(workspace)
    .bind(document)
    .fetch_one(&pool)
    .await
    .unwrap();
    let embedding: Vec<f32> = serde_json::from_str(&vector).unwrap();
    assert_eq!(embedding.len(), dimension as usize);
    let chunk = PgChunkEmbeddingIndex::new(pool.clone(), &model_name);
    let fleet = PgFleetEmbeddingIndex::new(pool.clone(), &model_name);
    let mut reports = Vec::new();
    for kind in ["chunk", "entity", "relationship", "report"] {
        for scope in ["document", "workspace"] {
            // Community reports have workspace provenance, not document lineage.
            if kind == "report" && scope == "document" {
                continue;
            }
            let request = VectorQuery {
                model_id: ModelId(model),
                model_revision: "live-audit".into(),
                workspace_id: Some(WorkspaceId::new(workspace)),
                tenant_id: (kind != "report").then(|| TenantId::new(tenant)),
                document_ids: (scope == "document").then(|| vec![document]),
                modalities: None,
                filter_ids: None,
                vector_type: None,
                embedding: embedding.clone(),
                limit: 10,
            };
            let mut samples = Vec::new();
            let mut hit_count = 0;
            for probe in 0..32 {
                let start = Instant::now();
                let scores: Vec<f32> = if kind == "chunk" {
                    chunk
                        .search(&request)
                        .await
                        .unwrap()
                        .into_iter()
                        .map(|hit| hit.score)
                        .collect()
                } else {
                    let family = match kind {
                        "entity" => EmbeddingFamily::Entity,
                        "relationship" => EmbeddingFamily::Relationship,
                        _ => EmbeddingFamily::Report,
                    };
                    fleet
                        .search(family, &request)
                        .await
                        .unwrap()
                        .into_iter()
                        .map(|hit| hit.score)
                        .collect()
                };
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                assert!(
                    scores.windows(2).all(|pair| pair[0] >= pair[1]),
                    "{kind}/{scope} score order"
                );
                assert!(scores.iter().all(|score| score.is_finite()));
                if probe >= 2 {
                    samples.push(elapsed);
                }
                hit_count = scores.len();
            }
            let plan = typed_ann_plan::production_plan(&pool, kind, &request, model).await;
            samples.sort_by(f64::total_cmp);
            let report = json!({
                "family": kind, "scope": scope, "dimension": dimension,
                "samples": samples.len(), "hits": hit_count,
                "p50_ms": samples[15], "p95_ms": samples[28],
                "max_ms": samples[29], "explain": plan_structure(&plan),
                "scope_enforcement": "production tenant-access role and RLS",
                "planner_scan_switches_forced": false,
            });
            println!(
                "LIVE_QUERY_AUDIT {}",
                json!({"family":kind,"scope":scope,"p50_ms":samples[15],"p95_ms":samples[28],"hits":hit_count})
            );
            reports.push(report);
        }
    }
    std::fs::write(output, serde_json::to_string_pretty(&reports).unwrap()).unwrap();
    pool.close().await;
}
