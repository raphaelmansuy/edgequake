//! SPEC-180 W4 — images in a `normal` workspace that embeds text and images
//! (Postgres, fake Vertex `:embedContent`; never Google).
//!
//! ```bash
//! DATABASE_URL=postgres://edgequake:edgequake@localhost:5432/edgequake \
//!   cargo test -p edgequake-api --features postgres --test e2e_spec180_normal_image -- --nocapture
//! ```
//!
//! Covers T-180-image-chunk, T-180-image-on-text-ws, T-180-text-to-image,
//! T-180-image-to-image, T-180-image-query-rejected, T-180-image-corrupt,
//! T-180-image-gif and the create-time model and dimension rules.

#![cfg(feature = "postgres")]

mod common;

use axum::http::StatusCode;
use common::spec180_harness::{count, create_workspace, harness, upload_scoped, Harness, USER_ID};
use common::spec180_images::{
    create_image_workspace, create_workspace_raw, png, post_multipart, start_fake_vertex,
    upload_image, upload_image_raw, Part,
};
use serde_json::{json, Value};
use serial_test::serial;

const TOMATO_TEXT: &str = "Tomatoes need warm soil and regular watering through the summer.";

fn first_chunk_doc(response: &Value) -> Option<String> {
    response["sources"]
        .as_array()?
        .iter()
        .find(|s| s["source_type"] == "chunk")?["document_id"]
        .as_str()
        .map(str::to_string)
}

