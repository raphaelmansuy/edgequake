//! SPEC-091 IW5: cross-tenant ANN isolation on typed `chunk_embeddings`.
//!
//! Queries scoped to workspace A must never return chunks belonging to B.
//!
//! Run:
//!   cargo test -p edgequake-storage --features postgres \
//!     --test e2e_spec091_cross_tenant_ann_leak -- --test-threads=1
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::traits::domain::{
    EmbeddingIndex, EmbeddingRow, ModelId, VectorQuery, WorkspaceId,
};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use std::collections::HashSet;
use uuid::Uuid;

const DIM: usize = 1536;

#[tokio::test]
async fn e2e_spec091_cross_tenant_ann_no_leak() {
    let Some(cfg) = require_or_skip_postgres("spec091_ann_leak") else {
        return;
    };
    let _g = w3::w3_lock().lock().await;
    let pool = contract_pg_pool(&cfg).await;

    let ws_a = w3::seed_workspace(&pool, "ann-a").await;
    let ws_b = w3::seed_workspace(&pool, "ann-b").await;
    let doc_a = w3::seed_document(&pool, ws_a).await;
    let doc_b = w3::seed_document(&pool, ws_b).await;

    let index = edgequake_storage::PgChunkEmbeddingIndex::new(pool.clone(), "iw5-ann-leak-model");

    let mut rows_a: Vec<EmbeddingRow> = Vec::new();
    let mut ids_b: HashSet<Uuid> = HashSet::new();
    for i in 0..5 {
        let cid_a = w3::seed_chunk(&pool, doc_a, ws_a, i, &format!("a chunk {i}")).await;
        let emb_a = w3::make_embedding(DIM, 800 + i as u32);
        rows_a.push(EmbeddingRow {
            chunk_id: cid_a.into(),
            workspace_id: WorkspaceId(ws_a),
            dimensions: DIM as i32,
            embedding: emb_a,
        });

        let cid_b = w3::seed_chunk(&pool, doc_b, ws_b, i, &format!("b chunk {i}")).await;
        ids_b.insert(cid_b);
        let emb_b = w3::make_embedding(DIM, 900 + i as u32);
        rows_a.push(EmbeddingRow {
            chunk_id: cid_b.into(),
            workspace_id: WorkspaceId(ws_b),
            dimensions: DIM as i32,
            embedding: emb_b,
        });
    }
    index
        .upsert_batch(ModelId(Uuid::nil()), &rows_a)
        .await
        .expect("upsert");

    let query_emb = w3::make_embedding(DIM, 850);
    let hits = index
        .search(&VectorQuery {
            model_id: ModelId(Uuid::nil()),
            workspace_id: Some(WorkspaceId(ws_a)),
            embedding: query_emb,
            limit: 20,
                    allowed_document_ids: None,
})
        .await
        .expect("search a");

    assert!(!hits.is_empty(), "workspace A should have hits");
    for hit in &hits {
        assert!(
            !ids_b.contains(&hit.chunk_id.0),
            "ANN must not return workspace B chunk {:?}",
            hit.chunk_id.0
        );
    }

    w3::cleanup_workspace(&pool, ws_a).await;
    w3::cleanup_workspace(&pool, ws_b).await;
}
