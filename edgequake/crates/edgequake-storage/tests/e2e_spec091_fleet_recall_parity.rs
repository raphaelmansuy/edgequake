//! SPEC-091 IW2 recall parity: typed `entity_embeddings` ANN recall@10
//! matches exact cosine baseline on a seeded corpus.
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::embedding_family::EmbeddingFamily;
use edgequake_storage::traits::domain::{
    FleetEmbeddingIndex, FleetEmbeddingKey, FleetEmbeddingRow, ModelId, VectorQuery, WorkspaceId,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::collections::HashSet;
use uuid::Uuid;

const DIM: usize = 1536;
const CORPUS: usize = 24;
const TOP_K: usize = 10;

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

#[tokio::test]
async fn e2e_spec091_fleet_recall_parity_entity_typed_vs_exact() {
    let Some(cfg) = require_or_skip_postgres("spec091_iw2_fleet_recall") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;
    let ws = w3::seed_workspace(&pool, "fleet-recall").await;

    let index = edgequake_storage::PgFleetEmbeddingIndex::new(pool.clone(), "iw2-recall-model");

    let mut corpus: Vec<(String, Vec<f32>)> = Vec::new();
    let mut rows: Vec<FleetEmbeddingRow> = Vec::new();
    for i in 0..CORPUS {
        let name = format!("ENTITY_{i}");
        let eid: Uuid = sqlx::query_scalar(
            "INSERT INTO entities (name, entity_type, workspace_id) VALUES ($1, 'concept', $2) \
             RETURNING id",
        )
        .bind(&name)
        .bind(ws)
        .fetch_one(&pool)
        .await
        .expect("seed entity");
        let emb = w3::make_embedding(DIM, 700 + i as u32);
        let legacy_id = format!("entity:{name}");
        corpus.push((legacy_id.clone(), emb.clone()));
        rows.push(FleetEmbeddingRow {
            key: FleetEmbeddingKey::Entity(eid),
            workspace_id: WorkspaceId(ws),
            dimensions: DIM as i32,
            embedding: emb,
            legacy_vector_id: Some(legacy_id),
        });
    }
    index
        .upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &rows)
        .await
        .expect("upsert");

    let probes = [0u32, 3, 11, 17];
    let mut total_recall = 0f32;
    for probe_seed in probes {
        let query_emb = w3::make_embedding(DIM, 900 + probe_seed);
        let mut exact: Vec<(String, f32)> = corpus
            .iter()
            .map(|(id, emb)| (id.clone(), cosine(&query_emb, emb)))
            .collect();
        exact.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let exact_top: HashSet<String> =
            exact.iter().take(TOP_K).map(|(id, _)| id.clone()).collect();

        let hits = index
            .search(
                EmbeddingFamily::Entity,
                &VectorQuery {
                    model_id: ModelId(Uuid::nil()),
                    workspace_id: Some(WorkspaceId(ws)),
                    embedding: query_emb,
                    limit: TOP_K as u32,
                            allowed_document_ids: None,
},
            )
            .await
            .expect("typed search");
        let typed_top: HashSet<String> = hits.iter().map(|h| h.legacy_id.clone()).collect();
        let overlap = exact_top.intersection(&typed_top).count() as f32;
        total_recall += overlap / TOP_K as f32;
    }
    let mean_recall = total_recall / probes.len() as f32;
    assert!(
        mean_recall >= 0.9,
        "entity typed recall parity gate failed: {mean_recall:.3} < 0.9"
    );

    w3::cleanup_workspace(&pool, ws).await;
}
