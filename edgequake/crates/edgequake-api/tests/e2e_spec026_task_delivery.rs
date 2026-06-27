//! SPEC-026 Phase 4 — external task delivery E2E.

mod common;

use common::spec026_multimodal::{parse_accepted_upload, text_upload_request};
use edgequake_storage::EntityId;
use serial_test::serial;
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
#[serial]
async fn bridged_delivery_processes_text_upload() {
    std::env::set_var("EDGEQUAKE_TASK_DELIVERY", "bridged");
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let (_, track_id) = parse_accepted_upload(
        app.clone()
            .oneshot(text_upload_request(
                "spec026-bridged.txt",
                "Bridged delivery: Dr. Sarah Chen leads EdgeQuake.",
            ))
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(
        common::wait_for_document_processed(app, &track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(workers
        .graph_storage
        .get_node(&node_id)
        .await
        .unwrap()
        .is_some());

    std::env::remove_var("EDGEQUAKE_TASK_DELIVERY");
}

/// External worker path: notify_only + StorageHydratingTaskQueue (Postgres SSOT).
#[tokio::test]
#[serial]
async fn storage_hydrating_worker_processes_task() {
    std::env::set_var("EDGEQUAKE_TASK_DELIVERY", "notify_only");
    let workers = common::create_test_app_with_workers().await;
    let app = workers.app();

    let (_, track_id) = parse_accepted_upload(
        app.clone()
            .oneshot(text_upload_request(
                "spec026-hydrating.txt",
                "Hydrating worker: Dr. Sarah Chen leads EdgeQuake.",
            ))
            .await
            .unwrap(),
    )
    .await;

    assert_eq!(
        common::wait_for_document_processed(app, &track_id, Duration::from_secs(90)).await,
        "completed"
    );

    let node_id = EntityId::new("Sarah Chen").as_graph_node_id().to_string();
    assert!(workers
        .graph_storage
        .get_node(&node_id)
        .await
        .unwrap()
        .is_some());

    std::env::remove_var("EDGEQUAKE_TASK_DELIVERY");
}
