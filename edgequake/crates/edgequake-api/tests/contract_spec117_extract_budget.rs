//! SPEC-117 — Workspace extract budget API contract.
//!
//! Run:
//!   cargo test -p edgequake-api --test contract_spec117_extract_budget

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{create_test_app, extract_json};
use edgequake_api::openapi::ApiDoc;
use serde_json::json;
use tower::ServiceExt;
use utoipa::OpenApi;
use uuid::Uuid;

async fn create_tenant(app: &axum::Router) -> String {
    let slug = format!("spec117-{}", Uuid::new_v4());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": format!("SPEC-117 {slug}"),
                        "slug": slug,
                        "plan": "pro"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    extract_json(response).await["id"]
        .as_str()
        .expect("tenant id")
        .to_string()
}

#[tokio::test]
async fn spec117_api_create_update_get_extract_budget() {
    let app = create_test_app();
    let tenant_id = create_tenant(&app).await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Budget WS",
                        "slug": format!("budget-{}", Uuid::new_v4()),
                        "extract_budget_mode": "custom",
                        "extract_max_entities": 40,
                        "extract_max_records": 100
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let created = extract_json(create).await;
    assert_eq!(created["extract_budget_mode"].as_str(), Some("custom"));
    assert_eq!(created["extract_max_entities"].as_u64(), Some(40));
    assert_eq!(created["extract_max_records"].as_u64(), Some(100));
    let workspace_id = created["id"].as_str().unwrap().to_string();

    let update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}"))
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", &tenant_id)
                .body(Body::from(
                    json!({
                        "extract_budget_mode": "custom",
                        "extract_max_entities": 20,
                        "extract_max_records": 50
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
    let updated = extract_json(update).await;
    assert_eq!(updated["extract_max_entities"].as_u64(), Some(20));
    assert_eq!(updated["extract_max_records"].as_u64(), Some(50));

    let clear = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}"))
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", &tenant_id)
                .body(Body::from(
                    json!({ "extract_budget_mode": "inherit" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(clear.status(), StatusCode::OK);
    let cleared = extract_json(clear).await;
    assert!(
        cleared["extract_max_entities"].is_null() || cleared.get("extract_max_entities").is_none(),
        "inherit clears keys: {cleared}"
    );
}

#[tokio::test]
async fn spec117_api_rejects_entities_gt_records() {
    let app = create_test_app();
    let tenant_id = create_tenant(&app).await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Bad Budget",
                        "slug": format!("bad-budget-{}", Uuid::new_v4()),
                        "extract_max_entities": 50,
                        "extract_max_records": 40
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn spec117_openapi_mentions_extract_budget_fields() {
    let doc = ApiDoc::openapi();
    let json = serde_json::to_value(&doc).expect("openapi json");
    let text = json.to_string();
    assert!(text.contains("extract_max_entities"), "{text}");
    assert!(text.contains("extract_max_records"), "{text}");
    assert!(text.contains("extract_budget_mode"), "{text}");
}

#[tokio::test]
async fn spec117_admission_stores_doc_caps_and_resolve_beats_workspace() {
    use edgequake_api::middleware::TenantContext;
    use edgequake_api::services::{
        admit_document_for_processing, ContentHasher, DocumentAdmissionInput,
        DocumentAdmissionOutcome, GleaningAdmissionOptions,
    };
    use edgequake_api::AppState;

    let state = AppState::new_memory(None::<String>);
    state.workspace_service.seed_default_workspace().await;
    // In-memory seed uses Uuid::from_u128(2/3) — not the common::TEST_* constants.
    let tenant_id = uuid::Uuid::from_u128(2).to_string();
    let workspace_id = uuid::Uuid::from_u128(3).to_string();

    let content = "SPEC-117 admission: Sarah Chen leads EdgeQuake.";
    let hash = ContentHasher::hash_str(content);
    let outcome = admit_document_for_processing(
        &state,
        &TenantContext {
            tenant_id: Some(tenant_id),
            workspace_id: Some(workspace_id),
            user_id: Some(common::TEST_USER_ID.to_string()),
        },
        DocumentAdmissionInput {
            text_content: content.to_string(),
            title: "spec117-caps.md".into(),
            source_type: "markdown",
            mime_type: Some("text/markdown".into()),
            raw_byte_size: content.len(),
            content_hash: hash,
            custom_metadata: None,
            security: Default::default(),
            track_id: None,
            expected_batch_count: None,
            gleaning: GleaningAdmissionOptions::default(),
            document_type: Some("markdown"),
            chunk_strategy: None,
            chunk_options: None,
            extract_max_entities: Some(20),
            extract_max_records: Some(50),
            multimodal: false,
            ingest_mode: None,
            multimodal_manifest: None,
        },
        "spec117",
    )
    .await
    .expect("admit");

    let track_id = match outcome {
        DocumentAdmissionOutcome::Accepted(a) => a.track_id,
        other => panic!("expected accepted, got {other:?}"),
    };

    let task = state
        .tasks
        .storage
        .get_task(&track_id)
        .await
        .expect("storage")
        .expect("task row");
    let meta = task
        .task_data
        .get("metadata")
        .expect("TextInsertData.metadata");
    assert_eq!(
        meta.get("extract_max_entities").and_then(|v| v.as_u64()),
        Some(20)
    );
    assert_eq!(
        meta.get("extract_max_records").and_then(|v| v.as_u64()),
        Some(50)
    );

    // prepare.rs soft-reads via from_value; factory resolve uses resolve_for_ingestion.
    let doc = edgequake_pipeline::ExtractionCaps::from_value(meta).expect("doc caps");
    let mut ws_meta = std::collections::HashMap::new();
    ws_meta.insert("extract_max_entities".into(), json!(60));
    ws_meta.insert("extract_max_records".into(), json!(150));
    let resolved = edgequake_pipeline::ExtractionCaps::resolve_for_ingestion(&ws_meta, Some(doc));
    assert_eq!(resolved.max_entities, 20);
    assert_eq!(resolved.max_total_records, 50);
}

#[tokio::test]
async fn spec117_text_upload_rejects_partial_extract_caps() {
    let app = create_test_app();
    let tenant_id = create_tenant(&app).await;
    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "WS",
                        "slug": format!("ws-partial-{}", Uuid::new_v4())
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let workspace_id = extract_json(create).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let upload = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", &tenant_id)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::from(
                    json!({
                        "title": "bad.md",
                        "content": "x",
                        "extract_max_entities": 20
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    // ValidationError maps to 422 (UNPROCESSABLE_ENTITY).
    assert_eq!(upload.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
