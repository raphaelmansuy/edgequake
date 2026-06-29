//! SPEC-028 Query Context Service E2E tests.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::services::global_retrieval_cache;
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

fn create_test_app() -> axum::Router {
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        AppState::test_state(),
    )
    .build_router()
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("parse json")
}

async fn post_context(app: &axum::Router, body: Value) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    extract_json(response).await
}

async fn collect_sse_json_events(response: axum::response::Response) -> Vec<Value> {
    let bytes = axum::body::to_bytes(response.into_body(), 512 * 1024)
        .await
        .expect("read sse body");
    let text = String::from_utf8_lossy(&bytes);
    text.lines()
        .filter_map(|line| {
            let payload = line.strip_prefix("data: ")?;
            serde_json::from_str(payload).ok()
        })
        .collect()
}

#[tokio::test]
async fn ec_empty_query_returns_422() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"query": ""}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn ec_context_retrieve_returns_bundle() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "What is EdgeQuake?",
                        "mode": "naive",
                        "content_granularity": "agent"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("retrieval_id").is_some());
    assert!(body.get("bundle").is_some());
    assert!(body["bundle"].get("subgraph").is_some());
    assert!(body["bundle"].get("chunks").is_some());
    assert!(body.get("retrieval_quality").is_some());
    assert!(body.get("retrieval_fingerprint").is_some());
}

#[tokio::test]
async fn ec_bypass_mode_rejected() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"query": "hello", "mode": "bypass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_search_then_fetch_roundtrip() {
    let app = create_test_app();

    let search_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context/search")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"query": "machine learning basics", "mode": "naive"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_resp.status(), StatusCode::OK);
    let search_body = extract_json(search_resp).await;
    let retrieval_id = search_body["results"][0]["retrieval_id"]
        .as_str()
        .expect("retrieval_id");

    let fetch_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/query/context/{}", retrieval_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetch_resp.status(), StatusCode::OK);
    let fetch_body = extract_json(fetch_resp).await;
    assert_eq!(fetch_body["retrieval_id"], retrieval_id);
    assert_eq!(fetch_body["cached"], true);
}

#[tokio::test]
async fn ec_context_only_parity_with_legacy_query() {
    let app = create_test_app();
    let query_text = "What is retrieval augmented generation?";

    let context_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": query_text,
                        "mode": "naive",
                        "content_granularity": "citation"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(context_resp.status(), StatusCode::OK);

    let legacy_resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": query_text,
                        "mode": "naive",
                        "context_only": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(legacy_resp.status(), StatusCode::OK);

    let context_body = extract_json(context_resp).await;
    let legacy_body = extract_json(legacy_resp).await;

    assert_eq!(legacy_body["answer"].as_str().unwrap(), "");
    assert_eq!(
        context_body["bundle"]["chunks"].as_array().map(|a| a.len()),
        legacy_body["sources"]
            .as_array()
            .map(|s| s.iter().filter(|x| x["source_type"] == "chunk").count())
    );
}

#[tokio::test]
async fn ec_invalid_retrieval_id_returns_400() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/query/context/not-a-valid-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_retrieval_response_includes_agent_hints_when_requested() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "graph retrieval test",
                        "mode": "naive",
                        "include_agent_hints": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("agent_hints").is_some());
}

#[tokio::test]
async fn ec_truncation_metadata_present() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"query": "truncation fields", "mode": "naive"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("truncation").is_some());
}

#[tokio::test]
async fn ec_unknown_retrieval_id_returns_404() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/query/context/ret_00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// --- Phase 4: Agent metadata + stream v3 (SPEC-028) ---

#[tokio::test]
async fn ec_retrieval_quality_coverage_score_in_range() {
    let app = create_test_app();
    let body = post_context(
        &app,
        json!({
            "query": "coverage score validation",
            "mode": "naive"
        }),
    )
    .await;

    let quality = &body["retrieval_quality"];
    assert!(quality.get("coverage_score").is_some());
    let score = quality["coverage_score"]
        .as_f64()
        .expect("coverage_score number");
    assert!((0.0..=1.0).contains(&score));
    assert!(quality.get("empty_context").is_some());
    assert!(quality.get("is_sufficient").is_some());
}

#[tokio::test]
async fn ec_agent_hints_include_suggested_followups() {
    let app = create_test_app();
    let body = post_context(
        &app,
        json!({
            "query": "agent hints followups",
            "mode": "naive",
            "include_agent_hints": true
        }),
    )
    .await;

    let hints = body.get("agent_hints").expect("agent_hints present");
    assert!(hints.get("suggested_followups").is_some());
    assert!(hints["suggested_followups"].is_array());
    assert!(!hints["suggested_followups"].as_array().unwrap().is_empty());
    assert!(hints.get("documents_touched").is_some());
}

