//! SPEC-026 Phase 2 — admission staging contract tests (P-11).

mod common;

use std::sync::Arc;

use edgequake_api::middleware::TenantContext;
use edgequake_api::services::{
    admit_document_for_processing, promote_staging_to_final, rollback_staging, ContentHasher,
    DocumentAdmissionInput, DocumentAdmissionOutcome, GleaningAdmissionOptions,
};
use edgequake_api::AppState;
use edgequake_storage::kv_keys;
use edgequake_storage::traits::KVStorage;

fn memory_kv() -> Arc<dyn KVStorage> {
    Arc::new(edgequake_storage::adapters::memory::MemoryKVStorage::new(
        "test",
    ))
}

fn tenant_ctx() -> TenantContext {
    TenantContext {
        tenant_id: Some(common::TEST_TENANT_ID.to_string()),
        workspace_id: Some(common::TEST_WORKSPACE_ID.to_string()),
        user_id: Some(common::TEST_USER_ID.to_string()),
    }
}

fn sample_input(content: &str, hash: &str) -> DocumentAdmissionInput {
    DocumentAdmissionInput {
        text_content: content.to_string(),
        title: "spec026-staging.txt".to_string(),
        source_type: "text",
        mime_type: Some("text/plain".to_string()),
        raw_byte_size: content.len(),
        content_hash: hash.to_string(),
        custom_metadata: None,
        track_id: None,
        expected_batch_count: None,
        gleaning: GleaningAdmissionOptions::default(),
        document_type: None,
        chunk_strategy: None,
        chunk_options: None,
        multimodal: false,
        ingest_mode: None,
        multimodal_manifest: None,
    }
}

#[tokio::test]
async fn admit_writes_staging_keys_not_final() {
    let state = AppState::new_memory(None::<String>);
    state.workspace_service.seed_default_workspace().await;

    let content = "Staging admit: Dr. Sarah Chen leads EdgeQuake.";
    let hash = ContentHasher::hash_str(content);
    let outcome = admit_document_for_processing(
        &state,
        &tenant_ctx(),
        sample_input(content, &hash),
        "spec026",
    )
    .await
    .expect("admit");

    let doc_id = match outcome {
        DocumentAdmissionOutcome::Accepted(a) => a.document_id,
        _ => panic!("expected accepted"),
    };

    assert!(state
        .storage
        .kv_storage
        .get_by_id(&kv_keys::staging_doc_metadata(&doc_id))
        .await
        .unwrap()
        .is_some());
    assert!(state
        .storage
        .kv_storage
        .get_by_id(&kv_keys::staging_doc_content(&doc_id))
        .await
        .unwrap()
        .is_some());
    assert!(state
        .storage
        .kv_storage
        .get_by_id(&kv_keys::doc_metadata(&doc_id))
        .await
        .unwrap()
        .is_none());
    assert!(state
        .storage
        .kv_storage
        .get_by_id(&kv_keys::doc_content(&doc_id))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn promote_copies_staging_to_final() {
    let kv = memory_kv();
    let doc_id = "doc-promote";
    let ws = common::TEST_WORKSPACE_ID;
    let hash = "abc123";

    kv.upsert(&[
        (
            kv_keys::staging_doc_metadata(doc_id),
            serde_json::json!({"status": "pending"}),
        ),
        (
            kv_keys::staging_doc_content(doc_id),
            serde_json::json!({"content": "hello"}),
        ),
        (
            kv_keys::staging_workspace_hash(ws, hash),
            serde_json::json!(doc_id),
        ),
    ])
    .await
    .unwrap();

    promote_staging_to_final(&kv, doc_id, ws, hash)
        .await
        .expect("promote");

    assert!(kv
        .get_by_id(&kv_keys::doc_metadata(doc_id))
        .await
        .unwrap()
        .is_some());
    assert!(kv
        .get_by_id(&kv_keys::doc_content(doc_id))
        .await
        .unwrap()
        .is_some());
    assert!(kv
        .get_by_id(&kv_keys::staging_doc_content(doc_id))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn rollback_deletes_staging_on_failure() {
    let kv = memory_kv();
    let doc_id = "doc-rollback";
    let ws = common::TEST_WORKSPACE_ID;
    let hash = "deadbeef";

    kv.upsert(&[(
        kv_keys::staging_doc_content(doc_id),
        serde_json::json!({"content": "orphan"}),
    )])
    .await
    .unwrap();

    rollback_staging(&kv, doc_id, ws, hash)
        .await
        .expect("rollback");

    assert!(kv
        .get_by_id(&kv_keys::staging_doc_content(doc_id))
        .await
        .unwrap()
        .is_none());
    assert!(kv
        .get_by_id(&kv_keys::doc_content(doc_id))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn failed_doc_has_no_orphan_content_kv() {
    let kv = memory_kv();
    let doc_id = "doc-failed";
    let ws = common::TEST_WORKSPACE_ID;
    let hash = "failhash";

    kv.upsert(&[(
        kv_keys::staging_doc_content(doc_id),
        serde_json::json!({"content": "will fail"}),
    )])
    .await
    .unwrap();

    rollback_staging(&kv, doc_id, ws, hash)
        .await
        .expect("rollback simulates worker failure");

    assert!(kv
        .get_by_id(&kv_keys::doc_content(doc_id))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn duplicate_hash_during_staging_rejected() {
    let state = AppState::new_memory(None::<String>);
    state.workspace_service.seed_default_workspace().await;

    let content = "Duplicate staging hash test content.";
    let hash = ContentHasher::hash_str(content);

    let first = admit_document_for_processing(
        &state,
        &tenant_ctx(),
        sample_input(content, &hash),
        "spec026",
    )
    .await
    .expect("first admit");

    let first_id = match first {
        DocumentAdmissionOutcome::Accepted(a) => a.document_id,
        _ => panic!("expected first accepted"),
    };

    let second = admit_document_for_processing(
        &state,
        &tenant_ctx(),
        sample_input(content, &hash),
        "spec026",
    )
    .await
    .expect("second admit");

    match second {
        DocumentAdmissionOutcome::DuplicateProcessing(dup) => {
            assert_eq!(dup.document_id, first_id);
        }
        other => panic!("expected duplicate processing, got {:?}", other),
    }
}
