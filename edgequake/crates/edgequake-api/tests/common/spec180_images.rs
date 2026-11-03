//! SPEC-180 image e2e helpers: a fake Vertex server, PNG fixtures, multipart requests.

#![cfg(feature = "postgres")]
#![allow(dead_code)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use super::spec180_harness::{wait_terminal, Harness, USER_ID};

/// The Google Cloud project that may call Vertex `gemini-embedding-2`.
///
/// SPEC-180 LAW-180-15: the project is pinned by the harness, never inherited from a
/// developer shell. `saas-app-001` is a different project that answers 403 for this model,
/// so an inherited export makes a live test fail for a reason that has nothing to do with
/// the code under test.
pub const LIVE_PROJECT: &str = "mobioos-506015";

/// A running fake `:embedContent` server. Dropping it stops the server.
pub struct FakeVertex {
    pub hits: std::sync::Arc<std::sync::atomic::AtomicU64>,
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for FakeVertex {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Start the fake and point the Vertex client at it. The project and the token are fakes:
/// the test never reaches Google (LAW-180-15).
pub async fn start_fake_vertex() -> FakeVertex {
    let state = edgequake_fake_llm::FakeLlmState::default();
    let hits = std::sync::Arc::clone(&state.hits);
    let (addr, handle) = edgequake_fake_llm::spawn_ephemeral(state)
        .await
        .expect("fake vertex");
    std::env::set_var("EDGEQUAKE_VERTEX_BASE_URL", format!("http://{addr}"));
    std::env::set_var("GOOGLE_CLOUD_PROJECT", "fake-project");
    std::env::set_var("GOOGLE_ACCESS_TOKEN", "fake-token");
    std::env::remove_var("EDGEQUAKE_SKIP_EMBED_PROBE");
    // WHY: the image probe is memoised per `(provider, model)` for five minutes, process-wide.
    // A test that starts this fake, probes successfully, then points at a dead port to assert
    // `embedding_auth` would read the earlier *success* out of that cache and see 201 instead of
    // 422 — which is exactly what happened, and it looked like a product defect. Starting a
    // fresh vendor double means every cached answer from a previous test is stale.
    edgequake_api::services::embed_capability_probe::forget_all();
    FakeVertex { hits, handle }
}

/// A small solid PNG. `concept` is appended after the image data, where decoders stop:
/// the fake embedder reads it as the image's meaning (`concept:<word>;`).
pub fn png(rgb: [u8; 3], concept: Option<&str>) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(16, 16, image::Rgb(rgb));
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .expect("png");
    if let Some(word) = concept {
        bytes.extend_from_slice(format!("concept:{word};").as_bytes());
    }
    bytes
}

/// A tenant and a `normal` workspace that embeds text and images, created through the API.
pub async fn create_image_workspace(h: &Harness, dimension: Option<u64>) -> (String, String) {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (st, tenant) = super::post_json(
        &h.app,
        "/api/v1/tenants",
        &json!({"name": format!("spec180 img {suffix}")}),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "tenant: {tenant}");
    let tenant_id = tenant["id"].as_str().unwrap().to_string();
    let (st, ws) = create_workspace_raw(
        h,
        &tenant_id,
        json!({
            "name": format!("img {suffix}"),
            "rag_profile": "normal",
            "llm_provider": "mock",
            "llm_model": "mock-model",
            "embedding_modalities": "text,image",
            "embedding_provider": "vertexai",
            "embedding_model": "gemini-embedding-2",
            "embedding_dimension": dimension,
        }),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "workspace: {ws}");
    (
        tenant_id,
        ws["id"].as_str().expect("workspace id").to_string(),
    )
}

pub async fn create_workspace_raw(h: &Harness, tenant: &str, body: Value) -> (StatusCode, Value) {
    super::post_json(
        &h.app,
        &format!("/api/v1/tenants/{tenant}/workspaces"),
        &body,
    )
    .await
}

/// One part of a multipart body.
pub enum Part<'a> {
    Text(&'a str, String),
    File(&'a str, &'a str, Vec<u8>),
}

fn multipart_body(boundary: &str, parts: &[Part<'_>]) -> Vec<u8> {
    let mut body = Vec::new();
    for part in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match part {
            Part::Text(name, value) => {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                        .as_bytes(),
                );
            }
            Part::File(name, filename, bytes) => {
                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
                    )
                    .as_bytes(),
                );
                body.extend_from_slice(bytes);
                body.extend_from_slice(b"\r\n");
            }
        }
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

pub async fn post_multipart(
    h: &Harness,
    uri: &str,
    tenant: &str,
    ws: &str,
    parts: &[Part<'_>],
) -> (StatusCode, Value) {
    let boundary = "----spec180boundary";
    let response = h
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("X-Tenant-ID", tenant)
                .header("X-User-ID", USER_ID)
                .header("X-Workspace-ID", ws)
                .body(Body::from(multipart_body(boundary, parts)))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, super::extract_json(response).await)
}

/// Upload an image file; returns the admission status and body.
pub async fn upload_image_raw(
    h: &Harness,
    tenant: &str,
    ws: &str,
    filename: &str,
    bytes: Vec<u8>,
) -> (StatusCode, Value) {
    post_multipart(
        h,
        "/api/v1/documents/upload",
        tenant,
        ws,
        &[Part::File("file", filename, bytes)],
    )
    .await
}

/// Upload an image and wait for it to finish; returns the document id and its final body.
pub async fn upload_image(
    h: &Harness,
    tenant: &str,
    ws: &str,
    filename: &str,
    bytes: Vec<u8>,
) -> (String, Value) {
    let (st, body) = upload_image_raw(h, tenant, ws, filename, bytes).await;
    assert!(st.is_success(), "image upload: {st} {body}");
    let doc_id = body["document_id"]
        .as_str()
        .expect("document_id")
        .to_string();
    let (status, doc) = wait_terminal(h, tenant, ws, &doc_id).await;
    assert_eq!(status, "completed", "image document ended {status}: {doc}");
    (doc_id, doc)
}