#[tokio::test]
async fn ec_retrieval_fingerprint_deterministic() {
    let app = create_test_app();
    let request = json!({
        "query": "fingerprint stability check",
        "mode": "naive",
        "content_granularity": "agent"
    });

    let first = post_context(&app, request.clone()).await;
    let second = post_context(&app, request).await;

    assert_eq!(
        first["retrieval_fingerprint"].as_str(),
        second["retrieval_fingerprint"].as_str()
    );
    assert!(first["retrieval_fingerprint"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
}

#[tokio::test]
async fn ec_expired_retrieval_id_returns_410() {
    let app = create_test_app();

    let search_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/context/search")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"query": "expiry gate test", "mode": "naive"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(search_resp.status(), StatusCode::OK);
    let search_body = extract_json(search_resp).await;
    let retrieval_id = search_body["results"][0]["retrieval_id"]
        .as_str()
        .expect("retrieval_id");

    global_retrieval_cache().expire_entry_for_test(retrieval_id);

    let fetch_resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/query/context/{}", retrieval_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(fetch_resp.status(), StatusCode::GONE);
}

#[tokio::test]
async fn ec_stream_v3_context_event_includes_bundle() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/stream")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "stream v3 bundle test",
                        "mode": "naive",
                        "stream_format": "v3"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let events = collect_sse_json_events(response).await;
    let context = events
        .iter()
        .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("context"))
        .expect("context SSE event");
    assert!(
        context.get("bundle").is_some(),
        "stream_format=v3 must emit ContextBundle in context event"
    );
    assert!(context["bundle"].get("subgraph").is_some());
    assert!(context["bundle"].get("chunks").is_some());
}

#[tokio::test]
async fn ec_stream_v2_context_event_omits_bundle() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query/stream")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "stream v2 no bundle",
                        "mode": "naive"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let events = collect_sse_json_events(response).await;
    let context = events
        .iter()
        .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("context"))
        .expect("context SSE event");
    assert!(
        context.get("bundle").is_none(),
        "default stream must not include bundle (v2 compat)"
    );
    assert!(
        context.get("subgraph").is_some(),
        "v2 stream must include structured subgraph (FP-028-09)"
    );
}

#[tokio::test]
async fn ec_query_response_includes_subgraph() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "What is EdgeQuake?",
                        "mode": "naive",
                        "context_only": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(
        body.get("subgraph").is_some(),
        "POST /query must expose subgraph"
    );
    assert!(body["subgraph"].get("entities").is_some());
    assert!(body["subgraph"].get("relationships").is_some());
}

#[tokio::test]
async fn ec_query_response_omits_subgraph_when_disabled() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "What is EdgeQuake?",
                        "mode": "naive",
                        "context_only": true,
                        "include_subgraph": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("subgraph").is_none());
}

// --- Phase 2: Agent artifact retrieval ---

async fn create_seeded_artifact_app() -> (axum::Router, String, String) {
    use edgequake_storage::kv_keys;

    const TENANT: &str = "00000000-0000-0000-0000-000000000001";
    const WORKSPACE: &str = "00000000-0000-0000-0000-000000000002";

    let state = AppState::test_state();
    let doc_id = "spec028-artifact-doc";
    let chunk_id = format!("{doc_id}-chunk-0");
    let kv = state.storage.kv_storage.clone();

    kv.upsert(&[(
        kv_keys::doc_metadata(doc_id),
        json!({
            "title": "Artifact Test Doc",
            "file_name": "test.md",
            "mime_type": "text/markdown",
            "status": "completed",
            "tenant_id": TENANT,
            "workspace_id": WORKSPACE
        }),
    )])
    .await
    .expect("seed metadata");

    kv.upsert(&[(
        format!("{doc_id}-content"),
        json!({"content": "# Artifact test document body with enough text for summary."}),
    )])
    .await
    .expect("seed content");

    kv.upsert(&[(
        chunk_id.clone(),
        json!({
            "content": "chunk artifact body",
            "index": 0,
            "token_count": 3,
            "start_line": 1,
            "end_line": 2
        }),
    )])
    .await
    .expect("seed chunk");

    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router();

    (app, doc_id.to_string(), chunk_id)
}

