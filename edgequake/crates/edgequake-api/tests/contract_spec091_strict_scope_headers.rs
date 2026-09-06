//! SPEC-091 IW0 (GAP-091-08/10/11): strict scope-header handling, PG-backed.
//!
//! Fail-closed contract across the three request surfaces:
//! - **documents**: a malformed `X-Workspace-ID` matches NOTHING (pre-IW0 it
//!   wildcard-matched every workspace — GAP-091-08).
//! - **tasks**: `get_task_for_context` checks workspace AND tenant
//!   unconditionally — headerless resolves to the default scope explicitly,
//!   malformed fails closed with 404 (GAP-091-10).
//! - **query**: `build_engine_request` never emits an unscoped vector query —
//!   headerless clamps to the default workspace UUID, malformed passes through
//!   raw so it can match nothing (GAP-091-11).
//!
//! Run:
//!   DATABASE_URL=postgresql://edgequake:edgequake_secret@localhost:5432/edgequake \
//!     cargo test -p edgequake-api --features postgres --test contract_spec091_strict_scope_headers
#![cfg(feature = "postgres")]

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;

async fn extract_status_and_json(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    let json = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, json)
}

async fn upload_probe_doc(app: &axum::Router, title: &str) -> (String, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        // Unique per probe title AND per run (GAP-091-29
                        // pattern): the spec013 harness shares one database
                        // across runs; workspace content-hash dedup must not
                        // match leftovers from a previous run.
                        "content": format!(
                            "strict-scope probe document content {title} {}",
                            uuid::Uuid::new_v4()
                        ),
                        "title": title,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, body) = extract_status_and_json(response).await;
    assert!(
        status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
        "probe upload failed: {status:?} {body:?}"
    );
    let doc_id = body["document_id"]
        .as_str()
        .or(body["id"].as_str())
        .expect("document_id in upload response")
        .to_string();
    let track_id = body["track_id"]
        .as_str()
        .expect("track_id in upload response")
        .to_string();
    (doc_id, track_id)
}

fn document_ids(list_json: &Value) -> Vec<String> {
    list_json["documents"]
        .as_array()
        .or(list_json.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|d| {
            d["id"]
                .as_str()
                .or(d["document_id"].as_str())
                .map(str::to_string)
        })
        .collect()
}

#[tokio::test]
#[serial]
async fn malformed_workspace_header_matches_no_documents() {
    let Some(app) = common::spec013_postgres::create_postgres_mock_app_or_skip().await else {
        return;
    };
    let (doc_id, _) = upload_probe_doc(&app, "strict-scope-malformed-list").await;

    // Sanity: explicit default scope (tenant + workspace headers) sees the
    // default-workspace document. NOTE: headerless list intentionally returns
    // an EMPTY list (`has_full_tenant_context` strict gate, tenant_guard.rs) —
    // absence is already fail-closed on this endpoint.
    let default_tenant = edgequake_api::middleware::default_tenant_uuid().to_string();
    let default_workspace = edgequake_api::middleware::default_workspace_uuid().to_string();
    let visible = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("X-Tenant-ID", &default_tenant)
                .header("X-Workspace-ID", &default_workspace)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let (status, list_json) = extract_status_and_json(visible).await;
    assert_eq!(status, StatusCode::OK, "list failed: {list_json:?}");
    assert!(
        document_ids(&list_json).contains(&doc_id),
        "explicit default scope must include the default-workspace doc; doc_id={doc_id} body={list_json:?}"
    );

    // The fix: malformed header matches NOTHING (pre-IW0 wildcard leak). A
    // valid tenant header accompanies it so the request clears the
    // `has_full_tenant_context` presence gate and reaches the scoping
    // predicates (the code path GAP-091-08 lived in).
    for malformed in ["not-a-uuid", "12345", "default;'--"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", &default_tenant)
                    .header("X-Workspace-ID", malformed)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, list_json) = extract_status_and_json(response).await;
        assert!(
            status == StatusCode::OK
                || status == StatusCode::BAD_REQUEST
                || status == StatusCode::FORBIDDEN,
            "malformed {malformed:?}: unexpected status {status:?}"
        );
        assert!(
            !document_ids(&list_json).contains(&doc_id),
            "malformed X-Workspace-ID {malformed:?} leaked the default-workspace doc (GAP-091-08)"
        );
    }
}

#[tokio::test]
#[serial]
async fn task_scope_is_unconditional_and_fail_closed() {
    let Some(app) = common::spec013_postgres::create_postgres_mock_app_or_skip().await else {
        return;
    };
    let (_, track_id) = upload_probe_doc(&app, "strict-scope-task-gate").await;

    // Headerless → default scope → the default-workspace task is visible
    // (proves the fix did not over-tighten the anonymous dev flow).
    let own = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{track_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        own.status(),
        StatusCode::OK,
        "headerless request must see the default-scope task"
    );

    // Malformed workspace header → 404 (pre-IW0: check skipped → 200 leak).
    let malformed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{track_id}"))
                .header("X-Workspace-ID", "not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        malformed.status(),
        StatusCode::NOT_FOUND,
        "malformed workspace header must 404 (fail closed, GAP-091-10)"
    );

    // Foreign concrete workspace → 404.
    let foreign = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tasks/{track_id}"))
                .header("X-Workspace-ID", uuid::Uuid::new_v4().to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);
}

#[test]
fn engine_request_is_never_unscoped() {
    use edgequake_api::middleware::default_workspace_uuid;
    use edgequake_api::services::{build_engine_request, QueryExecutionParams};
    use edgequake_query::QueryMode;

    let base = QueryExecutionParams {
        query: "q".into(),
        mode: QueryMode::Naive,
        max_results: None,
        context_only: true,
        prompt_only: false,
        enable_rerank: false,
        rerank_top_k: None,
        mix_weights: None,
        conversation_history: None,
        system_prompt: None,
        question_type: None,
        hl_keywords: None,
        ll_keywords: None,
        response_type: None,
        allowed_document_ids: None,
        data_tenant_id: None,
        workspace_id: None,
        llm_provider: None,
        llm_model: None,
        reasoning_effort: None,
            authz_principal: None,
    policy_generation: None,
    allow_fingerprint: None,
    };

    // Headerless → clamps to the default workspace UUID (GAP-091-11).
    let req = build_engine_request(&base);
    assert_eq!(
        req.workspace_id().as_deref(),
        Some(default_workspace_uuid().to_string()).as_deref()
    );

    // `default` alias resolves to the same concrete UUID.
    let req_alias = build_engine_request(&QueryExecutionParams {
        workspace_id: Some("default".into()),
        ..base.clone()
    });
    assert_eq!(
        req_alias.workspace_id().as_deref(),
        Some(default_workspace_uuid().to_string()).as_deref()
    );

    // Malformed passes through raw — matches nothing instead of silently
    // defaulting (fail closed).
    let req_bad = build_engine_request(&QueryExecutionParams {
        workspace_id: Some("not-a-uuid".into()),
        ..base
    });
    assert_eq!(req_bad.workspace_id().as_deref(), Some("not-a-uuid"));
}
