//! 086 — In-flight MD staging visible on list / track / pipeline activity (SPEC-086 Wave 1).
//!
//! Extends 068: progress already merges staging; list/track/activity must use the same SSOT.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::middleware::{default_tenant_uuid, default_workspace_uuid, TenantContext};
use edgequake_api::services::{
    admit_document_for_processing, ContentHasher, DocumentAdmissionAccepted, DocumentAdmissionInput,
    DocumentAdmissionOutcome, GleaningAdmissionOptions,
};
use edgequake_api::{create_router, AppState};
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

fn sample_md_input(content: &str, hash: &str) -> DocumentAdmissionInput {
    DocumentAdmissionInput {
        text_content: content.to_string(),
        title: "auto_disco_086.md".to_string(),
        source_type: "markdown",
        mime_type: Some("text/markdown".to_string()),
        raw_byte_size: content.len(),
        content_hash: hash.to_string(),
        custom_metadata: None,
        track_id: Some("batch-086".to_string()),
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

async fn admit_with_seeded_final(state: &AppState) -> DocumentAdmissionAccepted {
    let (tenant_id, workspace_id) = default_ids();
    // Seed a final doc so wsdoc index is non-empty (same RC4 class as 068).
    let existing_id = "existing-final-doc-086";
    let final_meta = json!({
        "id": existing_id,
        "title": "prior.md",
        "status": "completed",
        "track_id": "insert-old-086",
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

    let content = "# 086\n\nIn-flight markdown for list visibility.";
    let hash = ContentHasher::hash_str(content);
    let outcome = admit_document_for_processing(
        state,
        &tenant_ctx(),
        sample_md_input(content, &hash),
        "upload",
    )
    .await
    .expect("admit");
    match outcome {
        DocumentAdmissionOutcome::Accepted(a) => a,
        other => panic!("expected Accepted, got {other:?}"),
    }
}

/// ux086_v_staging_list — GET /documents includes staging in-flight MD.
#[tokio::test]
async fn list_documents_includes_staging_inflight_md() {
    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;
    let (tenant_id, workspace_id) = default_ids();
    let accepted = admit_with_seeded_final(&state).await;

    let app = create_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/documents")
                .header("X-Tenant-ID", &tenant_id)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let docs = body["documents"]
        .as_array()
        .expect("documents array");
    // SSOT: list id must be bare admit document_id (not staging:{id}).
    let found = docs
        .iter()
        .any(|d| d["id"].as_str() == Some(accepted.document_id.as_str()));
    assert!(
        found,
        "staging MD must appear in GET /documents with bare id={}; body={}",
        accepted.document_id,
        body
    );
    assert!(
        !docs.iter().any(|d| {
            d["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("staging:"))
        }),
        "list must not emit staging: prefixed ids; body={}",
        body
    );
}

/// ux086_v_staging_list — GET /documents/track/{insert-*} non-empty while staging.
#[tokio::test]
async fn track_status_includes_staging_insert_track() {
    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;
    let (tenant_id, workspace_id) = default_ids();
    let accepted = admit_with_seeded_final(&state).await;

    let app = create_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/documents/track/{}", accepted.track_id))
                .header("X-Tenant-ID", &tenant_id)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let docs = body["documents"].as_array().expect("track documents array");
    assert!(
        !docs.is_empty(),
        "track status must list staging insert; body={}",
        body
    );
    let found = docs
        .iter()
        .any(|d| d["id"].as_str() == Some(accepted.document_id.as_str()));
    assert!(found, "expected document_id in track response; body={}", body);
}

/// ux086_v_staging_list — pipeline activity sees staging MD as queued/working.
#[tokio::test]
async fn pipeline_activity_includes_staging_md() {
    let state = AppState::test_state();
    state.workspace_service.seed_default_workspace().await;
    let (tenant_id, workspace_id) = default_ids();
    let accepted = admit_with_seeded_final(&state).await;

    let app = create_router(state);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/pipeline/activity")
                .header("X-Tenant-ID", &tenant_id)
                .header("X-Workspace-ID", &workspace_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    let queued = body["queued"].as_array().cloned().unwrap_or_default();
    let working = body["working"].as_array().cloned().unwrap_or_default();
    let found = queued
        .iter()
        .chain(working.iter())
        .any(|d| {
            d["document_id"].as_str() == Some(accepted.document_id.as_str())
                || d["id"].as_str() == Some(accepted.document_id.as_str())
                || d["track_id"].as_str() == Some(accepted.track_id.as_str())
        });
    assert!(
        found,
        "staging MD must appear in pipeline activity queued/working; body={}",
        body
    );
}

/// ux086_v_reingest_fail_closed — delete Err must not clear for a second admit.
#[test]
fn contract_reingest_delete_err_fail_closed() {
    let src = include_str!("../src/services/document_reingest.rs");
    assert!(
        src.contains("blocking re-ingestion")
            && src.contains("Failed to delete old document data"),
        "ux086_reingest_fail_closed: delete Err must log and block re-ingestion"
    );
    let err_idx = src
        .find("Err(e) =>")
        .expect("delete match Err arm in resolve_workspace_duplicate_for_reingestion");
    let after_err = &src[err_idx..];
    let cleared_rel = after_err.find("ClearedForReingestion");
    let still_rel = after_err.find("StillProcessing");
    assert!(
        still_rel.is_some(),
        "Err arm must return StillProcessing (fail closed)"
    );
    assert!(
        cleared_rel.is_none() || still_rel.unwrap() < cleared_rel.unwrap(),
        "Err arm must prefer StillProcessing over ClearedForReingestion"
    );
}
