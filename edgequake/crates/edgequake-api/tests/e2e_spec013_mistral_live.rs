//! SPEC-013 live Mistral E2E with **PostgreSQL** datastore.
//!
//! ```bash
//! export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
//! export MISTRAL_API_KEY=...
//! cargo test -p edgequake-api --features postgres --test e2e_spec013_mistral_live -- --ignored --nocapture
//! ```

#![cfg(feature = "postgres")]

mod common;
use common::spec013_postgres;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{extract_json, post_json_with_tenant};
use serde_json::json;
use serial_test::serial;
use std::time::Duration;
use tower::ServiceExt;

fn mistral_available() -> bool {
    std::env::var("MISTRAL_API_KEY")
        .map(|k| !k.is_empty())
        .unwrap_or(false)
        && spec013_postgres::database_url().is_some()
}

#[tokio::test]
#[serial]
async fn spec013_workspace_created_with_mistral_providers() {
    if !mistral_available() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }
    let app = spec013_postgres::create_postgres_mistral_app().await;
    let suffix = uuid::Uuid::new_v4();

    let tenant_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "name": format!("SPEC013 WS Mistral {suffix}") }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tenant_res.status(), StatusCode::CREATED);
    let tenant_id = extract_json(tenant_res).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let ws_res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    spec013_postgres::mistral_workspace_json(format!("Mistral WS {suffix}"))
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ws_res.status(), StatusCode::CREATED);
    spec013_postgres::assert_workspace_uses_mistral(&extract_json(ws_res).await);
}

#[tokio::test]
#[ignore = "requires MISTRAL_API_KEY + DATABASE_URL — calls Mistral APIs"]
#[serial]
async fn spec013_mistral_health_reports_mistral_provider() {
    if !mistral_available() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }
    let app = spec013_postgres::create_postgres_mistral_app().await;
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
    let provider = body["llm_provider_name"].as_str().unwrap_or("");
    assert!(
        provider.contains("mistral"),
        "expected mistral provider, got {provider}"
    );
}

#[tokio::test]
#[ignore = "requires MISTRAL_API_KEY + DATABASE_URL — live document ingest"]
#[serial]
async fn spec013_mistral_ingest_short_document() {
    if !mistral_available() {
        return;
    }
    let app = spec013_postgres::create_postgres_mistral_app().await;
    let suffix = uuid::Uuid::new_v4();

    let tenant_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "name": format!("SPEC013 Mistral {suffix}") }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tenant_res.status(), StatusCode::CREATED);
    let tenant = extract_json(tenant_res).await;
    let tenant_id = tenant["id"].as_str().unwrap();

    let ws_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    spec013_postgres::mistral_workspace_json("Mistral WS").to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ws_res.status(), StatusCode::CREATED);
    let ws = extract_json(ws_res).await;
    spec013_postgres::assert_workspace_uses_mistral(&ws);
    let workspace_id = ws["id"].as_str().unwrap();

    // Re-fetch from Postgres — workspace Mistral config must persist
    let get_ws = app
        .clone()
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
    assert_eq!(get_ws.status(), StatusCode::OK);
    spec013_postgres::assert_workspace_uses_mistral(&extract_json(get_ws).await);

    let content = format!(
        "EdgeQuake SPEC-013 Mistral Postgres test {suffix}. \
         Alice works at Acme Corp in Paris."
    );

    let (status, body) = post_json_with_tenant(
        &app,
        "/api/v1/documents",
        &json!({
            "content": content,
            "title": "spec013-mistral-live",
            "async_processing": true
        }),
        tenant_id,
        common::TEST_USER_ID,
        workspace_id,
    )
    .await;

    assert!(
        status == StatusCode::CREATED || status == StatusCode::OK,
        "upload: {status} {body:?}"
    );
    let doc_id = body["document_id"]
        .as_str()
        .or_else(|| body["id"].as_str())
        .expect("document_id");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        if tokio::time::Instant::now() > deadline {
            panic!("document did not complete within 180s");
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/documents/{doc_id}"))
                    .header("X-Tenant-ID", tenant_id)
                    .header("X-Workspace-ID", workspace_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if response.status().is_success() {
            let doc = extract_json(response).await;
            let status_str = doc["status"].as_str().unwrap_or("");
            if status_str.eq_ignore_ascii_case("completed") {
                if let Some(types) = doc.get("entity_types").and_then(|v| v.as_array()) {
                    for t in types {
                        let s = t.as_str().unwrap_or("");
                        assert!(!s.contains('/'), "entity type should be enforced, got {s}");
                    }
                }
                break;
            }
            if status_str.eq_ignore_ascii_case("failed") {
                panic!("document failed: {:?}", doc);
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
