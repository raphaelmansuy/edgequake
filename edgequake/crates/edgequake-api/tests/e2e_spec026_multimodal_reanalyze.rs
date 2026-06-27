//! SPEC-026 Phase 4h — multimodal re-analyze HTTP E2E (E06).

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use edgequake_storage::kv_keys;
use serial_test::serial;
use tower::ServiceExt;

const TABLE_ANALYZE_JSON: &str =
    r#"{"name":"revenue_table","type":"Table","description":"Revenue summary from reanalyze."}"#;

async fn post_reanalyze(app: &axum::Router, doc_id: &str) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/documents/{doc_id}/reanalyze"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"process_options":"t","reindex":false}"#.to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
#[serial]
async fn reanalyze_endpoint_updates_seeded_table_content() {
    let workers = common::create_test_app_with_llm_responses(&[TABLE_ANALYZE_JSON]).await;
    let app = workers.app();

    let doc_id = uuid::Uuid::new_v4().to_string();
    workers
        .kv_storage
        .upsert(&[
            (
                kv_keys::doc_metadata(&doc_id),
                serde_json::json!({
                    "id": doc_id,
                    "status": "completed",
                    "title": "reanalyze-e2e",
                    "tenant_id": common::TEST_TENANT_ID,
                    "workspace_id": common::TEST_WORKSPACE_ID,
                }),
            ),
            (
                kv_keys::doc_content(&doc_id),
                serde_json::json!({
                    "content": r#"Report <table id="tb-1" format="html"><tr><td>Revenue</td></tr></table> end"#
                }),
            ),
        ])
        .await
        .unwrap();

    let resp = post_reanalyze(app, &doc_id).await;
    let status = resp.status();
    let body_bytes = common::spec026_multimodal::response_body_bytes(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&body_bytes)
    );
    let parsed: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(parsed["success"], 1);

    let content = workers
        .kv_storage
        .get_by_id(&kv_keys::doc_content(&doc_id))
        .await
        .unwrap()
        .unwrap();
    let text = content["content"].as_str().unwrap();
    assert!(text.contains("Revenue summary from reanalyze"));
    assert!(text.contains("[Table Name]revenue_table"));
}

#[tokio::test]
#[serial]
async fn reanalyze_returns_404_for_missing_document() {
    let workers = common::create_test_app_with_llm_responses(&[]).await;
    let resp = post_reanalyze(workers.app(), "00000000-0000-0000-0000-000000000099").await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
