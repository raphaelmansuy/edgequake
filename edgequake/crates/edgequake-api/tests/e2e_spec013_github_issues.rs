//! SPEC-013 intensive E2E — GitHub issues #216–#233 (PostgreSQL datastore).
//!
//! ```bash
//! export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
//! cargo test -p edgequake-api --features postgres --test e2e_spec013_github_issues
//! ```

#![cfg(feature = "postgres")]

mod common;
mod spec013_postgres;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{extract_json, post_json, post_json_with_tenant};
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;

#[tokio::test]
#[serial]
async fn spec013_issue232_list_api_keys_after_create() {
    let app = spec013_postgres::create_postgres_mock_app().await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/api-keys")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "name": "spec013-list-test", "scopes": ["read"] }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let created = extract_json(create).await;
    let key_id = created["key_id"].as_str().expect("key_id");

    let list = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/api-keys")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let body = extract_json(list).await;
    assert!(body["total"].as_u64().unwrap_or(0) >= 1);
    let keys = body["keys"].as_array().expect("keys array");
    assert!(
        keys.iter().any(|k| k["key_id"].as_str() == Some(key_id)),
        "created key must appear in list (Postgres KV)"
    );
}

#[tokio::test]
#[serial]
async fn spec013_issue216_update_workspace_entity_types() {
    let app = spec013_postgres::create_postgres_mock_app().await;
    let suffix = uuid::Uuid::new_v4();

    let (_, tenant_body) = post_json(
        &app,
        "/api/v1/tenants",
        &json!({ "name": format!("SPEC013 {suffix}") }),
    )
    .await;
    let tenant_id = tenant_body["id"].as_str().unwrap();

    let (_, ws_body) = post_json(
        &app,
        &format!("/api/v1/tenants/{tenant_id}/workspaces"),
        &json!({
            "name": "Entity Types WS",
            "entity_types": ["PERSON", "ORGANIZATION"]
        }),
    )
    .await;
    let workspace_id = ws_body["id"].as_str().unwrap();

    let update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}"))
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", tenant_id)
                .body(Body::from(
                    json!({
                        "entity_types": ["PERSON", "PRODUCT", "CONCEPT"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
    let updated = extract_json(update).await;
    let types: Vec<&str> = updated["entity_types"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(types.contains(&"PERSON"));
    assert!(types.contains(&"PRODUCT"));
    assert!(!types.contains(&"ORGANIZATION"));

    // Re-fetch from Postgres to prove persistence
    let get = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{workspace_id}"))
                .header("X-Tenant-ID", tenant_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let persisted = extract_json(get).await;
    let persisted_types: Vec<&str> = persisted["entity_types"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(persisted_types.contains(&"PRODUCT"));
}

#[tokio::test]
#[serial]
async fn spec013_issue231_document_upload_workspace_header() {
    let app = spec013_postgres::create_postgres_mock_app().await;
    let suffix = uuid::Uuid::new_v4();

    let (_, tenant_body) = post_json(
        &app,
        "/api/v1/tenants",
        &json!({ "name": format!("SPEC013 upload {suffix}") }),
    )
    .await;
    let tenant_id = tenant_body["id"].as_str().unwrap();

    let (_, ws_body) = post_json(
        &app,
        &format!("/api/v1/tenants/{tenant_id}/workspaces"),
        &json!({ "name": "Upload WS" }),
    )
    .await;
    let workspace_id = ws_body["id"].as_str().unwrap();

    let content = format!("SPEC-013 workspace isolation {suffix}");
    let (status, body) = post_json_with_tenant(
        &app,
        "/api/v1/documents",
        &json!({
            "content": content,
            "title": "spec013-doc",
            "async_processing": true
        }),
        tenant_id,
        common::TEST_USER_ID,
        workspace_id,
    )
    .await;
    assert!(
        status == StatusCode::CREATED || status == StatusCode::OK,
        "upload failed: {status} {:?}",
        body
    );
    assert!(body.get("document_id").is_some() || body.get("id").is_some());
}

#[tokio::test]
#[serial]
async fn spec013_issue231_models_endpoint_reports_defaults() {
    let app = spec013_postgres::create_postgres_mock_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/models")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("default_llm_model").is_some());
    assert!(body.get("default_embedding_model").is_some());
}

#[tokio::test]
#[serial]
async fn spec013_issue233_workspace_create_without_models_uses_server_defaults() {
    let app = spec013_postgres::create_postgres_mock_app().await;
    let suffix = uuid::Uuid::new_v4();

    let (_, tenant_body) = post_json(
        &app,
        "/api/v1/tenants",
        &json!({ "name": format!("SPEC013 defaults {suffix}") }),
    )
    .await;
    let tenant_id = tenant_body["id"].as_str().unwrap();

    // #233 regression guard: create workspace without explicit model picks.
    let (status, ws) = post_json(
        &app,
        &format!("/api/v1/tenants/{tenant_id}/workspaces"),
        &json!({ "name": "Defaults WS" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    assert!(ws["llm_model"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(ws["llm_provider"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(ws["embedding_model"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert!(ws["embedding_provider"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
}

#[tokio::test]
#[serial]
async fn spec013_health_storage_mode_is_postgresql() {
    let app = spec013_postgres::create_postgres_mock_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["storage_mode"].as_str(), Some("postgresql"));
}

const PDF_FIXTURE: &[u8] =
    include_bytes!("../../../../legacy/edgequake-pdf/test-data/001_simple_text.pdf");

fn multipart_pdf_body(
    filename: &str,
    pdf_bytes: &[u8],
    fields: &[(&str, &str)],
) -> (String, Vec<u8>) {
    let boundary = "----EdgeQuakeSpec013PdfBoundary";
    let mut body: Vec<u8> = Vec::with_capacity(pdf_bytes.len() + 512);
    for (k, v) in fields {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n").as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
             Content-Type: application/pdf\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(pdf_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (boundary.to_string(), body)
}

/// Cancel while PDF is still pending/processing (in-process tests have no task worker).
#[tokio::test]
#[serial]
async fn spec013_pdf_cancel_while_processing_is_accepted() {
    let app = spec013_postgres::create_postgres_mock_app().await;
    let suffix = uuid::Uuid::new_v4();

    let (_, tenant_body) = post_json(
        &app,
        "/api/v1/tenants",
        &json!({ "name": format!("SPEC013 cancel {suffix}") }),
    )
    .await;
    let tenant_id = tenant_body["id"].as_str().unwrap();

    let (_, ws_body) = post_json(
        &app,
        &format!("/api/v1/tenants/{tenant_id}/workspaces"),
        &json!({ "name": "Cancel WS" }),
    )
    .await;
    let workspace_id = ws_body["id"].as_str().unwrap();

    let fields = [
        ("title", "spec013-cancel"),
        ("enable_vision", "false"),
        ("pdf_parser_backend", "text"),
    ];
    let (boundary, body) = multipart_pdf_body("001_simple_text.pdf", PDF_FIXTURE, &fields);
    let upload = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/pdf")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("X-Tenant-ID", tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", workspace_id)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        upload.status().is_success(),
        "pdf upload failed: {:?}",
        extract_json(upload).await
    );
    let pdf_id = extract_json(upload).await["pdf_id"]
        .as_str()
        .unwrap()
        .to_string();

    let cancel = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/documents/pdf/{pdf_id}/cancel"))
                .header("X-Tenant-ID", tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", workspace_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        cancel.status().is_success(),
        "cancel on pending/processing PDF must succeed (no worker in test harness): {:?}",
        extract_json(cancel).await
    );
}

/// Requires live API with background workers (`SPEC013_LIVE_API_URL`, e.g. http://localhost:8090).
#[tokio::test]
#[serial]
async fn spec013_pdf_cancel_on_completed_returns_conflict_live() {
    let Some(base) = std::env::var("SPEC013_LIVE_API_URL")
        .ok()
        .filter(|s| !s.is_empty())
    else {
        eprintln!("SKIP: SPEC013_LIVE_API_URL not set");
        return;
    };

    let client = reqwest::Client::new();
    let tenant: serde_json::Value = client
        .post(format!("{base}/api/v1/tenants"))
        .json(&json!({ "name": format!("SPEC013 live cancel {}", uuid::Uuid::new_v4()) }))
        .send()
        .await
        .expect("tenant create")
        .json()
        .await
        .expect("tenant json");
    let tenant_id = tenant["id"].as_str().expect("tenant id");

    let ws: serde_json::Value = client
        .post(format!("{base}/api/v1/tenants/{tenant_id}/workspaces"))
        .json(&json!({ "name": "Live Cancel WS" }))
        .send()
        .await
        .expect("ws create")
        .json()
        .await
        .expect("ws json");
    let workspace_id = ws["id"].as_str().expect("workspace id");

    let (boundary, body) = multipart_pdf_body(
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &[
            ("title", "spec013-live-cancel"),
            ("enable_vision", "false"),
            ("pdf_parser_backend", "text"),
        ],
    );

    let upload = client
        .post(format!("{base}/api/v1/documents/pdf"))
        .header("X-Tenant-ID", tenant_id)
        .header("X-User-ID", common::TEST_USER_ID)
        .header("X-Workspace-ID", workspace_id)
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send()
        .await
        .expect("upload");
    assert!(
        upload.status().is_success(),
        "upload status {}",
        upload.status()
    );
    let upload_body: serde_json::Value = upload.json().await.expect("upload json");
    let pdf_id = upload_body["pdf_id"].as_str().expect("pdf_id");

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(300);
    loop {
        if tokio::time::Instant::now() > deadline {
            panic!("pdf did not complete on live API within 300s");
        }
        let st: serde_json::Value = client
            .get(format!("{base}/api/v1/documents/pdf/{pdf_id}"))
            .header("X-Tenant-ID", tenant_id)
            .header("X-User-ID", common::TEST_USER_ID)
            .header("X-Workspace-ID", workspace_id)
            .send()
            .await
            .expect("status get")
            .json()
            .await
            .expect("status json");
        if st["status"].as_str() == Some("completed") {
            break;
        }
        if st["status"].as_str() == Some("failed") {
            panic!("pdf failed on live API: {st:?}");
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    let cancel = client
        .delete(format!("{base}/api/v1/documents/pdf/{pdf_id}/cancel"))
        .header("X-Tenant-ID", tenant_id)
        .header("X-User-ID", common::TEST_USER_ID)
        .header("X-Workspace-ID", workspace_id)
        .send()
        .await
        .expect("cancel");
    assert_eq!(
        cancel.status(),
        reqwest::StatusCode::CONFLICT,
        "cancel on completed PDF must be 409: {}",
        cancel.text().await.unwrap_or_default()
    );
}
