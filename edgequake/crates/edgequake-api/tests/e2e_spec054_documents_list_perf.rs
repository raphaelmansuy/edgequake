//! SPEC-054 L1-a — Documents list latency with batched AGE entity reconcile.
//!
//! ```bash
//! export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake
//! cargo test -p edgequake-api --features postgres --test e2e_spec054_documents_list_perf -- --nocapture
//! ```
//!
//! Budget: warm in-process `GET /api/v1/documents` &lt; 500ms.
//! Tight AGE reconcile gate (&lt;200ms batched prefixes) lives in
//! `e2e_spec054_age_pgvector_perf` (specs/054-fix-bugs-17/003 L1-a).

#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::extract_json;
use common::spec013_postgres;
use serde_json::json;
use serial_test::serial;
use std::time::{Duration, Instant};
use tower::ServiceExt;
use uuid::Uuid;

async fn create_tenant_workspace(app: &axum::Router) -> (String, String) {
    let suffix = Uuid::new_v4();
    let tenant_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": format!("l1a-tenant-{suffix}") }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tenant_resp.status(), StatusCode::CREATED);
    let tenant = extract_json(tenant_resp).await;
    let tenant_id = tenant["id"].as_str().expect("tenant id").to_string();

    let ws_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("content-type", "application/json")
                .header("X-Tenant-ID", &tenant_id)
                .body(Body::from(
                    json!({
                        "name": format!("l1a-ws-{suffix}"),
                        "llm_provider": "mock",
                        "embedding_provider": "mock",
                        "embedding_dimension": 384,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        ws_resp.status().is_success(),
        "workspace create status={}",
        ws_resp.status()
    );
    let ws = extract_json(ws_resp).await;
    let workspace_id = ws["id"].as_str().expect("workspace id").to_string();
    (tenant_id, workspace_id)
}

async fn list_documents(
    app: &axum::Router,
    tenant_id: &str,
    workspace_id: &str,
) -> (StatusCode, serde_json::Value, Duration) {
    let start = Instant::now();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("X-Tenant-ID", tenant_id)
                .header("X-Workspace-ID", workspace_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let elapsed = start.elapsed();
    let status = resp.status();
    let body = extract_json(resp).await;
    (status, body, elapsed)
}

#[tokio::test]
#[serial]
async fn e2e_l1a_documents_list_under_500ms_warm() {
    let Some(app) = spec013_postgres::create_postgres_mock_app_or_skip().await else {
        eprintln!("SKIP: no PostgreSQL DATABASE_URL configured");
        return;
    };
    let (tenant_id, workspace_id) = create_tenant_workspace(&app).await;

    for i in 0..8 {
        let text_body = json!({
            "title": format!("l1a-doc-{i}.md"),
            "content": format!("L1-a list fixture document {i}"),
        });
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("content-type", "application/json")
                    .header("X-Tenant-ID", &tenant_id)
                    .header("X-Workspace-ID", &workspace_id)
                    .body(Body::from(text_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "seed document {i} status={}",
            resp.status()
        );
    }

    // Warm — in-process AppState pays first-touch pool/plan costs.
    for _ in 0..3 {
        let (status, _, _) = list_documents(&app, &tenant_id, &workspace_id).await;
        assert!(status.is_success(), "warm list status={status}");
    }

    let mut samples = Vec::with_capacity(5);
    for _ in 0..5 {
        let (status, body, elapsed) = list_documents(&app, &tenant_id, &workspace_id).await;
        assert!(status.is_success(), "list status={status} body={body}");
        samples.push(elapsed);
    }
    samples.sort();
    let worst = samples[samples.len() - 1];
    // In-process harness budget (catches Seq Scan cliffs of seconds).
    // Production curl budget remains ~200ms in specs/054-fix-bugs-17/003;
    // AGE batch reconcile is gated separately at <200ms in storage e2e.
    assert!(
        worst < Duration::from_millis(500),
        "L1-a FAIL: documents list worst {worst:?} exceeds 500ms in-process budget (samples={samples:?})"
    );
    eprintln!("OK L1-a: GET /api/v1/documents warm samples={samples:?} max={worst:?}");
}
