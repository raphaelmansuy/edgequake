//! #405 / SPEC-091: typed chunk FTS must not SELECT retired `eq_*_vectors`.
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-storage --features postgres --test e2e_spec405_typed_fts_no_legacy -- --nocapture
#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/spec091_w3.rs"]
mod w3;

use edgequake_storage::adapters::postgres::PostgresPool;
use edgequake_storage::traits::{MetadataFilter, VectorStorage};
use edgequake_storage::{PgVectorStorage, VECTOR_BACKEND_ENV};
use postgres_test_config::{contract_pg_pool, require_or_skip_postgres};
use uuid::Uuid;

const DIM: usize = 4;
const TOKEN: &str = "UniqueFtsToken405Alpha";

async fn table_exists(pool: &sqlx::PgPool, table: &str) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = $1)",
    )
    .bind(table)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn has_content_tsv(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = 'chunks'
              AND column_name = 'content_tsv'
        )",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

async fn mark_ready(pool: &sqlx::PgPool, chunk_id: Uuid) {
    sqlx::query(
        "INSERT INTO public.chunk_serving_state (chunk_id, state) VALUES ($1, 'ready') \
         ON CONFLICT (chunk_id) DO UPDATE SET state = 'ready'",
    )
    .bind(chunk_id)
    .execute(pool)
    .await
    .expect("seed serving ready");
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn e2e_spec405_typed_fts_without_legacy_vectors() {
    let Some(cfg) = require_or_skip_postgres("spec405_typed_fts") else {
        return;
    };
    let _env = w3::w3_env_guard().await;
    let pool = contract_pg_pool(&cfg).await;

    for table in ["chunks", "documents", "workspaces", "chunk_serving_state"] {
        if !table_exists(&pool, table).await {
            eprintln!("skip: typed table {table} missing — run migrations");
            return;
        }
    }
    if !has_content_tsv(&pool).await {
        eprintln!("skip: migration 136 content_tsv missing");
        return;
    }

    let prev_backend = std::env::var(VECTOR_BACKEND_ENV).ok();
    std::env::set_var(VECTOR_BACKEND_ENV, "typed_embeddings");

    let ws = w3::seed_workspace(&pool, "spec405").await;
    let doc = w3::seed_document(&pool, ws).await;
    let prose_id = w3::seed_chunk(
        &pool,
        doc,
        ws,
        0,
        &format!("{TOKEN} quantum lattice prose narrative"),
    )
    .await;
    let chart_id = {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO chunks (id, document_id, workspace_id, chunk_index, content, metadata) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(doc)
        .bind(ws)
        .bind(1_i32)
        .bind(format!("{TOKEN} chart revenue 42 million USD"))
        .bind(serde_json::json!({
            "legacy_chunk_key": format!("{doc}-chunk-1"),
            "modality": "chart"
        }))
        .execute(&pool)
        .await
        .expect("seed chart chunk");
        id
    };
    mark_ready(&pool, prose_id).await;
    mark_ready(&pool, chart_id).await;

    let prose_key = format!("{doc}-chunk-0");
    let chart_key = format!("{doc}-chunk-1");

    let mut pg_cfg = cfg.clone();
    pg_cfg.namespace = format!("spec405_ws_{}", ws.as_simple());
    let storage = PgVectorStorage::with_pool(
        PostgresPool::from_existing(pool.clone(), pg_cfg.clone()),
        pg_cfg,
        DIM,
    );
    let legacy_table = storage
        .vectors_table_name()
        .trim_start_matches("public.")
        .to_string();
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "precondition: {legacy_table} must be absent (no CREATE)"
    );

    // 1) Happy path — typed FTS hits without 42P01.
    let hits = storage
        .text_search_filtered(
            TOKEN,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("typed FTS must succeed without legacy vectors table");
    assert!(
        hits.iter().any(|h| h.id == prose_key && h.score > 0.0),
        "expected prose legacy_chunk_key hit, got {hits:?}"
    );
    assert!(
        hits.iter().any(|h| h.id == chart_key && h.score > 0.0),
        "expected chart legacy_chunk_key hit, got {hits:?}"
    );
    assert!(
        !table_exists(&pool, &legacy_table).await,
        "typed FTS must not CREATE {legacy_table}"
    );

    // 2) Non-chunk vector_type → empty.
    let entity_hits = storage
        .text_search_filtered(
            TOKEN,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                vector_type: Some("entity".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("entity FTS");
    assert!(
        entity_hits.is_empty(),
        "non-chunk vector_type must short-circuit empty, got {entity_hits:?}"
    );

    // 3) Unresolvable workspace name → empty.
    let bad_ws = storage
        .text_search_filtered(
            TOKEN,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some("no-such-workspace-405-xyz".into()),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("bad workspace");
    assert!(
        bad_ws.is_empty(),
        "unresolvable workspace must return empty, got {bad_ws:?}"
    );

    // 4) Non-UUID document filter → empty.
    let bad_docs = storage
        .text_search_filtered(
            TOKEN,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                document_ids: Some(vec!["not-a-uuid".into()]),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("bad document_ids");
    assert!(
        bad_docs.is_empty(),
        "non-UUID document_ids must fail closed empty, got {bad_docs:?}"
    );

    // 5) Modality filter → chart only.
    let chart_hits = storage
        .text_search_filtered(
            TOKEN,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                vector_type: Some("chunk".into()),
                modalities: Some(vec!["chart".into()]),
                ..Default::default()
            }),
        )
        .await
        .expect("modality FTS");
    assert_eq!(
        chart_hits.len(),
        1,
        "modality=chart must return only chart row, got {chart_hits:?}"
    );
    assert_eq!(chart_hits[0].id, chart_key);

    // 6) filter_ids via legacy_chunk_key.
    let filtered = storage
        .text_search_filtered(
            TOKEN,
            10,
            Some(std::slice::from_ref(&prose_key)),
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("filter_ids FTS");
    assert_eq!(
        filtered.len(),
        1,
        "filter_ids must restrict hits, got {filtered:?}"
    );
    assert_eq!(filtered[0].id, prose_key);

    // 6b) document_ids must not return another document that shares the token.
    let other_doc = w3::seed_document(&pool, ws).await;
    let other_id = w3::seed_chunk(
        &pool,
        other_doc,
        ws,
        0,
        &format!("{TOKEN} other document shares the term"),
    )
    .await;
    mark_ready(&pool, other_id).await;
    let other_key = format!("{other_doc}-chunk-0");
    let scoped_docs = storage
        .text_search_filtered(
            TOKEN,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                document_ids: Some(vec![doc.to_string()]),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("document-scoped FTS");
    assert!(
        scoped_docs.iter().any(|h| h.id == prose_key),
        "document scope must keep the requested document, got {scoped_docs:?}"
    );
    assert!(
        scoped_docs.iter().all(|h| h.id != other_key),
        "document scope must drop the other document, got {scoped_docs:?}"
    );

    // 7) Explicit legacy_tables + missing relation → Err without SELECT 42P01.
    std::env::set_var(VECTOR_BACKEND_ENV, "legacy_tables");
    let legacy_err = storage
        .text_search_filtered(
            TOKEN,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await;
    let err = legacy_err.expect_err("legacy_tables with absent relation must Err");
    let msg = err.to_string();
    assert!(
        msg.contains("absent") && msg.contains(&legacy_table)
            || msg.contains("legacy vectors relation"),
        "preflight error must name absent relation, got: {msg}"
    );
    assert!(
        !msg.contains("42P01"),
        "must not surface SQLSTATE 42P01 from a failed SELECT: {msg}"
    );

    match prev_backend {
        Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
        None => std::env::remove_var(VECTOR_BACKEND_ENV),
    }
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::await_holding_lock)]
async fn tenant_chunk_search_skips_the_other_tenants_term_matches() {
    let Some(cfg) = require_or_skip_postgres("spec405_tenant_fts") else {
        return;
    };
    let _env = w3::w3_env_guard().await;
    let pool = contract_pg_pool(&cfg).await;
    if !has_content_tsv(&pool).await {
        eprintln!("skip: migration 136 content_tsv missing");
        return;
    }

    let prev_backend = std::env::var(VECTOR_BACKEND_ENV).ok();
    std::env::set_var(VECTOR_BACKEND_ENV, "typed_embeddings");

    let ws = w3::seed_workspace(&pool, "spec405-tenant").await;
    let doc = w3::seed_document(&pool, ws).await;
    let own_tenant = w3::workspace_tenant(&pool, ws).await;
    let token = format!("TenantFence{}", Uuid::new_v4().simple());
    let own_id = w3::seed_chunk(&pool, doc, ws, 0, &format!("{token} owned by the caller")).await;
    let other_tenant = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (tenant_id, name, slug, is_active) VALUES ($1, $2, $3, TRUE)")
        .bind(other_tenant)
        .bind("spec405 other")
        .bind(format!("spec405-{}", other_tenant.as_simple()))
        .execute(&pool)
        .await
        .expect("other tenant");
    let other_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO chunks (id, document_id, workspace_id, chunk_index, content, metadata, tenant_id) \
         VALUES ($1, $2, $3, 1, $4, $5, $6)",
    )
    .bind(other_id)
    .bind(doc)
    .bind(ws)
    .bind(format!("{token} owned by someone else"))
    .bind(serde_json::json!({"legacy_chunk_key": format!("{doc}-chunk-1")}))
    .bind(other_tenant)
    .execute(&pool)
    .await
    .expect("other chunk");
    mark_ready(&pool, own_id).await;
    mark_ready(&pool, other_id).await;

    let mut pg_cfg = cfg.clone();
    pg_cfg.namespace = format!("spec405_tenant_{}", ws.as_simple());
    let storage = PgVectorStorage::with_pool(
        PostgresPool::from_existing(pool.clone(), pg_cfg.clone()),
        pg_cfg,
        DIM,
    );
    let own_key = format!("{doc}-chunk-0");
    let other_key = format!("{doc}-chunk-1");
    let scoped = storage
        .text_search_filtered(
            &token,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                tenant_id: Some(own_tenant.to_string()),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("tenant FTS");
    let open = storage
        .text_search_filtered(
            &token,
            10,
            None,
            Some(&MetadataFilter {
                workspace_id: Some(ws.to_string()),
                vector_type: Some("chunk".into()),
                ..Default::default()
            }),
        )
        .await
        .expect("workspace FTS");

    sqlx::query("DELETE FROM chunks WHERE id = $1 OR id = $2")
        .bind(own_id)
        .bind(other_id)
        .execute(&pool)
        .await
        .expect("cleanup chunks");
    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(doc)
        .execute(&pool)
        .await
        .expect("cleanup document");
    sqlx::query("DELETE FROM workspaces WHERE workspace_id = $1")
        .bind(ws)
        .execute(&pool)
        .await
        .expect("cleanup workspace");
    sqlx::query("DELETE FROM tenants WHERE tenant_id = $1 OR tenant_id = $2")
        .bind(own_tenant)
        .bind(other_tenant)
        .execute(&pool)
        .await
        .expect("cleanup tenants");

    match prev_backend {
        Some(v) => std::env::set_var(VECTOR_BACKEND_ENV, v),
        None => std::env::remove_var(VECTOR_BACKEND_ENV),
    }

    assert!(
        scoped.iter().any(|hit| hit.id == own_key),
        "caller tenant must keep its chunk, got {scoped:?}"
    );
    assert!(
        scoped.iter().all(|hit| hit.id != other_key),
        "caller tenant must skip the other tenant's chunk, got {scoped:?}"
    );
    assert!(
        open.iter().any(|hit| hit.id == own_key) && open.iter().any(|hit| hit.id == other_key),
        "workspace search without a tenant must keep both chunks, got {open:?}"
    );
}
