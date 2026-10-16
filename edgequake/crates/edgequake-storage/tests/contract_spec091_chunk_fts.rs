//! SPEC-091 RM2 — typed chunks.content_tsv lexical search (RM-AC-08 lexical).
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;

use edgequake_storage::chunk_fts::search_chunks_fts;
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use uuid::Uuid;

#[tokio::test]
async fn contract_spec091_chunk_fts_hit() {
    let Some(cfg) = require_or_skip_postgres("spec091_rm2_chunk_fts") else {
        return;
    };
    let pool = contract_pg_pool(&cfg).await;

    let has_tsv: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = 'chunks'
              AND column_name = 'content_tsv'
        )",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if !has_tsv {
        eprintln!("skip: migration 136 content_tsv missing");
        return;
    }

    let doc = Uuid::new_v4();
    let ws = Uuid::new_v4();
    // workspace_id FK may fail if workspace row missing — use metadata-only scope.
    sqlx::query(
        "INSERT INTO public.documents (id, title, content, status, metadata)
         VALUES ($1, 'fts-seed', 'seed', 'indexed', jsonb_build_object('workspace_id', $2::text))
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(doc)
    .bind(ws.to_string())
    .execute(&pool)
    .await
    .expect("insert document");

    sqlx::query(
        "INSERT INTO public.chunks (id, document_id, chunk_index, content, metadata)
         VALUES ($1, $2, 0, 'UniqueFtsTokenAlpha123 quantum lattice', '{}'::jsonb)
         ON CONFLICT (document_id, chunk_index) DO UPDATE SET content = EXCLUDED.content",
    )
    .bind(Uuid::new_v4())
    .bind(doc)
    .execute(&pool)
    .await
    .expect("insert chunk");

    let hits = search_chunks_fts(&pool, "UniqueFtsTokenAlpha123", Some(ws), 10)
        .await
        .expect("fts");
    assert!(
        !hits.is_empty(),
        "typed chunk FTS must hit seeded content under workspace filter"
    );
    let foreign = search_chunks_fts(&pool, "UniqueFtsTokenAlpha123", Some(Uuid::new_v4()), 10)
        .await
        .expect("foreign workspace fts");
    assert!(
        foreign.is_empty(),
        "a different workspace must not see the seeded chunk"
    );
}
