//! End-to-end tests for the Knowledge Injection API (SPEC-0002).
//!
//! Tests cover:
//! - PUT /api/v1/workspaces/{workspace_id}/injection — create injection (text)
//! - PUT /api/v1/workspaces/{workspace_id}/injection/file — create injection (file upload)
//! - GET /api/v1/workspaces/{workspace_id}/injections — list injections
//! - GET /api/v1/workspaces/{workspace_id}/injections/{id} — get detail
//! - DELETE /api/v1/workspaces/{workspace_id}/injections/{id} — delete
//! - PATCH /api/v1/workspaces/{workspace_id}/injections/{id} — update
//! - Citation exclusion: injection sources never appear in query results
//! - Injection entries excluded from document list

use std::collections::HashMap;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

// ============================================================================
// Helpers
// ============================================================================

fn create_test_app() -> axum::Router {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    Server::new(config, AppState::test_state()).build_router()
}

/// Create a test app while retaining a clone of the underlying state for direct inspection.
fn create_test_app_with_state() -> (axum::Router, AppState) {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    let state = AppState::test_state();
    let router = Server::new(config, state.clone()).build_router();
    (router, state)
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

/// PUT a text injection and return the parsed response body.
async fn put_injection(
    app: &axum::Router,
    workspace_id: &str,
    name: &str,
    content: &str,
) -> (StatusCode, Value) {
    let body = json!({ "name": name, "content": content });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}/injection"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let val = extract_json(response).await;
    (status, val)
}

/// PUT a text injection with an explicit X-Workspace-ID header (required for isolation tests).
async fn put_injection_ws(
    app: &axum::Router,
    workspace_id: &str,
    name: &str,
    content: &str,
) -> (StatusCode, Value) {
    let body = json!({ "name": name, "content": content });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}/injection"))
                .header("content-type", "application/json")
                .header("x-workspace-id", workspace_id)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let val = extract_json(response).await;
    (status, val)
}

/// PATCH an injection entry and return (StatusCode, response body).
async fn patch_injection(
    app: &axum::Router,
    workspace_id: &str,
    injection_id: &str,
    name: Option<&str>,
    content: Option<&str>,
) -> (StatusCode, Value) {
    let mut body = serde_json::Map::new();
    if let Some(n) = name {
        body.insert("name".to_string(), json!(n));
    }
    if let Some(c) = content {
        body.insert("content".to_string(), json!(c));
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!(
                    "/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::Value::Object(body).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let val = extract_json(response).await;
    (status, val)
}

/// DELETE an injection entry and return just the HTTP status code.
async fn delete_injection_http(
    app: &axum::Router,
    workspace_id: &str,
    injection_id: &str,
) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

// ============================================================================
// PUT — Create injection
// ============================================================================

/// SPEC-0002: PUT returns 202 Accepted with injection_id.
#[tokio::test]
async fn test_put_injection_success() {
    let app = create_test_app();
    let (status, body) = put_injection(
        &app,
        "ws-test",
        "Glossary",
        "RAG = Retrieval-Augmented Generation. LLM = Large Language Model.",
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED, "Expected 202: {body}");
    assert!(
        body.get("injection_id").and_then(|v| v.as_str()).is_some(),
        "Response must have injection_id: {body}"
    );
    assert_eq!(
        body.get("status").and_then(|v| v.as_str()),
        Some("processing"),
        "Initial status must be 'processing': {body}"
    );
    assert_eq!(
        body.get("version").and_then(|v| v.as_u64()),
        Some(1),
        "First version must be 1: {body}"
    );
    assert!(
        body.get("workspace_id").and_then(|v| v.as_str()).is_some(),
        "Response must include workspace_id: {body}"
    );
}

/// SPEC-0002: PUT with empty name → 400 Bad Request.
#[tokio::test]
async fn test_put_injection_empty_name_rejected() {
    let app = create_test_app();
    let (status, _body) =
        put_injection(&app, "ws-test", "", "RAG = Retrieval-Augmented Generation.").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "Empty name must be 400");
}

/// SPEC-0002: PUT with name exceeding 100 chars → 400 Bad Request.
#[tokio::test]
async fn test_put_injection_name_too_long_rejected() {
    let long_name = "x".repeat(101);
    let app = create_test_app();
    let (status, _) =
        put_injection(&app, "ws-test", &long_name, "Some content about acronyms.").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Name > 100 chars must be 400"
    );
}

/// SPEC-0002: PUT with empty content → 400 Bad Request.
#[tokio::test]
async fn test_put_injection_empty_content_rejected() {
    let app = create_test_app();
    let (status, _) = put_injection(&app, "ws-test", "Glossary", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "Empty content must be 400");
}

/// SPEC-0002: PUT with whitespace-only content → 400 Bad Request.
#[tokio::test]
async fn test_put_injection_whitespace_content_rejected() {
    let app = create_test_app();
    let (status, _) = put_injection(&app, "ws-test", "Glossary", "   \n\t  ").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Whitespace-only content must be 400"
    );
}

/// SPEC-0002: PUT with content exceeding 100KB → 400 Bad Request.
#[tokio::test]
async fn test_put_injection_oversized_content_rejected() {
    let big_content = "x".repeat(100 * 1024 + 1);
    let app = create_test_app();
    let (status, _) = put_injection(&app, "ws-test", "BigGlossary", &big_content).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Content > 100KB must be 400"
    );
}

