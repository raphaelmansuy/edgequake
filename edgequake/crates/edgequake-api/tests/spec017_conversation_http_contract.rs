//! SPEC-017 conversation HTTP contract — memory API roundtrip (P3).
//!
//! Proves ConversationStorage trait wiring through REST handlers (not unit test only).

mod common;

use axum::http::StatusCode;
use common::{
    create_test_app, get_with_tenant, post_json_with_tenant, TEST_TENANT_ID, TEST_USER_ID,
    TEST_WORKSPACE_ID,
};
use serde_json::json;

/// Create conversation → add message via list endpoint verification.
#[tokio::test]
async fn spec017_conversation_http_create_and_list_contract() {
    let app = create_test_app();

    let create_payload = json!({ "title": "SPEC-017 HTTP contract" });
    let (status, body) = post_json_with_tenant(
        &app,
        "/api/v1/conversations",
        &create_payload,
        TEST_TENANT_ID,
        TEST_USER_ID,
        TEST_WORKSPACE_ID,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "create conversation: {body}");
    let conv_id = body["id"].as_str().expect("conversation id");
    assert_eq!(body["title"], "SPEC-017 HTTP contract");

    let (list_status, list_body) = get_with_tenant(
        &app,
        "/api/v1/conversations",
        TEST_TENANT_ID,
        TEST_USER_ID,
        TEST_WORKSPACE_ID,
    )
    .await;

    assert_eq!(
        list_status,
        StatusCode::OK,
        "list conversations: {list_body}"
    );
    let items = list_body["items"].as_array().expect("items array");
    assert!(
        items.iter().any(|c| c["id"].as_str() == Some(conv_id)),
        "created conversation not in list: {list_body}"
    );
}

/// Missing tenant headers → 400 (auth boundary).
#[tokio::test]
async fn spec017_conversation_http_requires_tenant_headers() {
    let app = create_test_app();
    let (status, _) = common::post_json(
        &app,
        "/api/v1/conversations",
        &json!({ "title": "no tenant" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