fn artifact_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header("X-Tenant-ID", "00000000-0000-0000-0000-000000000001")
        .header("X-Workspace-ID", "00000000-0000-0000-0000-000000000002")
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn ec_artifact_invalid_type_returns_400() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/query/context/artifacts/unknown/id-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_artifact_document_retrieve_metadata() {
    let (app, doc_id, _) = create_seeded_artifact_app().await;
    let response = app
        .oneshot(artifact_request(&format!(
            "/api/v1/query/context/artifacts/document/{}",
            doc_id
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["artifact_type"], "document");
    assert!(body["document"].get("chunk_count").is_some());
    assert!(body["document"]["content"].is_null());
}

#[tokio::test]
async fn ec_artifact_document_include_content() {
    let (app, doc_id, _) = create_seeded_artifact_app().await;
    let response = app
        .oneshot(artifact_request(&format!(
            "/api/v1/query/context/artifacts/document/{}?include_content=true",
            doc_id
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body["document"]["content"]
        .as_str()
        .unwrap()
        .contains("Artifact test"));
}

#[tokio::test]
async fn ec_artifact_chunk_retrieve_content() {
    let (app, _doc_id, chunk_id) = create_seeded_artifact_app().await;
    let response = app
        .oneshot(artifact_request(&format!(
            "/api/v1/query/context/artifacts/chunk/{}",
            chunk_id
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["artifact_type"], "chunk");
    assert_eq!(body["chunk"]["content"], "chunk artifact body");
}

#[tokio::test]
async fn ec_artifact_figure_requires_document_id() {
    let (app, _doc_id, _) = create_seeded_artifact_app().await;
    let response = app
        .oneshot(artifact_request(
            "/api/v1/query/context/artifacts/figure/fig-1",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_artifact_markdown_retrieve_from_kv() {
    let (app, doc_id, _) = create_seeded_artifact_app().await;
    let response = app
        .oneshot(artifact_request(&format!(
            "/api/v1/query/context/artifacts/markdown/{}",
            doc_id
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["artifact_type"], "markdown");
    assert_eq!(body["markdown"]["source"], "kv");
    assert!(body["markdown"]["markdown"]
        .as_str()
        .unwrap()
        .contains("Artifact test"));
}

#[tokio::test]
async fn ec_artifact_pdf_retrieve_with_markdown() {
    use edgequake_storage::{
        CreatePdfRequest, ExtractionMethod, PdfProcessingStatus, UpdatePdfProcessingRequest,
    };
    use uuid::Uuid;

    const TENANT: &str = "00000000-0000-0000-0000-000000000001";
    const WORKSPACE: &str = "00000000-0000-0000-0000-000000000002";

    let state = AppState::test_state();
    let doc_id = "spec028-pdf-artifact-doc";
    let pdf_uuid = Uuid::new_v4();
    let ws_uuid = Uuid::parse_str(WORKSPACE).unwrap();
    let kv = state.storage.kv_storage.clone();

    kv.upsert(&[(
        edgequake_storage::kv_keys::doc_metadata(doc_id),
        json!({
            "title": "PDF Artifact Doc",
            "source_type": "pdf",
            "pdf_id": pdf_uuid.to_string(),
            "tenant_id": TENANT,
            "workspace_id": WORKSPACE,
            "status": "completed"
        }),
    )])
    .await
    .expect("seed pdf doc metadata");

    let pdf_storage = state.storage.pdf_storage.as_ref().expect("pdf storage");
    let pdf_data = b"%PDF-1.4 spec028 test pdf bytes".to_vec();
    let checksum = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(&pdf_data))
    };
    let created_id = pdf_storage
        .create_pdf(CreatePdfRequest {
            workspace_id: ws_uuid,
            filename: "test.pdf".into(),
            content_type: "application/pdf".into(),
            file_size_bytes: pdf_data.len() as i64,
            sha256_checksum: checksum,
            page_count: Some(1),
            pdf_data,
            vision_model: None,
        })
        .await
        .expect("create pdf");

    // Use deterministic pdf_id from metadata when create returns new id — update metadata
    kv.upsert(&[(
        edgequake_storage::kv_keys::doc_metadata(doc_id),
        json!({
            "title": "PDF Artifact Doc",
            "source_type": "pdf",
            "pdf_id": created_id.to_string(),
            "tenant_id": TENANT,
            "workspace_id": WORKSPACE,
            "status": "completed"
        }),
    )])
    .await
    .expect("reseed metadata");

    pdf_storage
        .update_pdf_processing(UpdatePdfProcessingRequest {
            pdf_id: created_id,
            processing_status: PdfProcessingStatus::Completed,
            extraction_method: Some(ExtractionMethod::Vision),
            markdown_content: Some("# PDF extracted markdown".into()),
            extraction_errors: None,
            document_id: None,
            vision_model: None,
        })
        .await
        .expect("update pdf processing");

    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router();

    let response = app
        .clone()
        .oneshot(artifact_request(&format!(
            "/api/v1/query/context/artifacts/pdf/{}?include_content=true",
            created_id
        )))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(body["artifact_type"], "pdf");
    assert!(body["pdf"]["download_path"]
        .as_str()
        .unwrap()
        .contains("/documents/pdf/"));
    assert!(body["pdf"]["markdown_content"]
        .as_str()
        .unwrap()
        .contains("PDF extracted"));

    let md_resp = app
        .oneshot(artifact_request(&format!(
            "/api/v1/query/context/artifacts/markdown/{}",
            doc_id
        )))
        .await
        .unwrap();
    assert_eq!(md_resp.status(), StatusCode::OK);
    let md_body = extract_json(md_resp).await;
    assert_eq!(md_body["markdown"]["source"], "pdf_storage");
}
