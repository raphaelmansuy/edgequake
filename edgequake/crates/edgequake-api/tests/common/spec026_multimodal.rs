//! SPEC-026 Phase 4 multimodal E2E fixtures and helpers (DRY SSOT).
//!
//! LightRAG parity: mock VLM JSON matches `prompt_multimodal.py` image_analysis schema.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use serde_json::Value;
use std::path::PathBuf;

/// Minimal valid 1×1 PNG (67 bytes).
pub const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

/// Mock VLM response (LightRAG `{name,type,description}` schema).
pub const MOCK_VLM_SARAH_JSON: &str = r#"{"name":"sarah_chen_profile","type":"Photo","description":"Dr. Sarah Chen leads EdgeQuake research in Zurich."}"#;

/// Base64 of [`TINY_PNG`] for data-URI markdown fixtures.
pub const TINY_PNG_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

/// Enable inline VLM analyze (LightRAG `VLM_PROCESS_ENABLE=true`; default is off).
pub fn enable_vlm_process_in_tests() {
    std::env::set_var("VLM_PROCESS_ENABLE", "true");
}

/// Lower pixel gate so 1×1 fixture PNG can reach mock VLM in E2E.
pub fn allow_tiny_images_in_tests() {
    enable_vlm_process_in_tests();
    std::env::set_var("VLM_MIN_IMAGE_PIXEL", "1");
}

pub fn restore_vlm_image_limits() {
    std::env::remove_var("VLM_MIN_IMAGE_PIXEL");
    std::env::remove_var("VLM_PROCESS_ENABLE");
}

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spec026")
        .join(name)
}

pub fn load_fixture_utf8(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("missing fixture {name}: {e}"))
}

/// Build multipart body for PNG upload to `/documents/upload`.
pub fn build_png_multipart(boundary: &str, filename: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(TINY_PNG);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

/// POST multipart PNG to `/documents/upload`.
pub fn png_upload_request(boundary: &str, filename: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/documents/upload")
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(build_png_multipart(boundary, filename)))
        .unwrap()
}

/// POST JSON text document to `/api/v1/documents`.
pub fn text_upload_request(title: &str, content: &str) -> Request<Body> {
    let body = serde_json::json!({
        "content": content,
        "title": title,
    });
    Request::builder()
        .method("POST")
        .uri("/api/v1/documents")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Markdown simulating post-convert PDF with embedded data-URI (LightRAG drawing surrogate).
pub fn markdown_with_data_uri_image() -> String {
    format!(
        "# Report\n\nSee chart:\n\n![inline chart](data:image/png;base64,{TINY_PNG_B64})\n\nEnd.\n"
    )
}

/// Markdown with LightRAG native `<drawing/>` placeholder (no sidecar asset yet).
pub fn markdown_with_drawing_tag() -> String {
    "# Report\n\n<drawing id=\"im-spec026-0001\" format=\"png\" caption=\"Chart\" />\n\nEnd.\n"
        .to_string()
}

pub async fn response_body_bytes(response: Response) -> axum::body::Bytes {
    axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap()
}

/// Parse a 202 Accepted upload response.
pub async fn parse_accepted_upload(response: Response) -> (String, String) {
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let parsed: Value = serde_json::from_slice(&response_body_bytes(response).await).unwrap();
    (
        parsed["document_id"].as_str().unwrap().to_string(),
        parsed["track_id"].as_str().unwrap().to_string(),
    )
}
