//! SPEC-013: Systematic E2E proof that **PDF ingestion** + **RAG query** works
//! end-to-end on PostgreSQL using **Mistral** providers.
//!
//! Goals:
//! - Always start from a clean state (fresh tenant + fresh workspace).
//! - Upload a deterministic PDF fixture (embedded bytes).
//! - Wait for PDF processing to complete and validate extracted markdown exists.
//! - Execute a query scoped to the workspace and assert we get an answer + sources.
//!
//! Requires:
//! - `DATABASE_URL` (or POSTGRES_PASSWORD + host/port/user/db)
//! - `MISTRAL_API_KEY`
//!
//! Run:
//! `cargo test -p edgequake-api --features postgres --test e2e_spec013_mistral_pdf_query -- --nocapture`

#![cfg(feature = "postgres")]

mod common;
use common::spec013_postgres;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use common::{extract_json, post_json_with_tenant};
use serde_json::{json, Value};
use serial_test::serial;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tower::ServiceExt;
use uuid::Uuid;

const PDF_FIXTURE: &[u8] =
    include_bytes!("../../../../legacy/edgequake-pdf/test-data/001_simple_text.pdf");

fn mistral_available() -> bool {
    std::env::var("MISTRAL_API_KEY")
        .map(|k| !k.is_empty())
        .unwrap_or(false)
        && spec013_postgres::database_url().is_some()
}

fn require_or_skip_mistral() -> bool {
    let available = mistral_available();
    if available {
        return true;
    }
    let strict = std::env::var("EDGEQUAKE_REQUIRE_MISTRAL_TESTS")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if strict {
        panic!(
            "EDGEQUAKE_REQUIRE_MISTRAL_TESTS is enabled, but MISTRAL_API_KEY or DATABASE_URL is missing"
        );
    }
    false
}