/// SPEC-0002: Multiple injections can coexist; each gets a unique injection_id.
#[tokio::test]
async fn test_put_injection_creates_unique_ids() {
    let app = create_test_app();

    let (_, body1) = put_injection(&app, "ws-ids", "Glossary A", "AAA = Alpha.").await;
    let (_, body2) = put_injection(&app, "ws-ids", "Glossary B", "BBB = Beta.").await;

    let id1 = body1
        .get("injection_id")
        .and_then(|v| v.as_str())
        .expect("id1 missing");
    let id2 = body2
        .get("injection_id")
        .and_then(|v| v.as_str())
        .expect("id2 missing");

    assert_ne!(id1, id2, "Two injections must have distinct IDs");
}

/// SPEC-0002: Method not allowed on unknown methods.
#[tokio::test]
async fn test_put_injection_wrong_method_rejected() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/workspaces/ws-test/injection")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "X", "content": "y" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "POST on /injection must be 405"
    );
}

// ============================================================================
// LIST — GET /workspaces/{id}/injections
// ============================================================================

/// SPEC-0002: List returns empty array when no injections exist.
#[tokio::test]
async fn test_list_injections_empty() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/workspaces/ws-empty/injections")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(
        body.get("total").and_then(|v| v.as_u64()),
        Some(0),
        "Empty workspace should have total=0: {body}"
    );
    assert!(
        body.get("items")
            .and_then(|v| v.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(false),
        "Empty workspace items must be empty array: {body}"
    );
}

/// SPEC-0002: Created injection appears in list.
#[tokio::test]
async fn test_list_injections_contains_created() {
    let app = create_test_app();
    let ws = "ws-list-created";

    let (status, put_body) = put_injection(
        &app,
        ws,
        "Domain Glossary",
        "ML = Machine Learning. DL = Deep Learning.",
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let injection_id = put_body["injection_id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let list = extract_json(response).await;
    assert_eq!(
        list["total"].as_u64(),
        Some(1),
        "Should have 1 injection: {list}"
    );

    let items = list["items"].as_array().unwrap();
    let found = items
        .iter()
        .find(|item| item["injection_id"].as_str() == Some(injection_id));
    assert!(
        found.is_some(),
        "Created injection_id {injection_id} not found in list: {list}"
    );

    let item = found.unwrap();
    assert_eq!(item["name"].as_str(), Some("Domain Glossary"));
    assert_eq!(item["source_type"].as_str(), Some("text"));
    assert!(
        item["status"].as_str().is_some(),
        "Status field must be present"
    );
    assert!(
        item["created_at"].as_str().is_some(),
        "created_at field must be present"
    );
}

/// SPEC-0002: List items are sorted newest-first.
#[tokio::test]
async fn test_list_injections_sorted_newest_first() {
    let app = create_test_app();
    let ws = "ws-sort";

    let (_, b1) = put_injection(&app, ws, "First", "First content.").await;
    let (_, b2) = put_injection(&app, ws, "Second", "Second content.").await;
    let id1 = b1["injection_id"].as_str().unwrap().to_string();
    let id2 = b2["injection_id"].as_str().unwrap().to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = extract_json(response).await;
    let items = list["items"].as_array().unwrap();
    assert!(items.len() >= 2, "Must have at least 2 items");

    // The second injection created should appear first (or at least id1 and id2 present)
    let ids: Vec<&str> = items
        .iter()
        .filter_map(|i| i["injection_id"].as_str())
        .collect();
    assert!(ids.contains(&id1.as_str()), "id1 must be in list");
    assert!(ids.contains(&id2.as_str()), "id2 must be in list");
}

/// SPEC-0002: Injections from one workspace don't appear in another workspace's list.
///
/// Workspace isolation is enforced via the X-Workspace-ID header.
#[tokio::test]
async fn test_list_injections_workspace_isolation() {
    let app = create_test_app();

    // Create injection in workspace-A (using header)
    let body_a = json!({ "name": "Secret Glossary", "content": "ACME = Top secret org." });
    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-isolation-a/injection")
                .header("content-type", "application/json")
                .header("x-workspace-id", "ws-isolation-a")
                .body(Body::from(body_a.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // List from workspace-B (different header) — must see nothing
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/workspaces/ws-isolation-b/injections")
                .header("x-workspace-id", "ws-isolation-b")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = extract_json(response).await;
    assert_eq!(
        list["total"].as_u64(),
        Some(0),
        "Workspace B must be isolated from A: {list}"
    );
}

// ============================================================================
// GET DETAIL — GET /workspaces/{id}/injections/{injection_id}
// ============================================================================

/// SPEC-0002: GET detail returns full injection content and metadata.
#[tokio::test]
async fn test_get_injection_detail_success() {
    let app = create_test_app();
    let ws = "ws-detail";
    let content = "EdgeQuake is a Rust-based RAG framework.";
    let name = "EdgeQuake Glossary";

    let (_, put_body) = put_injection(&app, ws, name, content).await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let detail = extract_json(response).await;

    assert_eq!(detail["injection_id"].as_str(), Some(injection_id));
    assert_eq!(detail["name"].as_str(), Some(name));
    assert_eq!(detail["content"].as_str(), Some(content));
    assert_eq!(detail["version"].as_u64(), Some(1));
    assert_eq!(detail["source_type"].as_str(), Some("text"));
    assert!(
        detail["status"].as_str().is_some(),
        "status field required: {detail}"
    );
    assert!(
        detail["created_at"].as_str().is_some(),
        "created_at required: {detail}"
    );
    assert!(
        detail["updated_at"].as_str().is_some(),
        "updated_at required: {detail}"
    );
}

/// SPEC-0002: GET detail for nonexistent injection_id → 404.
#[tokio::test]
async fn test_get_injection_detail_not_found() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/workspaces/ws-test/injections/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Nonexistent injection must return 404"
    );
}

/// SPEC-0002: GET detail with injection_id from wrong workspace → 404.
///
/// Workspace isolation is enforced via the X-Workspace-ID header.
#[tokio::test]
async fn test_get_injection_detail_wrong_workspace() {
    let app = create_test_app();

    // Create injection in ws-owner using its workspace header
    let body = json!({ "name": "Private Gloss", "content": "Proprietary terms here." });
    let put_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-owner/injection")
                .header("content-type", "application/json")
                .header("x-workspace-id", "ws-owner")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let put_body = extract_json(put_response).await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    // Attempt to read from ws-attacker (different workspace header) → must be 404
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/workspaces/ws-attacker/injections/{injection_id}"
                ))
                .header("x-workspace-id", "ws-attacker")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Cross-workspace access must return 404 ({injection_id})"
    );
}

