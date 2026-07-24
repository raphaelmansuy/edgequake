//! SPEC-084 / GH-316 — workspace-fair claim interleaves workspaces.

use edgequake_tasks::memory::MemoryTaskStorage;
use edgequake_tasks::{Task, TaskStorage, TaskType};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn issue316_workspace_with_oldest_pending_wins_memory() {
    let storage = Arc::new(MemoryTaskStorage::new());
    let tenant = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let ws_a = Uuid::parse_str("00000000-0000-0000-0000-0000000000aa").unwrap();
    let ws_b = Uuid::parse_str("00000000-0000-0000-0000-0000000000bb").unwrap();

    let a = Task::new(
        tenant,
        ws_a,
        TaskType::Insert,
        serde_json::json!({"document_id":"a","content":"x"}),
    );
    storage.create_task(&a).await.unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    let b = Task::new(
        tenant,
        ws_b,
        TaskType::Insert,
        serde_json::json!({"document_id":"b","content":"y"}),
    );
    storage.create_task(&b).await.unwrap();

    // WS-A is older → claimed first.
    let c1 = storage
        .claim_next("w1", Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(c1.workspace_id, ws_a);

    // With A active, least-loaded policy prefers B next (interleave).
    let c2 = storage
        .claim_next("w1", Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(c2.workspace_id, ws_b);
}

#[tokio::test]
async fn issue316_second_workspace_not_starved_by_first_backlog_memory() {
    let storage = Arc::new(MemoryTaskStorage::new());
    let tenant = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let ws_a = Uuid::parse_str("00000000-0000-0000-0000-0000000000aa").unwrap();
    let ws_b = Uuid::parse_str("00000000-0000-0000-0000-0000000000bb").unwrap();

    // WS-A floods first (older backlog).
    for i in 0..8 {
        let a = Task::new(
            tenant,
            ws_a,
            TaskType::Insert,
            serde_json::json!({"document_id": format!("a{i}"), "content":"x"}),
        );
        storage.create_task(&a).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(5)).await;

    // WS-B arrives later with one task.
    let b = Task::new(
        tenant,
        ws_b,
        TaskType::Insert,
        serde_json::json!({"document_id":"b0","content":"y"}),
    );
    storage.create_task(&b).await.unwrap();

    let first = storage
        .claim_next("w1", Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.workspace_id, ws_a, "oldest backlog claimed first");

    // After A holds one active lease, B must not wait for A's entire backlog.
    let second = storage
        .claim_next("w2", Duration::from_secs(30))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        second.workspace_id, ws_b,
        "workspace with zero active load must interleave ahead of saturated backlog"
    );
}

#[tokio::test]
async fn issue316_tenant_cap_still_holds_via_workspace_lane() {
    use edgequake_tasks::{FairnessClass, TenantConcurrencyLimiter, TryAcquireOutcome};

    let limiter = TenantConcurrencyLimiter::new(2, 2);
    let tenant = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let ws_a = Uuid::parse_str("00000000-0000-0000-0000-0000000000aa").unwrap();
    let ws_b = Uuid::parse_str("00000000-0000-0000-0000-0000000000bb").unwrap();

    let p_a = match limiter
        .try_acquire(tenant, ws_a, FairnessClass::Ingest)
        .await
    {
        TryAcquireOutcome::Acquired(p) => p,
        other => panic!("expected Acquired for ws_a, got {other:?}"),
    };
    let p_b = match limiter
        .try_acquire(tenant, ws_b, FairnessClass::Ingest)
        .await
    {
        TryAcquireOutcome::Acquired(p) => p,
        other => panic!("expected Acquired for ws_b, got {other:?}"),
    };

    // Tenant ingest cap = 2 → third workspace parks even if its lane is free.
    assert!(matches!(
        limiter
            .try_acquire(
                tenant,
                Uuid::parse_str("00000000-0000-0000-0000-0000000000cc").unwrap(),
                FairnessClass::Ingest
            )
            .await,
        TryAcquireOutcome::AtCapacity
    ));

    // Same workspace cannot take a second ingest slot (workspace lane = 1).
    assert!(matches!(
        limiter
            .try_acquire(tenant, ws_a, FairnessClass::Ingest)
            .await,
        TryAcquireOutcome::AtCapacity
    ));

    drop(p_a);
    drop(p_b);
}
