//! 068 — Text/MD ingest progress identity parity with PDF (SPEC-054 task_id SSOT).
//!
//! Admits a text document, asserts `track_id == task_id` (`insert-*`), and that
//! `GET /ingestion/{track_id}/progress` returns 200 while staging metadata exists
//! (even when the workspace already has other final docs / non-empty wsdoc index).

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::middleware::{default_tenant_uuid, default_workspace_uuid, TenantContext};
use edgequake_api::services::{
    admit_document_for_processing, ContentHasher, DocumentAdmissionInput, DocumentAdmissionOutcome,
    GleaningAdmissionOptions,
};
use edgequake_api::{create_router, AppState};
use edgequake_storage::kv_keys;
use serde_json::{json, Value};
use tower::ServiceExt;

fn default_ids() -> (String, String) {
    (
        default_tenant_uuid().to_string(),
        default_workspace_uuid().to_string(),
    )
}

fn tenant_ctx() -> TenantContext {
    let (tenant_id, workspace_id) = default_ids();
    TenantContext {
        tenant_id: Some(tenant_id),
        workspace_id: Some(workspace_id),
        user_id: Some(common::TEST_USER_ID.to_string()),
    }
}

fn sample_md_input(
    content: &str,
    hash: &str,
    client_track: Option<&str>,
) -> DocumentAdmissionInput {
    DocumentAdmissionInput {
        text_content: content.to_string(),
        title: "long_2607.14952v2.md".to_string(),
        source_type: "markdown",
        mime_type: Some("text/markdown".to_string()),
        raw_byte_size: content.len(),
        content_hash: hash.to_string(),
        custom_metadata: None,
        track_id: client_track.map(|s| s.to_string()),
        expected_batch_count: None,
        gleaning: GleaningAdmissionOptions::default(),
        document_type: Some("markdown"),
        chunk_strategy: None,
        chunk_options: None,
        multimodal: false,
        ingest_mode: None,
        multimodal_manifest: None,
    }
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn admit_sets_track_id_equal_to_insert_task_id() {
    // test_state: auth disabled (same harness as contract_spec048_progress)
    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;

    let content = "# Title\n\nMarkdown body for 068 progress parity.";
    let hash = ContentHasher::hash_str(content);
    let outcome = admit_document_for_processing(
        &state,
        &tenant_ctx(),
        sample_md_input(content, &hash, Some("batch-client-1")),
        "upload",
    )
    .await
    .expect("admit");

    let accepted = match outcome {
        DocumentAdmissionOutcome::Accepted(a) => a,
        other => panic!("expected Accepted, got {other:?}"),
    };

    assert!(
        accepted.task_id.starts_with("insert-"),
        "task_id={}",
        accepted.task_id
    );
    assert_eq!(
        accepted.track_id, accepted.task_id,
        "068: response track_id must equal task_id"
    );

    let staging = state
        .storage
        .kv_storage
        .get_by_id(&kv_keys::staging_doc_metadata(&accepted.document_id))
        .await
        .unwrap()
        .expect("staging metadata");
    assert_eq!(
        staging.get("track_id").and_then(|v| v.as_str()),
        Some(accepted.task_id.as_str())
    );
    assert_eq!(
        staging.get("task_id").and_then(|v| v.as_str()),
        Some(accepted.task_id.as_str())
    );
    assert_eq!(
        staging.get("client_track_id").and_then(|v| v.as_str()),
        Some("batch-client-1")
    );
}

#[tokio::test]
async fn ingestion_progress_200_for_insert_track_with_staging_and_wsdoc() {
    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;
    let (tenant_id, workspace_id) = default_ids();

    // Seed a final doc so wsdoc index is non-empty (RC4 regression).
    let existing_id = "existing-final-doc";
    let final_meta = json!({
        "id": existing_id,
        "title": "prior.md",
        "status": "completed",
        "track_id": "insert-old",
        "tenant_id": tenant_id,
        "workspace_id": workspace_id,
    });
    edgequake_api::services::workspace_document_index::upsert_final_document_metadata(
        state.storage.kv_storage.as_ref(),
        existing_id,
        final_meta,
    )
    .await
    .unwrap();

    let content = "# New\n\nIn-flight markdown.";
    let hash = ContentHasher::hash_str(content);
    let outcome = admit_document_for_processing(
        &state,
        &tenant_ctx(),
        sample_md_input(content, &hash, None),
        "upload",
    )
    .await
    .expect("admit");
    let accepted = match outcome {
        DocumentAdmissionOutcome::Accepted(a) => a,
        other => panic!("expected Accepted, got {other:?}"),
    };

    let app = create_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/ingestion/{}/progress", accepted.track_id))
                .header("X-Tenant-ID", tenant_id)
                .header("X-Workspace-ID", workspace_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "progress must not 404 for insert-* while staging"
    );
    let body = json_body(response).await;
    assert_eq!(body["track_id"].as_str(), Some(accepted.track_id.as_str()));
    assert_eq!(
        body["document_id"].as_str(),
        Some(accepted.document_id.as_str())
    );
}