fn chunk_docs(response: &Value) -> Vec<String> {
    response["sources"]
        .as_array()
        .map(|sources| {
            sources
                .iter()
                .filter(|s| s["source_type"] == "chunk")
                .filter_map(|s| s["document_id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

async fn query_json(h: &Harness, tenant: &str, ws: &str, body: Value) -> (StatusCode, Value) {
    common::post_json_with_tenant(&h.app, "/api/v1/query", &body, tenant, USER_ID, ws).await
}

async fn query_image(
    h: &Harness,
    tenant: &str,
    ws: &str,
    query: &str,
    bytes: Vec<u8>,
) -> (StatusCode, Value) {
    let request = json!({"query": query, "context_only": true}).to_string();
    post_multipart(
        h,
        "/api/v1/query",
        tenant,
        ws,
        &[
            Part::Text("request", request),
            Part::File("query_image", "q.png", bytes),
        ],
    )
    .await
}

#[test]
#[serial]
fn an_image_becomes_one_pixel_embedded_chunk_and_is_found_by_text_and_by_image() {
    common::spec180::block_on_big_stack(image_round_trip());
}

async fn image_round_trip() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 image: postgres not reachable");
        return;
    };
    let fake = start_fake_vertex().await;
    let (tenant, ws) = create_image_workspace(&h, None).await;

    // The workspace stores what the wizard chose.
    let (st, got) = common::get_with_tenant(
        &h.app,
        &format!("/api/v1/workspaces/{ws}"),
        &tenant,
        USER_ID,
        &ws,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{got}");
    assert_eq!(got["embedding_modalities"], "text,image", "{got}");
    assert_eq!(got["embedding_dimension"], 1536, "{got}");

    let (kestrel, _) = upload_image(
        &h,
        &tenant,
        &ws,
        "kestrel.png",
        png([200, 60, 20], Some("kestrel")),
    )
    .await;
    let (tomato, _) = upload_image(
        &h,
        &tenant,
        &ws,
        "tomato.png",
        png([220, 30, 30], Some("tomato")),
    )
    .await;
    let (_garden, _, _) = upload_scoped(&h, &tenant, &ws, "garden", TOMATO_TEXT).await;
    assert!(
        fake.hits.load(std::sync::atomic::Ordering::Relaxed) >= 3,
        "the fake was not used"
    );

    // T-180-image-chunk: one chunk, image modality, one vector, no graph, no chat call.
    let image_chunks = count(
        &h.pool,
        "SELECT count(*) FROM chunks WHERE workspace_id = $1 AND source_modality = 'image'",
        &ws,
    )
    .await;
    assert_eq!(image_chunks, 2, "one chunk per image");
    assert_eq!(
        count(
            &h.pool,
            "SELECT count(*) FROM chunk_embeddings WHERE workspace_id = $1",
            &ws
        )
        .await,
        3,
        "one vector per image and one for the text chunk"
    );
    assert_eq!(h.mock.chat_calls(), 0, "ingest called the chat model");

    // T-180-text-to-image: a word names the picture it describes.
    let (st, body) = query_json(
        &h,
        &tenant,
        &ws,
        json!({"query": "kestrel", "context_only": true}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(
        first_chunk_doc(&body).as_deref(),
        Some(kestrel.as_str()),
        "{body}"
    );

    // T-180-image-to-image: another picture of the same thing finds it first.
    let (st, body) = query_image(&h, &tenant, &ws, "", png([10, 10, 10], Some("tomato"))).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(
        first_chunk_doc(&body).as_deref(),
        Some(tomato.as_str()),
        "{body}"
    );

    // Kind filters: only images, then only text.
    let (_, only_images) = query_json(
        &h,
        &tenant,
        &ws,
        json!({
            "query": "tomatoes", "context_only": true,
            "document_filter": {"source_modalities": ["image"]},
        }),
    )
    .await;
    let image_docs = chunk_docs(&only_images);
    assert!(
        !image_docs.is_empty(),
        "image filter found nothing: {only_images}"
    );
    assert!(
        image_docs.iter().all(|d| *d == kestrel || *d == tomato),
        "{only_images}"
    );
    let (_, only_text) = query_json(
        &h,
        &tenant,
        &ws,
        json!({
            "query": "tomatoes", "context_only": true,
            "document_filter": {"doc_kinds": ["markdown"]},
        }),
    )
    .await;
    let docs = chunk_docs(&only_text);
    assert!(!docs.is_empty(), "text filter found nothing: {only_text}");
    assert!(
        !docs.contains(&kestrel) && !docs.contains(&tomato),
        "text filter returned an image: {only_text}"
    );
}

#[test]
#[serial]
fn images_are_refused_where_the_workspace_cannot_embed_them() {
    common::spec180::block_on_big_stack(refusals());
}

async fn refusals() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 image refusals: postgres not reachable");
        return;
    };
    let _fake = start_fake_vertex().await;

    // T-180-image-on-text-ws and T-180-image-query-rejected.
    let (tenant, text_ws) = create_workspace(&h, "normal").await;
    let (st, body) = upload_image_raw(&h, &tenant, &text_ws, "a.png", png([1, 2, 3], None)).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "embedding_model_not_multimodal", "{body}");
    let (st, body) = query_image(&h, &tenant, &text_ws, "x", png([1, 2, 3], None)).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "image_query_not_supported", "{body}");

    // T-180-image-gif, T-180-image-corrupt and an invalid query image.
    let (tenant, ws) = create_image_workspace(&h, Some(768)).await;
    let (st, body) = upload_image_raw(
        &h,
        &tenant,
        &ws,
        "a.gif",
        b"GIF89a\x01\x00\x01\x00".to_vec(),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "image_animated_unsupported", "{body}");
    let (st, body) = upload_image_raw(
        &h,
        &tenant,
        &ws,
        "bad.png",
        b"\x89PNG\r\n\x1a\nnot really".to_vec(),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "image_decode_failed", "{body}");
    let (st, body) = query_image(&h, &tenant, &ws, "x", b"just text".to_vec()).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "query_image_invalid", "{body}");
}

#[test]
#[serial]
fn create_checks_the_embedding_model_and_the_dimension() {
    common::spec180::block_on_big_stack(create_rules());
}

async fn create_rules() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 image create rules: postgres not reachable");
        return;
    };
    let _fake = start_fake_vertex().await;
    let (st, tenant) = common::post_json(
        &h.app,
        "/api/v1/tenants",
        &json!({"name": format!("spec180 rules {}", uuid::Uuid::new_v4().simple())}),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{tenant}");
    let tenant = tenant["id"].as_str().unwrap().to_string();
    let base = |model: &str, provider: &str, dim: Option<u64>| {
        json!({
            "name": format!("w-{}", uuid::Uuid::new_v4().simple()),
            "rag_profile": "normal", "embedding_modalities": "text,image",
            "embedding_provider": provider, "embedding_model": model, "embedding_dimension": dim,
        })
    };

    // A text-only model cannot back an image workspace.
    let (st, err) =
        create_workspace_raw(&h, &tenant, base("text-embedding-3-small", "openai", None)).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{err}");
    assert!(
        ["embedding_model_not_multimodal", "model_not_eligible"]
            .contains(&err["code"].as_str().unwrap_or("")),
        "{err}"
    );
    // The multimodal model serves 768, 1536 and 3072 only.
    let (st, err) = create_workspace_raw(
        &h,
        &tenant,
        base("gemini-embedding-2", "vertexai", Some(1000)),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{err}");
    assert_eq!(err["code"], "embedding_dimension_rejected", "{err}");
    let (st, ok) = create_workspace_raw(
        &h,
        &tenant,
        base("gemini-embedding-2", "vertexai", Some(3072)),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{ok}");
    assert_eq!(ok["embedding_dimension"], 3072, "{ok}");

    // A probe that fails is reported at create time, not on the first upload.
    std::env::set_var("EDGEQUAKE_VERTEX_BASE_URL", "http://127.0.0.1:9");
    let (st, err) = create_workspace_raw(
        &h,
        &tenant,
        base("gemini-embedding-2", "vertexai", Some(768)),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{err}");
    assert_eq!(err["code"], "embedding_auth", "{err}");
}
