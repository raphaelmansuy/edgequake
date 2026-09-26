//! SPEC-045 — Periodic orphan document KV sync battle tests.

use std::sync::Arc;

use edgequake_api::services::{
    extract_document_id_from_task, sync_document_failed_on_orphan_heartbeat,
};
use edgequake_storage::adapters::memory::MemoryKVStorage;
use edgequake_storage::traits::KVStorage;
use edgequake_tasks::{Task, TaskStatus, TaskType};
use serde_json::json;

#[tokio::test]
async fn spec045_orphan_heartbeat_syncs_document_to_failed() {
    let kv: Arc<dyn KVStorage> = Arc::new(MemoryKVStorage::new("spec045-orphan"));
    kv.initialize().await.unwrap();

    let doc_id = "f6fa9cad-bbff-4892-a855-3bd7d70da044";
    let metadata_key = format!("{doc_id}-metadata");
    kv.upsert(&[(
        metadata_key.clone(),
        json!({
            "id": doc_id,
            "status": "processing",
            "current_stage": "extracting",
            "workspace_id": "default",
        }),
    )])
    .await
    .unwrap();

    let task = Task {
        track_id: "task-orphan-1".to_string(),
        tenant_id: uuid::Uuid::new_v4(),
        workspace_id: uuid::Uuid::new_v4(),
        task_type: TaskType::Insert,
        status: TaskStatus::Failed,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
        error_message: Some("heartbeat lost".to_string()),
        error: None,
        retry_count: 0,
        max_retries: 3,
        consecutive_timeout_failures: 0,
        circuit_breaker_tripped: false,
        task_data: json!({
            "metadata": { "document_id": doc_id }
        }),
        metadata: None,
        progress: None,
        result: None,
        lease_owner: None,
        lease_token: None,
        lease_expires_at: None,
        fairness_hold_until: None,
    };

    let err_msg = "Task heartbeat lost (no update for 12 minutes). The worker may have crashed.";
    sync_document_failed_on_orphan_heartbeat(Arc::clone(&kv), None, &task, err_msg)
        .await
        .expect("sync");

    let updated = kv.get_by_id(&metadata_key).await.unwrap().unwrap();
    assert_eq!(
        updated.get("status").and_then(|v| v.as_str()),
        Some("failed")
    );
    assert_eq!(
        updated.get("failure_class").and_then(|v| v.as_str()),
        Some("unknown")
    );
    assert!(updated.get("error_message").is_some());
}

#[test]
fn spec045_extract_document_id_wired_in_main_periodic_orphan() {
    let main_rs = include_str!("../../../src/main.rs");
    assert!(main_rs.contains("sync_document_failed_on_orphan_heartbeat"));
    assert!(main_rs.contains("periodic_kv_storage"));
    assert!(
        main_rs.contains("periodic_pg_pool"),
        "periodic orphan check must thread pg_pool into heartbeat sync"
    );
    assert!(extract_document_id_from_task(&Task::new(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        TaskType::Insert,
        json!({ "metadata": { "document_id": "x" } }),
    ))
    .is_some());
}
