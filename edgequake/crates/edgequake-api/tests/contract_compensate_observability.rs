//! SPEC-057 P3: compensate DLQ + store_contention shape on queue-metrics / ready.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{create_test_app, extract_json, TEST_TENANT_ID, TEST_USER_ID, TEST_WORKSPACE_ID};
use edgequake_storage::{
    adapters::memory::{MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage},
    compensate_merge_failure_with_kv, compensation_quarantine_total, kv_keys, GraphStorage,
    KVStorage, StorageError, VectorSearchResult, VectorStorage,
};
use std::sync::atomic::{AtomicBool, Ordering};
use tower::ServiceExt;

struct FailDelete {
    inner: MemoryVectorStorage,
    failed: AtomicBool,
}

#[async_trait::async_trait]
impl VectorStorage for FailDelete {
    fn namespace(&self) -> &str {
        self.inner.namespace()
    }
    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
    async fn initialize(&self) -> edgequake_storage::error::Result<()> {
        self.inner.initialize().await
    }
    async fn finalize(&self) -> edgequake_storage::error::Result<()> {
        self.inner.finalize().await
    }
    async fn query(
        &self,
        q: &[f32],
        k: usize,
        f: Option<&[String]>,
    ) -> edgequake_storage::error::Result<Vec<VectorSearchResult>> {
        self.inner.query(q, k, f).await
    }
    async fn upsert(
        &self,
        data: &[(String, Vec<f32>, serde_json::Value)],
    ) -> edgequake_storage::error::Result<()> {
        self.inner.upsert(data).await
    }
    async fn delete(&self, ids: &[String]) -> edgequake_storage::error::Result<()> {
        if !self.failed.swap(true, Ordering::SeqCst) {
            return Err(StorageError::Database("contract inject-fail".into()));
        }
        self.inner.delete(ids).await
    }
    async fn delete_entity(&self, n: &str) -> edgequake_storage::error::Result<()> {
        self.inner.delete_entity(n).await
    }
    async fn delete_entity_relations(&self, n: &str) -> edgequake_storage::error::Result<()> {
        self.inner.delete_entity_relations(n).await
    }
    async fn get_by_id(&self, id: &str) -> edgequake_storage::error::Result<Option<Vec<f32>>> {
        self.inner.get_by_id(id).await
    }
    async fn get_by_ids(
        &self,
        ids: &[String],
    ) -> edgequake_storage::error::Result<Vec<(String, Vec<f32>)>> {
        self.inner.get_by_ids(ids).await
    }
    async fn is_empty(&self) -> edgequake_storage::error::Result<bool> {
        self.inner.is_empty().await
    }
    async fn count(&self) -> edgequake_storage::error::Result<usize> {
        self.inner.count().await
    }
    async fn clear(&self) -> edgequake_storage::error::Result<()> {
        self.inner.clear().await
    }
    async fn clear_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> edgequake_storage::error::Result<usize> {
        self.inner.clear_workspace(workspace_id).await
    }
    async fn delete_by_document(
        &self,
        document_id: &str,
    ) -> edgequake_storage::error::Result<usize> {
        self.inner.delete_by_document(document_id).await
    }
    async fn query_filtered(
        &self,
        query_embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&edgequake_storage::MetadataFilter>,
    ) -> edgequake_storage::error::Result<Vec<VectorSearchResult>> {
        self.inner
            .query_filtered(query_embedding, top_k, filter_ids, metadata_filter)
            .await
    }
}

#[tokio::test]
async fn queue_metrics_includes_store_contention_object() {
    let app = create_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/pipeline/queue-metrics")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = extract_json(response).await;
    let store = json
        .get("store_contention")
        .expect("store_contention nested object");
    assert!(store.get("level").is_some());
    assert!(store.get("compensation_quarantine_total").is_some());
    assert!(store.get("db_pool_util_warn").is_some());
    assert!(store.get("compensation_quarantine_critical").is_some());
}

#[tokio::test]
async fn inject_quarantine_visible_on_queue_metrics() {
    let before = compensation_quarantine_total();
    let graph = MemoryGraphStorage::new("contract");
    graph.initialize().await.unwrap();
    let vector = FailDelete {
        inner: MemoryVectorStorage::new("contract", 4),
        failed: AtomicBool::new(false),
    };
    vector.initialize().await.unwrap();
    vector
        .upsert(&[("c-chunk-0".to_string(), vec![0.1; 4], serde_json::json!({}))])
        .await
        .unwrap();
    let kv = MemoryKVStorage::new("contract");
    kv.initialize().await.unwrap();

    compensate_merge_failure_with_kv(
        &graph,
        &vector,
        Some(&kv as &dyn KVStorage),
        "contract-doc",
        &["c-chunk-0".to_string()],
        &[],
        &[],
        &[],
        &[],
        &[],
        "contract inject",
    )
    .await;

    assert!(
        compensation_quarantine_total() > before,
        "quarantine metric must increment"
    );
    let keys = kv
        .keys_with_prefix(&kv_keys::compensation_quarantine_prefix("contract-doc"))
        .await
        .unwrap();
    assert!(!keys.is_empty(), "KV DLQ key must exist");

    // Critical floor is warn+1 when both are set; use warn=0 so critical=1 is honored.
    std::env::set_var("EDGEQUAKE_COMPENSATION_QUARANTINE_WARN", "0");
    std::env::set_var("EDGEQUAKE_COMPENSATION_QUARANTINE_CRITICAL", "1");

    let app = create_test_app();
    let metrics = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/pipeline/queue-metrics")
                .header("X-Tenant-ID", TEST_TENANT_ID)
                .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                .header("X-User-ID", TEST_USER_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(metrics.status(), StatusCode::OK);
    let json = extract_json(metrics).await;
    let total = json["store_contention"]["compensation_quarantine_total"]
        .as_u64()
        .unwrap_or(0);
    assert!(total >= 1);
    assert_eq!(json["store_contention"]["level"].as_str(), Some("critical"));

    // Assessor SSOT used by /ready (handlers only project).
    assert!(edgequake_api::store_contention::readiness_blocked_by_store(
        None
    ));

    #[cfg(feature = "postgres")]
    {
        let ready = create_test_app()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = extract_json(ready).await;
        let blockers = body["blockers"].as_array().cloned().unwrap_or_default();
        let joined = blockers
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(body["ready"], false);
        assert!(
            joined.contains("store_contention"),
            "expected store_contention ready blocker when quarantine critical, got {body}"
        );
    }

    std::env::remove_var("EDGEQUAKE_COMPENSATION_QUARANTINE_CRITICAL");
    std::env::remove_var("EDGEQUAKE_COMPENSATION_QUARANTINE_WARN");
}
