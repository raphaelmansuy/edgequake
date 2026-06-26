//! SPEC-014: Batch upload API for documents and PDFs (issue #236).

#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::extract_json;
use common::spec013_postgres;
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;

const PDF_FIXTURE: &[u8] =
    include_bytes!("../../../../legacy/edgequake-pdf/test-data/001_simple_text.pdf");

fn multipart_text_files(parts: &[(&str, &str)], field_name: &str) -> (String, Vec<u8>) {
    let boundary = format!("batch-{}", uuid::Uuid::new_v4().simple());
    let mut body = Vec::new();
    for (filename, content) in parts {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                field_name, filename
            )
            .as_bytes(),
        );
        body.extend_from_slice(b"Content-Type: text/plain\r\n\r\n");
        body.extend_from_slice(content.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    (boundary, body)
}

fn multipart_pdf_files(filenames: &[&str], field_name: &str) -> (String, Vec<u8>) {
    let boundary = format!("pdf-batch-{}", uuid::Uuid::new_v4().simple());
    let mut body = Vec::new();
    for filename in filenames {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                field_name, filename
            )
            .as_bytes(),
        );
        body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
        body.extend_from_slice(PDF_FIXTURE);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());
    (boundary, body)
}

#[tokio::test]
#[serial]
async fn spec014_batch_text_upload_accepts_multiple_files() {
    let Some(app) = spec013_postgres::create_postgres_mock_app_or_skip().await else {
        eprintln!("SKIP: no PostgreSQL DATABASE_URL configured");
        return;
    };
    let suffix = uuid::Uuid::new_v4();

    let tenant = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "name": format!("spec014 tenant {suffix}") }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tenant_json = extract_json(tenant).await;
    let tenant_id = tenant_json["id"].as_str().unwrap();

    let ws = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", tenant_id)
                .body(Body::from(json!({ "name": "spec014 ws" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let ws_json = extract_json(ws).await;
    let workspace_id = ws_json["id"].as_str().unwrap();

    let (boundary, body) =
        multipart_text_files(&[("a.txt", "alpha"), ("b.md", "bravo markdown")], "files");
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/upload/batch")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("X-Tenant-ID", tenant_id)
                .header("X-Workspace-ID", workspace_id)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = extract_json(response).await;
    assert_eq!(body["total_files"], 2);
    assert_eq!(body["failed"], 0);
    assert_eq!(body["results"].as_array().unwrap().len(), 2);
}

#[tokio::test]
#[serial]
async fn spec014_batch_pdf_upload_accepts_multiple_files() {
    let Some(app) = spec013_postgres::create_postgres_mock_app_or_skip().await else {
        eprintln!("SKIP: no PostgreSQL DATABASE_URL configured");
        return;
    };
    let suffix = uuid::Uuid::new_v4();

    let tenant = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "name": format!("spec014 pdf tenant {suffix}") }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tenant_json = extract_json(tenant).await;
    let tenant_id = tenant_json["id"].as_str().unwrap();

    let ws = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", tenant_id)
                .body(Body::from(json!({ "name": "spec014 pdf ws" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let ws_json = extract_json(ws).await;
    let workspace_id = ws_json["id"].as_str().unwrap();

    // Same PDF twice -> first accepted, second duplicate.
    let (boundary, body) = multipart_pdf_files(&["a.pdf", "b.pdf"], "files");
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents/pdf/batch")
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .header("X-Tenant-ID", tenant_id)
                .header("X-Workspace-ID", workspace_id)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["total_files"], 2);
    assert_eq!(body["failed"], 0);
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert!(results
        .iter()
        .any(|r| r["status"] == "processing" || r["status"] == "duplicate"));
}