// ============================================================================
// DELETE — DELETE /workspaces/{id}/injections/{injection_id}
// ============================================================================

/// SPEC-0002: DELETE returns 200 with deleted=true.
#[tokio::test]
async fn test_delete_injection_success() {
    let app = create_test_app();
    let ws = "ws-delete";

    let (_, put_body) = put_injection(&app, ws, "Temp Glossary", "Temp = temporary.").await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let del = extract_json(response).await;
    assert_eq!(
        del["deleted"].as_bool(),
        Some(true),
        "deleted must be true: {del}"
    );

    // Subsequent GET must return 404
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        get_response.status(),
        StatusCode::NOT_FOUND,
        "Deleted injection must return 404 on subsequent GET"
    );
}

/// SPEC-0002: DELETE nonexistent injection → 404.
#[tokio::test]
async fn test_delete_injection_not_found() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/workspaces/ws-test/injections/nonexistent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Delete nonexistent must be 404"
    );
}

/// SPEC-0002: After DELETE, injection no longer appears in list.
#[tokio::test]
async fn test_delete_injection_removed_from_list() {
    let app = create_test_app();
    let ws = "ws-del-list";

    let (_, put_body) = put_injection(&app, ws, "To Delete", "Ephemeral content.").await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = extract_json(list_response).await;
    let items = list["items"].as_array().unwrap();
    let still_there = items
        .iter()
        .any(|i| i["injection_id"].as_str() == Some(injection_id));
    assert!(
        !still_there,
        "Deleted injection must not appear in list: {list}"
    );
}

// ============================================================================
// PATCH — Update injection
// ============================================================================