fn multipart_pdf_upload_body(
    filename: &str,
    pdf_bytes: &[u8],
    fields: &[(&str, &str)],
) -> (String, Vec<u8>) {
    let boundary = "----EdgeQuakeMistralPdfBoundary013";
    let mut body: Vec<u8> = Vec::with_capacity(pdf_bytes.len() + 1024);

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

fn extract_token(markdown: &str) -> Option<String> {
    markdown
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find(|t| t.len() >= 5)
        .map(|s| s.to_string())
}

fn ingest_wait_timeout() -> Duration {
    std::env::var("SPEC013_INGEST_MAX_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(600))
}

fn assert_ingest_slo_if_set(elapsed: Duration) {
    let Ok(max_secs) = std::env::var("SPEC013_INGEST_SLO_SECS") else {
        return;
    };
    let max_secs: u64 = max_secs
        .parse()
        .expect("SPEC013_INGEST_SLO_SECS must be a positive integer");
    assert!(
        elapsed.as_secs() <= max_secs,
        "ingest exceeded SPEC013_INGEST_SLO_SECS: {}s > {}s",
        elapsed.as_secs(),
        max_secs
    );
}

fn assert_query_slo_if_set(elapsed: Duration) {
    let Ok(max_secs) = std::env::var("SPEC013_QUERY_SLO_SECS") else {
        return;
    };
    let max_secs: u64 = max_secs
        .parse()
        .expect("SPEC013_QUERY_SLO_SECS must be a positive integer");
    assert!(
        elapsed.as_secs() <= max_secs,
        "query exceeded SPEC013_QUERY_SLO_SECS: {}s > {}s",
        elapsed.as_secs(),
        max_secs
    );
}

async fn upload_pdf_multipart(
    app: &Router,
    tenant_id: &str,
    workspace_id: &str,
    filename: &str,
    pdf_bytes: &[u8],
    fields: &[(&str, &str)],
) -> (StatusCode, Value) {
    let (boundary, body) = multipart_pdf_upload_body(filename, pdf_bytes, fields);
    let response = app
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
    let status = response.status();
    (status, extract_json(response).await)
}

async fn create_second_workspace(app: &Router, tenant_id: &str, label: &str) -> String {
    let suffix = Uuid::new_v4();
    let ws_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    spec013_postgres::mistral_workspace_json(format!("{label} ws {suffix}"))
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ws_res.status(), StatusCode::CREATED);
    let ws = extract_json(ws_res).await;
    spec013_postgres::assert_workspace_uses_mistral(&ws);
    ws["id"].as_str().unwrap().to_string()
}

async fn wait_for_pdf_completed(
    app: &Router,
    tenant_id: &str,
    user_id: &str,
    workspace_id: &str,
    pdf_id: &str,
    timeout: Duration,
) -> Value {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut last: Value = Value::Null;
    let mut polls: u32 = 0;

    loop {
        if tokio::time::Instant::now() > deadline {
            panic!("pdf not query-ready within {timeout:?} (polls={polls}): last={last:?}");
        }

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/documents/pdf/{pdf_id}"))
                    .header("X-Tenant-ID", tenant_id)
                    .header("X-User-ID", user_id)
                    .header("X-Workspace-ID", workspace_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        polls += 1;
        let mut sleep_for = Duration::from_secs(2);

        if response.status().is_success() {
            let body = extract_json(response).await;
            last = body.clone();
            let status_str = body["status"].as_str().unwrap_or("");
            if status_str.eq_ignore_ascii_case("failed") {
                panic!("pdf failed: {body:?}");
            }
            // Markdown "completed" can precede entity pipeline linking document_id.
            if status_str.eq_ignore_ascii_case("completed") {
                let doc_id = body["document_id"].as_str().unwrap_or("");
                if doc_id.len() > 10 {
                    return body;
                }
                sleep_for = Duration::from_millis(400);
            } else if status_str.eq_ignore_ascii_case("processing") {
                sleep_for = Duration::from_millis(800);
            }
        }

        tokio::time::sleep(sleep_for).await;
    }
}

async fn create_fresh_tenant_and_workspace(app: &axum::Router, label: &str) -> (String, String) {
    let suffix = Uuid::new_v4();
    let tenant_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "name": format!("{label} tenant {suffix}") }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = tenant_res.status();
    let tenant_body = extract_json(tenant_res).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "tenant create failed: {tenant_body:?}"
    );
    let tenant_id = tenant_body["id"].as_str().unwrap().to_string();

    let ws_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    spec013_postgres::mistral_workspace_json(format!("{label} ws {suffix}"))
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ws_res.status(), StatusCode::CREATED);
    let ws = extract_json(ws_res).await;
    spec013_postgres::assert_workspace_uses_mistral(&ws);
    let workspace_id = ws["id"].as_str().unwrap().to_string();
    (tenant_id, workspace_id)
}

