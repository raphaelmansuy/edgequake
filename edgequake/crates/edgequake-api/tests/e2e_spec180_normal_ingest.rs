//! SPEC-180 W2 — a `normal` workspace ingests chunks only (Postgres, mock providers).
//!
//! ```bash
//! DATABASE_URL=postgres://edgequake:edgequake@localhost:5432/edgequake \
//!   cargo test -p edgequake-api --test e2e_spec180_normal_ingest -- --nocapture
//! ```
//!
//! Covers T-180-ingest-zero-chat, T-180-upload-mode-rejected, T-180-reprocess-mode,
//! T-180-completed-zero-entities and T-180-df-delete (BM25 postings follow the chunks).

#![cfg(feature = "postgres")]

mod common;

use std::time::Duration;

use axum::http::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

use common::spec180_harness::{count, create_workspace, harness, upload_scoped, USER_ID};

const DOC_TEXT: &str = "Quarterly revenue grew twelve percent. The Zurich office hired \
    forty engineers. Zurich remains the largest European site for the research lab.";

#[test]
#[serial]
fn normal_workspace_ingests_chunks_without_chat_or_graph() {
    common::spec180::block_on_big_stack(
        normal_workspace_ingests_chunks_without_chat_or_graph_body(),
    );
}

async fn normal_workspace_ingests_chunks_without_chat_or_graph_body() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 normal ingest: postgres not reachable");
        return;
    };
    let (tenant, ws) = create_workspace(&h, "normal").await;

    let (st, view) = common::get_with_tenant(
        &h.app,
        &format!("/api/v1/workspaces/{ws}"),
        &tenant,
        USER_ID,
        &ws,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{view}");
    assert_eq!(view["rag_profile"], "normal", "{view}");
    assert_eq!(
        view["rag_profile_capabilities"]["extracts_graph"], false,
        "{view}"
    );

    // T-180-upload-mode-rejected: an extraction mode means nothing without a graph.
    let (st, err) = common::post_json_with_tenant(
        &h.app,
        "/api/v1/documents",
        &json!({"content": DOC_TEXT, "title": "mode", "metadata": {"extraction_mode": "decision"}}),
        &tenant,
        USER_ID,
        &ws,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{err}");
    assert_eq!(err["code"], "extraction_mode_not_applicable", "{err}");

    let (doc_id, track_id, status) = upload_scoped(&h, &tenant, &ws, "revenue", DOC_TEXT).await;
    assert_eq!(status, "completed", "doc {doc_id} track {track_id}");

    // T-180-ingest-zero-chat.
    assert_eq!(
        h.mock.chat_calls(),
        0,
        "a normal ingest called the chat model"
    );

    // T-180-completed-zero-entities: chunks exist, the graph does not.
    let chunks = count(
        &h.pool,
        "SELECT count(*) FROM chunks WHERE workspace_id = $1",
        &ws,
    )
    .await;
    assert!(chunks > 0, "no chunks written");
    let embeddings = count(
        &h.pool,
        "SELECT count(*) FROM chunk_embeddings WHERE workspace_id = $1",
        &ws,
    )
    .await;
    assert_eq!(embeddings, chunks, "every chunk is embedded");
    let postings = count(
        &h.pool,
        "SELECT count(*) FROM chunk_terms WHERE workspace_id = $1",
        &ws,
    )
    .await;
    assert!(postings > 0, "BM25 postings missing");
    let (docs, total_len): (i64, i64) = sqlx::query_as(
        "SELECT bm25_doc_count::bigint, bm25_total_len::bigint FROM workspaces WHERE workspace_id = $1",
    )
    .bind(uuid::Uuid::parse_str(&ws).unwrap())
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!(docs, chunks, "corpus statistics follow the chunks");
    assert!(total_len > 0);

    // T-180-reprocess-mode and rebuild: graph-only actions are 409.
    let (st, err) = common::post_json_with_tenant(
        &h.app,
        "/api/v1/documents/reprocess",
        &json!({"mode": "entities_only", "document_id": doc_id}),
        &tenant,
        USER_ID,
        &ws,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{err}");
    assert_eq!(err["code"], "reprocess_mode_not_applicable", "{err}");

    let (st, err) = common::post_json_with_tenant(
        &h.app,
        &format!("/api/v1/workspaces/{ws}/rebuild-knowledge-graph"),
        &json!({}),
        &tenant,
        USER_ID,
        &ws,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{err}");
    assert_eq!(err["code"], "profile_has_no_graph", "{err}");

    // T-180-df-delete: deleting the document retracts chunks and postings together.
    let (st, body) = common::delete_with_tenant(
        &h.app,
        &format!("/api/v1/documents/{doc_id}"),
        &tenant,
        USER_ID,
        &ws,
    )
    .await;
    assert!(st.is_success(), "delete: {st} {body}");
    let mut left = chunks;
    for _ in 0..80 {
        left = count(
            &h.pool,
            "SELECT count(*) FROM chunks WHERE workspace_id = $1",
            &ws,
        )
        .await;
        if left == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert_eq!(left, 0, "chunks survived delete");
    assert_eq!(
        count(
            &h.pool,
            "SELECT count(*) FROM chunk_terms WHERE workspace_id = $1",
            &ws
        )
        .await,
        0
    );
    let (docs, total_len): (i64, i64) = sqlx::query_as(
        "SELECT bm25_doc_count::bigint, bm25_total_len::bigint FROM workspaces WHERE workspace_id = $1",
    )
    .bind(uuid::Uuid::parse_str(&ws).unwrap())
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!((docs, total_len), (0, 0));

    edgequake_api::safety_limits::clear_test_provider_override();
}

/// A graph workspace keeps today's behavior: no profile word is needed and the
/// extraction-mode word is still accepted (LAW-180-1).
#[test]
#[serial]
fn graph_workspace_still_accepts_extraction_mode_word() {
    common::spec180::block_on_big_stack(graph_workspace_still_accepts_extraction_mode_word_body());
}

async fn graph_workspace_still_accepts_extraction_mode_word_body() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 graph regression: postgres not reachable");
        return;
    };
    let (tenant, ws) = create_workspace(&h, "graph").await;
    let (st, body) = common::post_json_with_tenant(
        &h.app,
        "/api/v1/documents",
        &json!({"content": DOC_TEXT, "title": "g", "metadata": {"extraction_mode": "bogus"}}),
        &tenant,
        USER_ID,
        &ws,
    )
    .await;
    // The word is validated (422), never rejected as "not applicable" (400).
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "invalid_extraction_mode", "{body}");
    edgequake_api::safety_limits::clear_test_provider_override();
}

/// API create/update rules over HTTP (T-180-create-*, T-180-immutable).
#[test]
#[serial]
fn workspace_create_validates_profile_words() {
    common::spec180::block_on_big_stack(workspace_create_validates_profile_words_body());
}

async fn workspace_create_validates_profile_words_body() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 create rules: postgres not reachable");
        return;
    };
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (_, tenant) = common::post_json(
        &h.app,
        "/api/v1/tenants",
        &json!({"name": format!("spec180 words {suffix}")}),
    )
    .await;
    let tenant_id = tenant["id"].as_str().unwrap().to_string();
    let uri = format!("/api/v1/tenants/{tenant_id}/workspaces");
    let post = |body: Value| {
        let app = h.app.clone();
        let uri = uri.clone();
        async move { common::post_json(&app, &uri, &body).await }
    };

    let (st, err) = post(json!({"name": "a", "rag_profile": "hybrid"})).await;
    assert_eq!(
        (st, err["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalid_rag_profile")),
        "{err}"
    );

    let (st, err) = post(json!({"name": "b", "embedding_modalities": "text,image"})).await;
    assert_eq!(
        (st, err["code"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("modalities_require_normal")
        ),
        "{err}"
    );

    let (st, ws) =
        post(json!({"name": "c", "rag_profile": "normal", "fts_regconfig": "french"})).await;
    assert_eq!(st, StatusCode::CREATED, "{ws}");
    assert_eq!(ws["fts_regconfig"], "french");
    let ws_id = ws["id"].as_str().unwrap();

    // Immutable: even an unchanged value is a 409.
    for body in [
        json!({"rag_profile": "normal"}),
        json!({"fts_regconfig": "french"}),
    ] {
        let response = common::put_with_tenant(
            &h.app,
            &format!("/api/v1/workspaces/{ws_id}"),
            &body,
            &tenant_id,
            USER_ID,
            ws_id,
        )
        .await;
        assert_eq!(response.0, StatusCode::CONFLICT, "{}", response.1);
    }
    edgequake_api::safety_limits::clear_test_provider_override();
}