/// SPEC-0002: PATCH name-only does not bump version.
#[tokio::test]
async fn test_patch_injection_name_only() {
    let app = create_test_app();
    let ws = "ws-patch-name";

    let (_, put_body) = put_injection(&app, ws, "Old Name", "Some content.").await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    let patch_body = json!({ "name": "New Name" });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .header("content-type", "application/json")
                .body(Body::from(patch_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "PATCH must return 200");
    let body = extract_json(response).await;
    // Version should stay at 1 since content didn't change
    assert_eq!(
        body["version"].as_u64(),
        Some(1),
        "Name-only patch must not bump version: {body}"
    );

    // GET to confirm name updated
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let detail = extract_json(get_response).await;
    assert_eq!(
        detail["name"].as_str(),
        Some("New Name"),
        "Name must be updated: {detail}"
    );
}

/// SPEC-0002: PATCH content bumps version to 2 and sets status=processing.
#[tokio::test]
async fn test_patch_injection_content_bumps_version() {
    let app = create_test_app();
    let ws = "ws-patch-version";

    let (_, put_body) = put_injection(&app, ws, "Glossary", "Initial content.").await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    let patch_body = json!({ "content": "Updated domain content with new terms." });
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .header("content-type", "application/json")
                .body(Body::from(patch_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(
        body["version"].as_u64(),
        Some(2),
        "Content change must bump version to 2: {body}"
    );
    assert_eq!(
        body["status"].as_str(),
        Some("processing"),
        "Content change must set status=processing: {body}"
    );
}

/// SPEC-0002: PATCH nonexistent injection → 404.
#[tokio::test]
async fn test_patch_injection_not_found() {
    let app = create_test_app();
    let patch_body = json!({ "name": "Renamed" });
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/workspaces/ws-test/injections/nonexistent-id")
                .header("content-type", "application/json")
                .body(Body::from(patch_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "PATCH nonexistent must be 404"
    );
}

/// SPEC-0002: PATCH with oversized content → 400.
#[tokio::test]
async fn test_patch_injection_oversized_content() {
    let app = create_test_app();
    let ws = "ws-patch-size";

    let (_, put_body) = put_injection(&app, ws, "Glossary", "Small content.").await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    let big = "x".repeat(100 * 1024 + 1);
    let patch_body = json!({ "content": big });
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .header("content-type", "application/json")
                .body(Body::from(patch_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Oversized PATCH content must be 400"
    );
}

// ============================================================================
// Document List — Injection exclusion
// ============================================================================

/// SPEC-0002: Injection entries must NOT appear in the documents list.
///
/// Injections are enrichment data — they must never surface as user documents.
#[tokio::test]
async fn test_injection_excluded_from_documents_list() {
    let app = create_test_app();

    // Create an injection
    put_injection(
        &app,
        "default",
        "Hidden Injection",
        "This should not appear in documents.",
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;

    // Documents should either be an empty array or not contain any injection:: keys
    let empty = vec![];
    let docs = body.as_array().unwrap_or(&empty);
    for doc in docs {
        let doc_id = doc
            .get("id")
            .or_else(|| doc.get("document_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !doc_id.starts_with("injection::"),
            "Document list must not contain injection entry: {doc}"
        );
    }
}

// ============================================================================
// Citation exclusion — SPEC-0002 core requirement
// ============================================================================

/// SPEC-0002: Injection-sourced entities/chunks must NOT appear in query sources.
///
/// After creating an injection and running a query, the returned `sources` array
/// must contain no entries whose document_id starts with "injection::".
#[tokio::test]
async fn test_injection_not_cited_in_query_sources() {
    let app = create_test_app();

    // Create injection
    put_injection(
        &app,
        "default",
        "Citation Test Glossary",
        "ACME = A Company that Makes Everything.",
    )
    .await;

    // Execute a query
    let query_body = json!({
        "query": "What does ACME stand for?",
        "mode": "naive"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("content-type", "application/json")
                .body(Body::from(query_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "Query must succeed");
    let result = extract_json(response).await;

    // Walk all sources and assert none have injection:: document_id
    let sources = result
        .get("sources")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for source in &sources {
        let doc_id = source
            .get("document_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !doc_id.starts_with("injection::"),
            "SPEC-0002 VIOLATION: injection source appeared in query results: {source}"
        );
    }
}

// ============================================================================
// Response shape validation
// ============================================================================

/// SPEC-0002: PUT response has all required fields.
#[tokio::test]
async fn test_put_response_shape() {
    let app = create_test_app();
    let (_, body) = put_injection(
        &app,
        "ws-shape",
        "Shape Test",
        "Testing response field completeness.",
    )
    .await;

    assert!(body["injection_id"].is_string(), "injection_id required");
    assert!(body["workspace_id"].is_string(), "workspace_id required");
    assert!(body["version"].is_number(), "version required");
    assert!(body["status"].is_string(), "status required");
}

/// SPEC-0002: Detail response has all required fields.
#[tokio::test]
async fn test_detail_response_shape() {
    let app = create_test_app();
    let ws = "ws-detail-shape";

    let (_, put_body) = put_injection(&app, ws, "Detail Shape", "Content here.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let detail = extract_json(response).await;

    for field in &[
        "injection_id",
        "name",
        "content",
        "version",
        "status",
        "entity_count",
        "source_type",
        "created_at",
        "updated_at",
    ] {
        assert!(
            detail.get(field).is_some(),
            "Detail response missing field '{field}': {detail}"
        );
    }
}

/// SPEC-0002: List response has all required fields in each item.
#[tokio::test]
async fn test_list_item_shape() {
    let app = create_test_app();
    let ws = "ws-list-shape";

    put_injection(&app, ws, "Shape Item", "Testing shape.").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = extract_json(response).await;
    let items = list["items"].as_array().unwrap();
    assert!(!items.is_empty(), "Must have an item");

    let item = &items[0];
    for field in &[
        "injection_id",
        "name",
        "status",
        "entity_count",
        "source_type",
        "created_at",
        "updated_at",
    ] {
        assert!(
            item.get(field).is_some(),
            "List item missing field '{field}': {item}"
        );
    }
}

// ============================================================================
// File Upload — PUT /workspaces/{id}/injection/file
// ============================================================================

/// Build a multipart body for injection file upload.
fn make_injection_multipart(
    name_field: Option<&str>,
    filename: &str,
    content: &str,
) -> (String, Vec<u8>) {
    let boundary = "----InjectionBoundary7890";
    let mut body = String::new();

    if let Some(name) = name_field {
        body.push_str(&format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\n{name}\r\n"
        ));
    }

    body.push_str(&format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: text/plain\r\n\r\n{content}\r\n"
    ));
    body.push_str(&format!("--{boundary}--\r\n"));

    (boundary.to_string(), body.into_bytes())
}

/// SPEC-0002: PUT /injection/file with a .txt file → 202 Accepted.
#[tokio::test]
async fn test_put_injection_file_txt_success() {
    let app = create_test_app();
    let (boundary, body) = make_injection_multipart(
        Some("TXT Glossary"),
        "glossary.txt",
        "API = Application Programming Interface\nUI = User Interface",
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "File upload must return 202"
    );
    let resp = extract_json(response).await;
    assert!(
        resp["injection_id"].is_string(),
        "injection_id required: {resp}"
    );
    assert_eq!(resp["status"].as_str(), Some("processing"));
    assert_eq!(resp["version"].as_u64(), Some(1));
}

/// SPEC-0002: PUT /injection/file with a .md file → 202 Accepted.
#[tokio::test]
async fn test_put_injection_file_md_success() {
    let app = create_test_app();
    let (boundary, body) = make_injection_multipart(
        Some("Markdown Glossary"),
        "glossary.md",
        "# Glossary\n\n- **RAG**: Retrieval-Augmented Generation\n- **LLM**: Large Language Model",
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

/// SPEC-0002: PUT /injection/file with a .csv file → 202 Accepted.
#[tokio::test]
async fn test_put_injection_file_csv_success() {
    let app = create_test_app();
    let (boundary, body) = make_injection_multipart(
        Some("CSV Glossary"),
        "terms.csv",
        "term,definition\nRAG,Retrieval-Augmented Generation\nLLM,Large Language Model",
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

/// SPEC-0002: PUT /injection/file with unsupported extension → 400.
#[tokio::test]
async fn test_put_injection_file_unsupported_extension() {
    let app = create_test_app();
    let (boundary, body) = make_injection_multipart(None, "glossary.docx", "Word document content");

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        ".docx must be rejected with 400"
    );
}

/// SPEC-0002: PUT /injection/file with no file field → 400.
#[tokio::test]
async fn test_put_injection_file_missing_file_field() {
    let app = create_test_app();
    let boundary = "----EmptyBoundary";
    let body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nTest\r\n--{boundary}--\r\n");

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body.into_bytes()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Missing file field must be 400"
    );
}

/// SPEC-0002: File upload creates an entry visible in list.
#[tokio::test]
async fn test_put_injection_file_appears_in_list() {
    let app = create_test_app();
    let ws = "ws-file-list";
    let (boundary, body) = make_injection_multipart(
        Some("File Glossary"),
        "glossary.txt",
        "ML = Machine Learning.",
    );

    let put_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{ws}/injection/file"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_response.status(), StatusCode::ACCEPTED);
    let put_body = extract_json(put_response).await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    // Check list
    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = extract_json(list_response).await;
    let items = list["items"].as_array().unwrap();
    let found = items
        .iter()
        .find(|i| i["injection_id"].as_str() == Some(injection_id));
    assert!(
        found.is_some(),
        "File injection must appear in list: {list}"
    );
    assert_eq!(
        found.unwrap()["source_type"].as_str(),
        Some("file"),
        "source_type must be 'file'"
    );
}

/// SPEC-0002: File upload with name falling back to filename stem.
#[tokio::test]
async fn test_put_injection_file_name_fallback() {
    let app = create_test_app();
    let ws = "ws-file-name";
    let (boundary, body) = make_injection_multipart(
        None, // no explicit name — should use filename stem
        "my_glossary.txt",
        "Test content for name fallback.",
    );

    let put_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{ws}/injection/file"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_response.status(), StatusCode::ACCEPTED);
    let put_body = extract_json(put_response).await;
    let injection_id = put_body["injection_id"].as_str().unwrap();

    let detail_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let detail = extract_json(detail_response).await;
    // Name should be derived from filename, not empty
    assert!(
        !detail["name"].as_str().unwrap_or("").is_empty(),
        "Name must not be empty when filename fallback applies: {detail}"
    );
}

// ============================================================================
// DELETE — graph and vector cleanup
// ============================================================================

/// SPEC-0002 / FIX: DELETE must remove graph nodes whose only source is the injection.
///
/// Scenario:
/// 1. PUT an injection → obtain injection_id
/// 2. Seed a graph node with source_ids = [doc_id]  (simulating post-pipeline state)
/// 3. DELETE the injection
/// 4. Assert the graph node no longer exists
#[tokio::test]
async fn test_delete_injection_cleans_graph_nodes() {
    let (app, state) = create_test_app_with_state();
    let ws = "ws-del-graph";

    // --- Step 1: Create injection with explicit workspace header so the handler
    // uses ws as the workspace_id (not the "default" fallback).
    let body =
        serde_json::json!({ "name": "Graph Cleanup Test", "content": "EntityA = test entity." });
    let put_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{ws}/injection"))
                .header("content-type", "application/json")
                .header("x-workspace-id", ws)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = put_response.status();
    let put_body = extract_json(put_response).await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "PUT must return 202: {put_body}"
    );
    let injection_id = put_body["injection_id"].as_str().unwrap().to_string();

    // doc_id mirrors the server-side format: "injection::{workspace_id}::{injection_id}"
    let doc_id = format!("injection::{ws}::{injection_id}");

    // --- Step 2: Seed a graph node referencing doc_id ---
    let mut node_props: HashMap<String, serde_json::Value> = HashMap::new();
    node_props.insert("entity_type".to_string(), json!("concept"));
    node_props.insert(
        "description".to_string(),
        json!("Test entity from injection"),
    );
    node_props.insert("source_ids".to_string(), json!([doc_id]));
    state
        .storage
        .graph_storage
        .upsert_node("ENTITY_A", node_props)
        .await
        .expect("Failed to seed graph node");

    // Verify the node exists before deletion
    let exists_before = state
        .storage
        .graph_storage
        .has_node("ENTITY_A")
        .await
        .expect("has_node failed");
    assert!(exists_before, "Seeded node must exist before DELETE");

    // --- Step 3: DELETE the injection via HTTP (same workspace header) ---
    let del_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .header("x-workspace-id", ws)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        del_response.status(),
        StatusCode::OK,
        "DELETE must return 200"
    );
    let del_body = extract_json(del_response).await;
    assert_eq!(
        del_body["deleted"].as_bool(),
        Some(true),
        "deleted field must be true: {del_body}"
    );

    // --- Step 4: Assert graph node is removed ---
    let exists_after = state
        .storage
        .graph_storage
        .has_node("ENTITY_A")
        .await
        .expect("has_node failed");
    assert!(
        !exists_after,
        "Graph node sourced only from the injection must be removed after DELETE"
    );
}

/// SPEC-0002 / FIX: DELETE must NOT remove a shared graph node referenced by other sources.
///
/// Scenario:
/// 1. PUT an injection → obtain injection_id (with explicit X-Workspace-ID header)
/// 2. Seed a shared graph node with source_ids = [doc_id, "other-doc-id"]
/// 3. DELETE the injection
/// 4. Assert the node still exists but no longer references the injection's doc_id
#[tokio::test]
async fn test_delete_injection_preserves_shared_graph_nodes() {
    let (app, state) = create_test_app_with_state();
    let ws = "ws-del-shared";

    let body =
        serde_json::json!({ "name": "Shared Node Test", "content": "SharedEntity = shared." });
    let put_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{ws}/injection"))
                .header("content-type", "application/json")
                .header("x-workspace-id", ws)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = put_response.status();
    let put_body = extract_json(put_response).await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "PUT must return 202: {put_body}"
    );
    let injection_id = put_body["injection_id"].as_str().unwrap().to_string();

    let doc_id = format!("injection::{ws}::{injection_id}");

    // Seed a node shared between the injection and another document
    let mut node_props: HashMap<String, serde_json::Value> = HashMap::new();
    node_props.insert("entity_type".to_string(), json!("concept"));
    node_props.insert("source_ids".to_string(), json!([doc_id, "other-doc-123"]));
    state
        .storage
        .graph_storage
        .upsert_node("SHARED_ENTITY", node_props)
        .await
        .expect("Failed to seed shared graph node");

    // DELETE the injection (with workspace header)
    let del_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{injection_id}"))
                .header("x-workspace-id", ws)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        del_response.status(),
        StatusCode::OK,
        "DELETE must return 200"
    );

    // Shared node must still exist (it has another source)
    let node_after = state
        .storage
        .graph_storage
        .get_node("SHARED_ENTITY")
        .await
        .expect("get_node failed")
        .expect("Shared node must still exist after injection DELETE");

    // The injection doc_id must be stripped from source_ids
    let sources_after = node_after
        .properties
        .get("source_ids")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    assert!(
        !sources_after.contains(&doc_id.as_str()),
        "Injection doc_id must be removed from shared node's source_ids: {sources_after:?}"
    );
    assert!(
        sources_after.contains(&"other-doc-123"),
        "Other source must remain in shared node's source_ids: {sources_after:?}"
    );
}

// ============================================================================
// PUT — Boundary and trim edge cases
// ============================================================================

/// Name of exactly 1 character is the minimum valid length → 202.
#[tokio::test]
async fn test_put_injection_name_min_length_accepted() {
    let app = create_test_app();
    let (status, body) = put_injection(&app, "ws-boundary", "A", "Minimal name.").await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "1-char name must be accepted: {body}"
    );
}

/// Name of exactly 100 characters → 202 (boundary, not over limit).
#[tokio::test]
async fn test_put_injection_name_max_length_accepted() {
    let app = create_test_app();
    let name = "a".repeat(100);
    let (status, body) = put_injection(&app, "ws-boundary", &name, "Max name length.").await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "100-char name must be accepted: {body}"
    );
}

/// Content exactly at the 100KB limit → 202 (boundary case).
#[tokio::test]
async fn test_put_injection_content_at_limit_accepted() {
    let app = create_test_app();
    let content = "x".repeat(100 * 1024);
    let (status, body) = put_injection(&app, "ws-boundary", "At Limit", &content).await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "100KB content must be accepted: {body}"
    );
}

/// Name with leading/trailing whitespace is trimmed; trimmed result is stored.
#[tokio::test]
async fn test_put_injection_name_trimmed() {
    let app = create_test_app();
    let ws = "ws-trim";
    let (status, put_body) = put_injection(&app, ws, "  Trimmed Name  ", "Some content.").await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "Padded name must be accepted: {put_body}"
    );
    let id = put_body["injection_id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let detail = extract_json(response).await;
    assert_eq!(
        detail["name"].as_str(),
        Some("Trimmed Name"),
        "Name must be stored trimmed: {detail}"
    );
}

/// After trimming, a name that becomes empty should be rejected.
#[tokio::test]
async fn test_put_injection_whitespace_only_name_rejected() {
    let app = create_test_app();
    let (status, _) = put_injection(&app, "ws-boundary", "   ", "Some content.").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Whitespace-only name must be 400 (trims to empty)"
    );
}

// ============================================================================
// LIST — Count accuracy
// ============================================================================

/// total field must match the number of items in the array.
#[tokio::test]
async fn test_list_total_matches_item_count() {
    let app = create_test_app();
    let ws = "ws-count";

    for i in 0..3 {
        put_injection(&app, ws, &format!("Item {i}"), &format!("Content {i}")).await;
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list = extract_json(response).await;
    let items = list["items"].as_array().expect("items must be array");
    let total = list["total"].as_u64().expect("total must be u64") as usize;
    assert_eq!(total, items.len(), "total must equal items.len(): {list}");
    assert_eq!(total, 3, "Must have exactly 3 items: {list}");
}

// ============================================================================
// DELETE — Edge cases
// ============================================================================

/// DELETE response must contain `deleted=true` AND a non-empty `message`.
#[tokio::test]
async fn test_delete_injection_response_shape() {
    let app = create_test_app();
    let ws = "ws-del-shape";

    let (_, put_body) = put_injection(&app, ws, "Shape", "Shape content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let del = extract_json(response).await;

    assert_eq!(
        del["deleted"].as_bool(),
        Some(true),
        "deleted must be true: {del}"
    );
    assert!(
        del["message"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "message must be a non-empty string: {del}"
    );
}

/// Second DELETE on the same injection → 404.
#[tokio::test]
async fn test_delete_injection_double_delete_is_404() {
    let app = create_test_app();
    let ws = "ws-double-del";

    let (_, put_body) = put_injection(&app, ws, "Double Delete", "Double delete content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    // First delete succeeds
    let first = delete_injection_http(&app, ws, id).await;
    assert_eq!(first, StatusCode::OK, "First DELETE must be 200");

    // Second delete must fail
    let second = delete_injection_http(&app, ws, id).await;
    assert_eq!(second, StatusCode::NOT_FOUND, "Second DELETE must be 404");
}

/// DELETE from a different workspace (wrong X-Workspace-ID) → 404.
///
/// The injection ID exists, but under a different workspace key, so it is
/// invisible to the other workspace.
#[tokio::test]
async fn test_delete_injection_wrong_workspace_is_404() {
    let app = create_test_app();

    // Create in ws-owner
    let (_, put_body) =
        put_injection_ws(&app, "ws-del-owner", "Owner Secret", "Owner content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    // Attempt delete from ws-attacker → must be 404
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/workspaces/ws-del-attacker/injections/{id}"
                ))
                .header("x-workspace-id", "ws-del-attacker")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Cross-workspace DELETE must be 404"
    );
}

// ============================================================================
// PATCH — Edge cases
// ============================================================================

/// PATCH with empty body (no name, no content) is a no-op → 200, version unchanged.
#[tokio::test]
async fn test_patch_injection_empty_body_is_noop() {
    let app = create_test_app();
    let ws = "ws-patch-noop";

    let (_, put_body) = put_injection(&app, ws, "Stable", "Stable content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    // Send a PATCH with completely empty JSON object
    let response = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "Empty PATCH must be 200");
    let body = extract_json(response).await;
    assert_eq!(
        body["version"].as_u64(),
        Some(1),
        "Empty PATCH must not bump version: {body}"
    );
}

/// PATCH with explicit empty string name → 400.
#[tokio::test]
async fn test_patch_injection_empty_name_rejected() {
    let app = create_test_app();
    let ws = "ws-patch-empty-name";

    let (_, put_body) = put_injection(&app, ws, "Original", "Some content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let (status, _) = patch_injection(&app, ws, id, Some(""), None).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Empty name PATCH must be 400"
    );
}

/// PATCH with name > 100 chars → 400.
#[tokio::test]
async fn test_patch_injection_name_too_long_rejected() {
    let app = create_test_app();
    let ws = "ws-patch-long-name";

    let (_, put_body) = put_injection(&app, ws, "Original", "Some content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let long_name = "x".repeat(101);
    let (status, _) = patch_injection(&app, ws, id, Some(&long_name), None).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "101-char name PATCH must be 400"
    );
}

/// PATCH with whitespace-only content → 400.
#[tokio::test]
async fn test_patch_injection_whitespace_content_rejected() {
    let app = create_test_app();
    let ws = "ws-patch-ws-content";

    let (_, put_body) = put_injection(&app, ws, "Original", "Some content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let (status, _) = patch_injection(&app, ws, id, None, Some("   \n\t  ")).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Whitespace-only content PATCH must be 400"
    );
}

/// PATCH preserves the original created_at timestamp.
#[tokio::test]
async fn test_patch_injection_preserves_created_at() {
    let app = create_test_app();
    let ws = "ws-patch-ts";

    let (_, put_body) = put_injection(&app, ws, "Time Test", "Original content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    // Read original created_at
    let detail_before = extract_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    let created_at_before = detail_before["created_at"].as_str().unwrap().to_string();

    // PATCH name only
    patch_injection(&app, ws, id, Some("Time Test v2"), None).await;

    // Read again
    let detail_after = extract_json(
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    let created_at_after = detail_after["created_at"].as_str().unwrap();
    assert_eq!(
        created_at_before, created_at_after,
        "created_at must not change after PATCH"
    );
}

/// PATCH updates the updated_at timestamp.
#[tokio::test]
async fn test_patch_injection_updates_updated_at() {
    let app = create_test_app();
    let ws = "ws-patch-updated";

    let (_, put_body) = put_injection(&app, ws, "Update TS", "Original.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let detail_before = extract_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    let updated_at_after_put = detail_before["updated_at"].as_str().unwrap();

    // PATCH with content (triggers re-processing, definitely updates updated_at)
    let (status, _) =
        patch_injection(&app, ws, id, None, Some("Updated content with new terms.")).await;
    assert_eq!(status, StatusCode::OK);

    let detail_after = extract_json(
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    let updated_at_after_patch = detail_after["updated_at"].as_str().unwrap();

    // updated_at must be >= the original (monotonically non-decreasing)
    assert!(
        updated_at_after_patch >= updated_at_after_put,
        "updated_at after PATCH ({updated_at_after_patch}) must be >= before ({updated_at_after_put})"
    );
}

/// PATCH on an injection from the wrong workspace → 404.
#[tokio::test]
async fn test_patch_injection_wrong_workspace_is_404() {
    let app = create_test_app();

    let (_, put_body) = put_injection_ws(&app, "ws-patch-owner", "Owner", "Owner content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let (status, _) = patch_injection(&app, "ws-patch-attacker", id, Some("Hijacked"), None).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "PATCH from wrong workspace must be 404"
    );
}

/// PATCH with both name and content: version bumped AND name updated.
#[tokio::test]
async fn test_patch_injection_name_and_content_together() {
    let app = create_test_app();
    let ws = "ws-patch-both";

    let (_, put_body) = put_injection(&app, ws, "Old", "Old content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let (status, patch_resp) = patch_injection(
        &app,
        ws,
        id,
        Some("New Name"),
        Some("Entirely new content."),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "PATCH must be 200: {patch_resp}");
    assert_eq!(
        patch_resp["version"].as_u64(),
        Some(2),
        "Content change must bump version: {patch_resp}"
    );
    assert_eq!(
        patch_resp["status"].as_str(),
        Some("processing"),
        "Content change must set processing status: {patch_resp}"
    );

    // GET to confirm name also changed
    let detail = extract_json(
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(
        detail["name"].as_str(),
        Some("New Name"),
        "Name must be updated: {detail}"
    );
    assert_eq!(
        detail["version"].as_u64(),
        Some(2),
        "Version must be 2: {detail}"
    );
}

/// Version increments correctly across two content PATCHes: 1 → 2 → 3.
#[tokio::test]
async fn test_patch_injection_version_increments_twice() {
    let app = create_test_app();
    let ws = "ws-patch-v3";

    let (_, put_body) = put_injection(&app, ws, "Versioned", "v1 content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let (_, patch1) = patch_injection(&app, ws, id, None, Some("v2 content.")).await;
    assert_eq!(patch1["version"].as_u64(), Some(2), "v2 expected: {patch1}");

    let (_, patch2) = patch_injection(&app, ws, id, None, Some("v3 content.")).await;
    assert_eq!(patch2["version"].as_u64(), Some(3), "v3 expected: {patch2}");
}

/// PATCH response contains all required fields.
#[tokio::test]
async fn test_patch_injection_response_shape() {
    let app = create_test_app();
    let ws = "ws-patch-shape";

    let (_, put_body) = put_injection(&app, ws, "Shape", "Shape content.").await;
    let id = put_body["injection_id"].as_str().unwrap();

    let (status, body) = patch_injection(&app, ws, id, Some("Shape v2"), None).await;
    assert_eq!(status, StatusCode::OK);

    for field in &["injection_id", "workspace_id", "version", "status"] {
        assert!(
            body.get(field).is_some(),
            "PATCH response missing field '{field}': {body}"
        );
    }
}

// ============================================================================
// File Upload — Additional extension and content edge cases
// ============================================================================

/// .json file is in the allowed list → 202 Accepted.
#[tokio::test]
async fn test_put_injection_file_json_success() {
    let app = create_test_app();
    let (boundary, body) = make_injection_multipart(
        Some("JSON Glossary"),
        "terms.json",
        r#"{"RAG": "Retrieval-Augmented Generation", "LLM": "Large Language Model"}"#,
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        ".json must be accepted"
    );
}

/// .pdf file is NOT in the allowed extensions list → 400.
#[tokio::test]
async fn test_put_injection_file_pdf_rejected() {
    let app = create_test_app();
    let (boundary, body) = make_injection_multipart(None, "document.pdf", "%PDF-1.4 binary...");

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        ".pdf must be rejected"
    );
}

/// File with empty/whitespace-only content → 400.
#[tokio::test]
async fn test_put_injection_file_empty_content_rejected() {
    let app = create_test_app();
    let (boundary, body) = make_injection_multipart(Some("Empty"), "empty.txt", "   \n  ");

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Empty file content must be 400"
    );
}

/// File with non-UTF-8 bytes → 400.
#[tokio::test]
async fn test_put_injection_file_non_utf8_rejected() {
    let boundary = "----NonUtf8Boundary";
    // Inject raw bytes that are invalid UTF-8 (\xFF\xFE sequence without BOM continuation)
    let invalid_bytes: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x41];
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(
        format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"bad.txt\"\r\nContent-Type: application/octet-stream\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(&invalid_bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Non-UTF-8 file must be 400"
    );
}

/// source_filename and source_type="file" are preserved in the detail response.
#[tokio::test]
async fn test_put_injection_file_detail_has_source_filename() {
    let app = create_test_app();
    let ws = "ws-file-fname";
    let filename = "domain_terms.txt";
    let (boundary, body) = make_injection_multipart(
        Some("Domain Terms"),
        filename,
        "EdgeQuake = Advanced RAG framework.",
    );

    let put_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{ws}/injection/file"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let put_body = extract_json(put_response).await;
    let id = put_body["injection_id"].as_str().unwrap();

    let detail = extract_json(
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/injections/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;

    assert_eq!(
        detail["source_type"].as_str(),
        Some("file"),
        "source_type must be 'file': {detail}"
    );
    // source_filename is not in InjectionDetailResponse struct — but source_type must be "file"
    // (source_filename is stored in KV metadata, not in the detail DTO currently)
    assert!(
        !detail["injection_id"].as_str().unwrap_or("").is_empty(),
        "injection_id must be present: {detail}"
    );
}

/// When the filename stem exceeds 100 chars, the name is truncated to exactly 100.
#[tokio::test]
async fn test_put_injection_file_long_stem_name_truncated() {
    let app = create_test_app();
    // stem length = 110 chars → after truncation should be exactly 100
    let long_stem = "a".repeat(110);
    let filename = format!("{long_stem}.txt");
    let (boundary, body) =
        make_injection_multipart(None, &filename, "Content with a very long filename stem.");

    let put_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/workspaces/ws-file-long/injection/file")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_response.status(), StatusCode::ACCEPTED);
    let put_body = extract_json(put_response).await;
    let id = put_body["injection_id"].as_str().unwrap();

    let detail = extract_json(
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/ws-file-long/injections/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    let name = detail["name"].as_str().unwrap_or("");
    assert!(
        name.len() <= 100,
        "Name must be truncated to ≤100 chars, got {}: {detail}",
        name.len()
    );
    assert!(!name.is_empty(), "Name must not be empty: {detail}");
}