#[tokio::test]
#[serial]
async fn spec013_mistral_pdf_ingest_and_query_is_systematic() {
    if !require_or_skip_mistral() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }

    // App is configured to use Mistral providers (live).
    let app = spec013_postgres::create_postgres_mistral_app().await;
    let suffix = Uuid::new_v4();

    // 1) Fresh tenant
    let tenant_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "name": format!("SPEC013 Mistral PDF {suffix}") }).to_string(),
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

    // 2) Fresh workspace with explicit Mistral config (persisted in Postgres)
    let ws_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    spec013_postgres::mistral_workspace_json(format!("Mistral PDF WS {suffix}"))
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ws_res.status(), StatusCode::CREATED);
    let ws = extract_json(ws_res).await;
    spec013_postgres::assert_workspace_uses_mistral(&ws);
    let workspace_id = ws["id"].as_str().unwrap().to_string();

    // 3) Upload deterministic PDF fixture (disable vision for determinism)
    let track_id = format!("spec013-track-{suffix}");
    let (boundary, body) = multipart_pdf_upload_body(
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &[
            ("title", "spec013-mistral-pdf-e2e"),
            ("enable_vision", "false"),
            ("pdf_parser_backend", "text"),
            // Force reindex to avoid any unexpected dedupe corner-case.
            ("force_reindex", "true"),
            ("track_id", &track_id),
        ],
    );

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
                .header("X-Tenant-ID", &tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        upload.status().is_success(),
        "pdf upload failed: status={} body={:?}",
        upload.status(),
        extract_json(upload).await
    );
    let upload_body = extract_json(upload).await;
    let pdf_id = upload_body["pdf_id"].as_str().expect("pdf_id").to_string();

    // 4) Wait for completion
    let ingest_started = Instant::now();
    let completed = wait_for_pdf_completed(
        &app,
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
        &pdf_id,
        ingest_wait_timeout(),
    )
    .await;
    let ingest_elapsed = ingest_started.elapsed();
    eprintln!("SPEC013_INGEST_ELAPSED_SECS={}", ingest_elapsed.as_secs());
    assert_ingest_slo_if_set(ingest_elapsed);
    let completed_document_id = completed["document_id"].as_str().unwrap_or("").to_string();
    assert!(
        completed_document_id.len() > 10,
        "expected document_id on completion, got: {completed:?}"
    );

    // 5) Validate extracted markdown is present
    let content_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/documents/pdf/{pdf_id}/content"))
                .header("X-Tenant-ID", &tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(content_res.status(), StatusCode::OK);
    let content = extract_json(content_res).await;
    let md = content["markdown_content"]
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    assert!(!md.is_empty(), "expected markdown_content, got {content:?}");

    // 6) Query pipeline must return answer + sources for the workspace
    let token = extract_token(&md).unwrap_or_else(|| "document".to_string());
    let query_started = Instant::now();
    let (q_status, q_body) = post_json_with_tenant(
        &app,
        "/api/v1/query",
        &json!({
            "query": format!("What does the PDF say about '{token}'?"),
            "mode": "hybrid"
        }),
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
    )
    .await;
    let query_elapsed = query_started.elapsed();
    eprintln!("SPEC013_QUERY_ELAPSED_SECS={}", query_elapsed.as_secs());
    assert_query_slo_if_set(query_elapsed);
    assert_eq!(q_status, StatusCode::OK, "query failed: {q_body:?}");
    assert!(
        q_body["answer"].as_str().unwrap_or("").trim().len() > 5,
        "expected non-empty answer: {q_body:?}"
    );
    assert!(
        q_body["sources"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "expected at least one source reference: {q_body:?}"
    );
    let has_expected_doc_source = q_body["sources"]
        .as_array()
        .map(|sources| {
            sources.iter().any(|s| {
                s["document_id"]
                    .as_str()
                    .map(|id| id == completed_document_id)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        has_expected_doc_source,
        "expected query sources to include uploaded document_id={}, got: {q_body:?}",
        completed_document_id
    );
}

#[tokio::test]
#[serial]
async fn spec013_mistral_pdf_ingestion_edge_cases_are_mitigated() {
    if !require_or_skip_mistral() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }

    let app = spec013_postgres::create_postgres_mistral_app().await;
    let (tenant_id, workspace_id) =
        create_fresh_tenant_and_workspace(&app, "SPEC013 Edge Cases").await;

    // Edge case 1: multipart without `file` field must fail closed (400).
    let boundary_missing = "----EdgeQuakeMistralMissingFileBoundary";
    let body_missing = format!(
        "--{boundary_missing}\r\n\
         Content-Disposition: form-data; name=\"title\"\r\n\r\n\
         missing-file\r\n\
         --{boundary_missing}\r\n\
         Content-Disposition: form-data; name=\"enable_vision\"\r\n\r\n\
         false\r\n\
         --{boundary_missing}--\r\n"
    )
    .into_bytes();
    let missing_file = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/pdf")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary_missing}"),
                )
                .header("X-Tenant-ID", &tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::from(body_missing))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_file.status(), StatusCode::BAD_REQUEST);

    // Edge case 2: malformed PDF bytes must be rejected (400), not enqueued.
    let track_bad = format!("spec013-bad-{}", Uuid::new_v4());
    let (boundary_bad, body_bad) = multipart_pdf_upload_body(
        "bad.pdf",
        b"not-a-real-pdf",
        &[("track_id", &track_bad), ("enable_vision", "false")],
    );
    let bad_pdf = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/pdf")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary_bad}"),
                )
                .header("X-Tenant-ID", &tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::from(body_bad))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_pdf.status(), StatusCode::BAD_REQUEST);

    // Rejected uploads should not create progress entries.
    let bad_progress = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/documents/pdf/progress/{track_bad}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_progress.status(), StatusCode::NOT_FOUND);

    // Baseline valid upload.
    let track_ok = format!("spec013-ok-{}", Uuid::new_v4());
    let (boundary_ok, body_ok) = multipart_pdf_upload_body(
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &[
            ("title", "spec013-edge-valid"),
            ("enable_vision", "false"),
            ("pdf_parser_backend", "text"),
            ("track_id", &track_ok),
        ],
    );
    let upload_ok = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/pdf")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary_ok}"),
                )
                .header("X-Tenant-ID", &tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::from(body_ok))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(upload_ok.status().is_success());
    let upload_ok_body = extract_json(upload_ok).await;
    let pdf_id = upload_ok_body["pdf_id"]
        .as_str()
        .expect("pdf_id")
        .to_string();

    // Progress should be visible soon after accepted upload.
    let mut saw_progress = false;
    for _ in 0..6 {
        let p = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/documents/pdf/progress/{track_ok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if p.status() == StatusCode::OK {
            saw_progress = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    assert!(
        saw_progress,
        "expected progress endpoint to eventually expose track_id"
    );

    let completed = wait_for_pdf_completed(
        &app,
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
        &pdf_id,
        ingest_wait_timeout(),
    )
    .await;
    let completed_document_id = completed["document_id"].as_str().unwrap_or("").to_string();

    // Edge case 3: duplicate upload should not create duplicate work.
    let (boundary_dup, body_dup) = multipart_pdf_upload_body(
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &[
            ("title", "spec013-edge-duplicate"),
            ("enable_vision", "false"),
        ],
    );
    let dup = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/pdf")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary_dup}"),
                )
                .header("X-Tenant-ID", &tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::from(body_dup))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(dup.status().is_success());
    let dup_body = extract_json(dup).await;
    assert_eq!(
        dup_body["status"].as_str(),
        Some("duplicate"),
        "dup body: {dup_body:?}"
    );
    assert!(
        dup_body["duplicate_of"].as_str().unwrap_or("").len() > 10,
        "duplicate response should include duplicate_of: {dup_body:?}"
    );
    assert_eq!(
        dup_body["duplicate_of"].as_str(),
        Some(pdf_id.as_str()),
        "duplicate should point to original pdf_id; got {dup_body:?}"
    );

    // Edge case 4: failed ingestion attempts do not poison query pipeline.
    let (q_status, q_body) = post_json_with_tenant(
        &app,
        "/api/v1/query",
        &json!({
            "query": "Summarize the uploaded PDF content",
            "mode": "hybrid"
        }),
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
    )
    .await;
    assert_eq!(q_status, StatusCode::OK, "query failed: {q_body:?}");
    assert!(
        q_body["answer"].as_str().unwrap_or("").trim().len() > 5,
        "expected non-empty answer: {q_body:?}"
    );
    assert!(
        q_body["sources"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "expected at least one source reference: {q_body:?}"
    );
    let has_expected_doc_source = q_body["sources"]
        .as_array()
        .map(|sources| {
            sources.iter().any(|s| {
                s["document_id"]
                    .as_str()
                    .map(|id| id == completed_document_id)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        has_expected_doc_source,
        "query should reference uploaded completed document_id={}, got: {q_body:?}",
        completed_document_id
    );
}

#[tokio::test]
#[serial]
async fn spec013_mistral_concurrent_pdf_uploads_converge_to_one_document() {
    if !require_or_skip_mistral() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }

    let app = spec013_postgres::create_postgres_mistral_app().await;
    let (tenant_id, workspace_id) =
        create_fresh_tenant_and_workspace(&app, "SPEC013 Concurrent").await;

    let track_a = format!("spec013-conc-a-{}", Uuid::new_v4());
    let track_b = format!("spec013-conc-b-{}", Uuid::new_v4());
    let fields_a = [
        ("title", "spec013-concurrent-a"),
        ("enable_vision", "false"),
        ("pdf_parser_backend", "text"),
        ("track_id", track_a.as_str()),
    ];
    let fields_b = [
        ("title", "spec013-concurrent-b"),
        ("enable_vision", "false"),
        ("pdf_parser_backend", "text"),
        ("track_id", track_b.as_str()),
    ];
    let upload_a = upload_pdf_multipart(
        &app,
        &tenant_id,
        &workspace_id,
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &fields_a,
    );
    let upload_b = upload_pdf_multipart(
        &app,
        &tenant_id,
        &workspace_id,
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &fields_b,
    );
    let ((status_a, body_a), (status_b, body_b)) = tokio::join!(upload_a, upload_b);
    assert!(status_a.is_success(), "upload A failed: {body_a:?}");
    assert!(status_b.is_success(), "upload B failed: {body_b:?}");

    let pdf_id_a = body_a["pdf_id"].as_str().expect("pdf_id_a").to_string();
    let pdf_id_b = body_b["pdf_id"].as_str().expect("pdf_id_b").to_string();

    let primary_pdf_id = if body_a["status"].as_str() == Some("duplicate") {
        body_a["duplicate_of"]
            .as_str()
            .unwrap_or(&pdf_id_a)
            .to_string()
    } else if body_b["status"].as_str() == Some("duplicate") {
        body_b["duplicate_of"]
            .as_str()
            .unwrap_or(&pdf_id_b)
            .to_string()
    } else {
        pdf_id_a.clone()
    };

    let completed = wait_for_pdf_completed(
        &app,
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
        &primary_pdf_id,
        ingest_wait_timeout(),
    )
    .await;
    let document_id = completed["document_id"].as_str().unwrap().to_string();

    for other_id in [&pdf_id_a, &pdf_id_b] {
        if other_id == &primary_pdf_id {
            continue;
        }
        let other = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/documents/pdf/{other_id}"))
                    .header("X-Tenant-ID", &tenant_id)
                    .header("X-User-ID", common::TEST_USER_ID)
                    .header("X-Workspace-ID", &workspace_id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if other.status().is_success() {
            let ob = extract_json(other).await;
            if ob["status"].as_str() == Some("duplicate") {
                assert_eq!(
                    ob["duplicate_of"].as_str(),
                    Some(primary_pdf_id.as_str()),
                    "secondary upload must reference primary pdf_id: {ob:?}"
                );
            }
        }
    }

    let (q_status, q_body) = post_json_with_tenant(
        &app,
        "/api/v1/query",
        &json!({
            "query": "Summarize the uploaded PDF",
            "mode": "hybrid"
        }),
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
    )
    .await;
    assert_eq!(q_status, StatusCode::OK, "query failed: {q_body:?}");
    let source_doc_ids: HashSet<String> = q_body["sources"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|s| s["document_id"].as_str().map(str::to_string))
        .collect();
    assert!(
        source_doc_ids.contains(&document_id),
        "query must reference the completed document_id={document_id}, got sources: {q_body:?}"
    );
    assert!(
        source_doc_ids.len() <= 3,
        "concurrent duplicate ingest should not explode source cardinality: {q_body:?}"
    );
}

#[tokio::test]
#[serial]
async fn spec013_mistral_query_is_isolated_across_workspaces() {
    if !require_or_skip_mistral() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }

    let app = spec013_postgres::create_postgres_mistral_app().await;
    let (tenant_id, workspace_a) =
        create_fresh_tenant_and_workspace(&app, "SPEC013 Isolation A").await;
    let workspace_b = create_second_workspace(&app, &tenant_id, "SPEC013 Isolation B").await;

    let track = format!("spec013-iso-{}", Uuid::new_v4());
    let (status, upload_body) = upload_pdf_multipart(
        &app,
        &tenant_id,
        &workspace_a,
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &[
            ("title", "spec013-isolation"),
            ("enable_vision", "false"),
            ("pdf_parser_backend", "text"),
            ("force_reindex", "true"),
            ("track_id", &track),
        ],
    )
    .await;
    assert!(status.is_success(), "upload failed: {upload_body:?}");
    let pdf_id = upload_body["pdf_id"].as_str().expect("pdf_id").to_string();

    let completed = wait_for_pdf_completed(
        &app,
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_a,
        &pdf_id,
        ingest_wait_timeout(),
    )
    .await;
    let doc_in_a = completed["document_id"].as_str().unwrap().to_string();

    let md_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/documents/pdf/{pdf_id}/content"))
                .header("X-Tenant-ID", &tenant_id)
                .header("X-User-ID", common::TEST_USER_ID)
                .header("X-Workspace-ID", &workspace_a)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let md_json = extract_json(md_res).await;
    let md = md_json["markdown_content"].as_str().unwrap_or("");
    let token = extract_token(md).unwrap_or_else(|| "document".to_string());

    let (q_status, q_body) = post_json_with_tenant(
        &app,
        "/api/v1/query",
        &json!({
            "query": format!("What does the PDF say about '{token}'?"),
            "mode": "hybrid"
        }),
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_b,
    )
    .await;
    assert_eq!(
        q_status,
        StatusCode::OK,
        "query in workspace B failed: {q_body:?}"
    );

    let leaks_a = q_body["sources"]
        .as_array()
        .map(|sources| {
            sources.iter().any(|s| {
                s["document_id"]
                    .as_str()
                    .map(|id| id == doc_in_a)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        !leaks_a,
        "workspace B query must not return sources from workspace A document_id={doc_in_a}: {q_body:?}"
    );
}

#[tokio::test]
#[serial]
async fn spec013_mistral_query_is_isolated_across_tenants() {
    if !require_or_skip_mistral() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }

    let app = spec013_postgres::create_postgres_mistral_app().await;
    let (tenant_a, workspace_a) = create_fresh_tenant_and_workspace(&app, "SPEC013 Tenant A").await;
    let (tenant_b, workspace_b) = create_fresh_tenant_and_workspace(&app, "SPEC013 Tenant B").await;

    let track = format!("spec013-tenant-iso-{}", Uuid::new_v4());
    let (status, upload_body) = upload_pdf_multipart(
        &app,
        &tenant_a,
        &workspace_a,
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &[
            ("title", "spec013-tenant-isolation"),
            ("enable_vision", "false"),
            ("pdf_parser_backend", "text"),
            ("force_reindex", "true"),
            ("track_id", &track),
        ],
    )
    .await;
    assert!(status.is_success(), "upload failed: {upload_body:?}");
    let pdf_id = upload_body["pdf_id"].as_str().expect("pdf_id").to_string();

    let completed = wait_for_pdf_completed(
        &app,
        &tenant_a,
        common::TEST_USER_ID,
        &workspace_a,
        &pdf_id,
        ingest_wait_timeout(),
    )
    .await;
    let doc_in_a = completed["document_id"].as_str().unwrap().to_string();

    let (q_status, q_body) = post_json_with_tenant(
        &app,
        "/api/v1/query",
        &json!({
            "query": "Summarize any uploaded PDF content",
            "mode": "hybrid"
        }),
        &tenant_b,
        common::TEST_USER_ID,
        &workspace_b,
    )
    .await;
    assert_eq!(
        q_status,
        StatusCode::OK,
        "query in tenant B failed: {q_body:?}"
    );

    let leaks_a = q_body["sources"]
        .as_array()
        .map(|sources| {
            sources.iter().any(|s| {
                s["document_id"]
                    .as_str()
                    .map(|id| id == doc_in_a)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        !leaks_a,
        "tenant B must not return sources from tenant A document_id={doc_in_a}: {q_body:?}"
    );
}

#[tokio::test]
#[serial]
async fn spec013_mistral_force_reindex_during_processing_is_safe() {
    if !require_or_skip_mistral() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }

    let app = spec013_postgres::create_postgres_mistral_app().await;
    let (tenant_id, workspace_id) =
        create_fresh_tenant_and_workspace(&app, "SPEC013 Force Reindex").await;

    let track_first = format!("spec013-reindex-first-{}", Uuid::new_v4());
    let fields_first = [
        ("title", "spec013-reindex-first"),
        ("enable_vision", "false"),
        ("pdf_parser_backend", "text"),
        ("track_id", track_first.as_str()),
    ];
    let (status_first, body_first) = upload_pdf_multipart(
        &app,
        &tenant_id,
        &workspace_id,
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &fields_first,
    )
    .await;
    assert!(
        status_first.is_success(),
        "first upload failed: {body_first:?}"
    );
    let pdf_id_first = body_first["pdf_id"].as_str().expect("pdf_id").to_string();

    let track_reindex = format!("spec013-reindex-second-{}", Uuid::new_v4());
    let fields_reindex = [
        ("title", "spec013-reindex-second"),
        ("enable_vision", "false"),
        ("pdf_parser_backend", "text"),
        ("force_reindex", "true"),
        ("track_id", track_reindex.as_str()),
    ];
    let (status_second, body_second) = upload_pdf_multipart(
        &app,
        &tenant_id,
        &workspace_id,
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &fields_reindex,
    )
    .await;
    assert!(
        status_second.is_success(),
        "reindex upload failed: {body_second:?}"
    );

    let primary_pdf_id = if body_second["status"].as_str() == Some("duplicate") {
        body_second["duplicate_of"]
            .as_str()
            .unwrap_or(&pdf_id_first)
            .to_string()
    } else {
        body_second["pdf_id"]
            .as_str()
            .unwrap_or(&pdf_id_first)
            .to_string()
    };

    let completed = wait_for_pdf_completed(
        &app,
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
        &primary_pdf_id,
        ingest_wait_timeout(),
    )
    .await;
    let document_id = completed["document_id"].as_str().unwrap().to_string();

    let (q_status, q_body) = post_json_with_tenant(
        &app,
        "/api/v1/query",
        &json!({
            "query": "Summarize the PDF content",
            "mode": "hybrid"
        }),
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
    )
    .await;
    assert_eq!(q_status, StatusCode::OK, "query failed: {q_body:?}");
    let source_ids: HashSet<String> = q_body["sources"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|s| s["document_id"].as_str().map(str::to_string))
        .collect();
    assert!(
        source_ids.contains(&document_id),
        "query must reference completed document after force_reindex race: {q_body:?}"
    );
    assert!(
        source_ids.len() <= 3,
        "force_reindex race must not fan out many document sources: {q_body:?}"
    );
}

#[tokio::test]
#[serial]
async fn spec013_mistral_rejected_upload_does_not_poison_query_sources() {
    if !require_or_skip_mistral() {
        eprintln!("SKIP: MISTRAL_API_KEY or DATABASE_URL not set");
        return;
    }

    let app = spec013_postgres::create_postgres_mistral_app().await;
    let (tenant_id, workspace_id) =
        create_fresh_tenant_and_workspace(&app, "SPEC013 Reject Poison").await;

    let track_bad = format!("spec013-poison-bad-{}", Uuid::new_v4());
    let fields_bad = [
        ("title", "spec013-poison-bad"),
        ("enable_vision", "false"),
        ("track_id", track_bad.as_str()),
    ];
    let (bad_status, _) = upload_pdf_multipart(
        &app,
        &tenant_id,
        &workspace_id,
        "bad.pdf",
        b"not-a-real-pdf",
        &fields_bad,
    )
    .await;
    assert_eq!(bad_status, StatusCode::BAD_REQUEST);

    let track_ok = format!("spec013-poison-ok-{}", Uuid::new_v4());
    let fields_ok = [
        ("title", "spec013-poison-ok"),
        ("enable_vision", "false"),
        ("pdf_parser_backend", "text"),
        ("force_reindex", "true"),
        ("track_id", track_ok.as_str()),
    ];
    let (ok_status, ok_body) = upload_pdf_multipart(
        &app,
        &tenant_id,
        &workspace_id,
        "001_simple_text.pdf",
        PDF_FIXTURE,
        &fields_ok,
    )
    .await;
    assert!(ok_status.is_success(), "valid upload failed: {ok_body:?}");
    let pdf_id = ok_body["pdf_id"].as_str().expect("pdf_id").to_string();

    let completed = wait_for_pdf_completed(
        &app,
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
        &pdf_id,
        ingest_wait_timeout(),
    )
    .await;
    let document_id = completed["document_id"].as_str().unwrap().to_string();

    let (q_status, q_body) = post_json_with_tenant(
        &app,
        "/api/v1/query",
        &json!({
            "query": "Summarize the uploaded document",
            "mode": "hybrid"
        }),
        &tenant_id,
        common::TEST_USER_ID,
        &workspace_id,
    )
    .await;
    assert_eq!(q_status, StatusCode::OK, "query failed: {q_body:?}");
    for source in q_body["sources"].as_array().unwrap_or(&vec![]) {
        let sid = source["document_id"].as_str().unwrap_or("");
        assert!(
            sid.is_empty() || sid == document_id,
            "sources must only reference the completed document, not rejected uploads: {q_body:?}"
        );
    }
}
