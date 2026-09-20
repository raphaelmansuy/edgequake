//! Cross-store saga compensation helpers (SPEC-021 P-C1 / SPEC-057 P3).
//!
//! WHY: the orchestrator path (`EdgeQuake::insert`) already rolls back chunk
//! vectors when the graph merge fails (`ingestion.rs::fail_with_chunk_vector_rollback`).
//! The processor path (`DocumentTaskProcessor::process_text_insert`) does not,
//! so a graph-batch failure there orphans chunk + entity vectors. This module
//! is the SINGLE shared implementation so both paths converge on identical
//! cleanup semantics (DRY).
//!
//! Principles:
//! - **Best-effort**: never returns an error. Compensation runs on an
//!   already-failing path; masking the original error would be worse.
//! - **Idempotent**: deletion is keyed by exact vector IDs, so retrying is safe.
//! - **Observable**: on cleanup failure, emits metric + durable KV DLQ record
//!   (`compensation_quarantine:{document_id}:{uuid}`) so operators can reconcile.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use crate::kv_key_schema::kv_keys;
use crate::traits::{GraphStorage, KVStorage, VectorStorage};

/// SPEC-091 Wave B2: typed quarantine destination (DIP).
///
/// WHY a global: compensation is a cross-cutting best-effort DLQ write
/// reachable from 10+ public call sites; threading a sink through every
/// signature would churn the entire storage/pipeline API for no behavioural
/// gain. The sink is installed once at startup (Postgres state wiring) via
/// [`set_quarantine_sink`]; when absent, `quarantine()` falls back to the
/// legacy KV record (tests + non-postgres builds).
#[async_trait::async_trait]
pub trait QuarantineSink: Send + Sync {
    async fn insert(
        &self,
        document_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), crate::error::StorageError>;
}

static QUARANTINE_SINK: OnceLock<Arc<dyn QuarantineSink>> = OnceLock::new();

/// Install the typed quarantine sink. First call wins; subsequent calls warn.
pub fn set_quarantine_sink(sink: Arc<dyn QuarantineSink>) {
    if QUARANTINE_SINK.set(sink).is_err() {
        tracing::warn!("quarantine sink already installed; ignoring duplicate set");
    }
}

/// Process-local quarantine counter (SSOT for store_contention assessor + tests).
static COMPENSATION_QUARANTINE_TOTAL: AtomicU64 = AtomicU64::new(0);

/// SPEC-058: entity/rel vectors that already existed and were skipped from
/// compensate artifact lists (shared across documents).
static COMPENSATE_SHARED_ENTITY_SKIPPED_TOTAL: AtomicU64 = AtomicU64::new(0);

/// SPEC-058: dimension mismatch rejected (fail-closed, no DROP).
static VECTOR_DIM_MISMATCH_REJECTED_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Snapshot of compensation quarantine events since process start.
pub fn compensation_quarantine_total() -> u64 {
    COMPENSATION_QUARANTINE_TOTAL.load(Ordering::Relaxed)
}

/// Snapshot of shared-entity vector IDs excluded from compensate delete lists.
pub fn compensate_shared_entity_skipped_total() -> u64 {
    COMPENSATE_SHARED_ENTITY_SKIPPED_TOTAL.load(Ordering::Relaxed)
}

/// Record N shared entity/rel vectors that pre-existed (not compensatable).
pub fn record_compensate_shared_entity_skipped(n: usize) {
    if n == 0 {
        return;
    }
    COMPENSATE_SHARED_ENTITY_SKIPPED_TOTAL.fetch_add(n as u64, Ordering::Relaxed);
    #[cfg(feature = "observability")]
    edgequake_observability::record_compensate_shared_entity_skipped(n as u64);
}

/// Snapshot of fail-closed dimension mismatch rejections.
pub fn vector_dim_mismatch_rejected_total() -> u64 {
    VECTOR_DIM_MISMATCH_REJECTED_TOTAL.load(Ordering::Relaxed)
}

/// Record a refused vector table rebuild due to dimension mismatch.
pub fn record_vector_dim_mismatch_rejected() {
    VECTOR_DIM_MISMATCH_REJECTED_TOTAL.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "observability")]
    edgequake_observability::record_vector_dim_mismatch_rejected();
}

#[cfg(test)]
pub fn reset_compensation_quarantine_total_for_tests() {
    COMPENSATION_QUARANTINE_TOTAL.store(0, Ordering::Relaxed);
}

#[cfg(test)]
pub fn reset_compensate_shared_entity_skipped_for_tests() {
    COMPENSATE_SHARED_ENTITY_SKIPPED_TOTAL.store(0, Ordering::Relaxed);
}

/// Record quarantine metric when observability feature is enabled (SPEC-045 SRE-I07).
#[cfg(feature = "observability")]
fn record_compensation_quarantine_metric(kind: &str) {
    edgequake_observability::record_compensation_quarantine(kind);
}

#[cfg(not(feature = "observability"))]
fn record_compensation_quarantine_metric(_kind: &str) {}

async fn quarantine(
    kv_storage: Option<&dyn KVStorage>,
    doc_id: &str,
    kind: &str,
    artifact_id: &str,
    cause: &str,
    cleanup_error: &str,
) {
    COMPENSATION_QUARANTINE_TOTAL.fetch_add(1, Ordering::Relaxed);
    record_compensation_quarantine_metric(kind);

    tracing::error!(
        document_id = %doc_id,
        kind = %kind,
        artifact_id = %artifact_id,
        merge_cause = %cause,
        cleanup_error = %cleanup_error,
        "quarantine: failed compensation cleanup; durable DLQ write attempted"
    );

    let record = serde_json::json!({
        "kind": kind,
        "id": artifact_id,
        "cause": cause,
        "cleanup_error": cleanup_error,
        "ts": chrono::Utc::now().to_rfc3339(),
    });

    // SPEC-091 Wave B2: typed table first (public.compensation_quarantine,
    // migration 107) when the sink is installed; KV is the fallback when the
    // typed write is unavailable or fails — never silently drop a DLQ record.
    if let Some(sink) = QUARANTINE_SINK.get() {
        match sink.insert(doc_id, record.clone()).await {
            Ok(()) => return,
            Err(e) => tracing::warn!(
                document_id = %doc_id,
                error = %e,
                "typed compensation quarantine insert failed; falling back to KV"
            ),
        }
    }

    let Some(kv) = kv_storage else {
        return;
    };
    let entry_id = uuid::Uuid::new_v4().to_string();
    let key = kv_keys::compensation_quarantine(doc_id, &entry_id);
    if let Err(e) = kv.upsert(&[(key.clone(), record)]).await {
        tracing::error!(
            document_id = %doc_id,
            key = %key,
            error = %e,
            "failed to persist compensation quarantine DLQ record"
        );
    }
}

/// Roll back chunk KV records written before a failed merge (SPEC-046 OPS-P1.8).
pub async fn compensate_orphan_kv(
    kv_storage: &dyn KVStorage,
    doc_id: &str,
    chunk_kv_ids: &[String],
    cause: &str,
) {
    if chunk_kv_ids.is_empty() {
        return;
    }

    match kv_storage.delete(chunk_kv_ids).await {
        Ok(()) => {
            tracing::warn!(
                document_id = %doc_id,
                chunk_kv_deleted = chunk_kv_ids.len(),
                cause = %cause,
                "saga_compensation: rolled back orphan chunk KV after graph failure (SPEC-046 OPS-P1.8)"
            );
        }
        Err(cleanup_err) => {
            quarantine(
                Some(kv_storage),
                doc_id,
                "kv",
                &chunk_kv_ids.join(","),
                cause,
                &cleanup_err.to_string(),
            )
            .await;
        }
    }
}

/// Roll back chunk vectors (and optionally entity vectors) written earlier in
/// the ingestion saga after the graph merge failed.
pub async fn compensate_orphan_vectors(
    vector_storage: &dyn VectorStorage,
    doc_id: &str,
    chunk_vector_ids: &[String],
    entity_vector_ids: &[String],
    cause: &str,
) {
    compensate_orphan_vectors_with_kv(
        vector_storage,
        None,
        doc_id,
        chunk_vector_ids,
        entity_vector_ids,
        cause,
    )
    .await;
}

/// Same as [`compensate_orphan_vectors`] with optional KV DLQ on cleanup failure.
pub async fn compensate_orphan_vectors_with_kv(
    vector_storage: &dyn VectorStorage,
    kv_storage: Option<&dyn KVStorage>,
    doc_id: &str,
    chunk_vector_ids: &[String],
    entity_vector_ids: &[String],
    cause: &str,
) {
    let mut all_ids: Vec<String> =
        Vec::with_capacity(chunk_vector_ids.len() + entity_vector_ids.len());
    all_ids.extend(chunk_vector_ids.iter().cloned());
    all_ids.extend(entity_vector_ids.iter().cloned());

    if all_ids.is_empty() {
        return;
    }

    let chunk_n = chunk_vector_ids.len();
    let entity_n = entity_vector_ids.len();

    match vector_storage.delete(&all_ids).await {
        Ok(()) => {
            tracing::warn!(
                document_id = %doc_id,
                chunk_vectors_deleted = chunk_n,
                entity_vectors_deleted = entity_n,
                cause = %cause,
                "saga_compensation: rolled back orphan vectors after graph failure (SPEC-021 P-C1)"
            );
        }
        Err(cleanup_err) => {
            quarantine(
                kv_storage,
                doc_id,
                "vector",
                &all_ids.join(","),
                cause,
                &cleanup_err.to_string(),
            )
            .await;
        }
    }
}

/// Roll back graph nodes and edges created during a failed merge attempt (P-G5).
pub async fn compensate_orphan_graph_writes(
    graph_storage: &dyn GraphStorage,
    doc_id: &str,
    nodes_created: &[String],
    edges_created: &[(String, String)],
    cause: &str,
) {
    compensate_orphan_graph_writes_with_kv(
        graph_storage,
        None,
        doc_id,
        nodes_created,
        edges_created,
        cause,
    )
    .await;
}

/// Same as [`compensate_orphan_graph_writes`] with optional KV DLQ.
pub async fn compensate_orphan_graph_writes_with_kv(
    graph_storage: &dyn GraphStorage,
    kv_storage: Option<&dyn KVStorage>,
    doc_id: &str,
    nodes_created: &[String],
    edges_created: &[(String, String)],
    cause: &str,
) {
    for (source, target) in edges_created {
        if let Err(e) = graph_storage.delete_edge(source, target).await {
            quarantine(
                kv_storage,
                doc_id,
                "edge",
                &format!("{source}->{target}"),
                cause,
                &e.to_string(),
            )
            .await;
        }
    }

    // SPEC-060: batch delete (native ANY) — per-node Cypher DETACH was O(K) RTs.
    if let Err(e) = graph_storage.delete_nodes_batch(nodes_created).await {
        quarantine(
            kv_storage,
            doc_id,
            "node",
            &nodes_created.join(","),
            cause,
            &e.to_string(),
        )
        .await;
    }

    if !nodes_created.is_empty() || !edges_created.is_empty() {
        tracing::warn!(
            document_id = %doc_id,
            nodes_deleted = nodes_created.len(),
            edges_deleted = edges_created.len(),
            cause = %cause,
            "saga_compensation: rolled back orphan graph writes after merge failure (SPEC-021 P-G5)"
        );
    }
}

/// Full merge-stage compensation: chunk vectors, new-entity vectors, new-edge
/// vectors, newly created graph nodes/edges, and optional chunk KV (P-G5 + OPS-P1.8).
#[allow(clippy::too_many_arguments)] // saga rollback mirrors merge stage arity
pub async fn compensate_merge_failure(
    graph_storage: &dyn GraphStorage,
    vector_storage: &dyn VectorStorage,
    doc_id: &str,
    chunk_vector_ids: &[String],
    entity_vector_ids: &[String],
    relationship_vector_ids: &[String],
    nodes_created: &[String],
    edges_created: &[(String, String)],
    cause: &str,
) {
    compensate_merge_failure_with_kv(
        graph_storage,
        vector_storage,
        None,
        doc_id,
        chunk_vector_ids,
        &[],
        entity_vector_ids,
        relationship_vector_ids,
        nodes_created,
        edges_created,
        cause,
    )
    .await;
}

/// Same as [`compensate_merge_failure`] plus optional KV chunk rollback + DLQ.
#[allow(clippy::too_many_arguments)]
pub async fn compensate_merge_failure_with_kv(
    graph_storage: &dyn GraphStorage,
    vector_storage: &dyn VectorStorage,
    kv_storage: Option<&dyn KVStorage>,
    doc_id: &str,
    chunk_vector_ids: &[String],
    chunk_kv_ids: &[String],
    entity_vector_ids: &[String],
    relationship_vector_ids: &[String],
    nodes_created: &[String],
    edges_created: &[(String, String)],
    cause: &str,
) {
    if let Some(kv) = kv_storage {
        compensate_orphan_kv(kv, doc_id, chunk_kv_ids, cause).await;
    }

    compensate_orphan_vectors_with_kv(
        vector_storage,
        kv_storage,
        doc_id,
        chunk_vector_ids,
        entity_vector_ids,
        cause,
    )
    .await;

    if !relationship_vector_ids.is_empty() {
        compensate_orphan_vectors_with_kv(
            vector_storage,
            kv_storage,
            doc_id,
            &[],
            relationship_vector_ids,
            cause,
        )
        .await;
    }

    compensate_orphan_graph_writes_with_kv(
        graph_storage,
        kv_storage,
        doc_id,
        nodes_created,
        edges_created,
        cause,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::{MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};
    use crate::error::StorageError;
    use crate::traits::{GraphStorageMutateOps, GraphStorageReadOps, KVStorage, VectorStorage};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    /// Vector storage that fails the first `delete` call (inject-fail for DLQ).
    struct FailOnceDeleteVector {
        inner: MemoryVectorStorage,
        failed_once: AtomicBool,
    }

    #[async_trait]
    impl VectorStorage for FailOnceDeleteVector {
        fn namespace(&self) -> &str {
            self.inner.namespace()
        }

        fn dimension(&self) -> usize {
            self.inner.dimension()
        }

        async fn initialize(&self) -> crate::error::Result<()> {
            self.inner.initialize().await
        }

        async fn finalize(&self) -> crate::error::Result<()> {
            self.inner.finalize().await
        }

        async fn query(
            &self,
            query_embedding: &[f32],
            top_k: usize,
            filter_ids: Option<&[String]>,
        ) -> crate::error::Result<Vec<crate::traits::VectorSearchResult>> {
            self.inner.query(query_embedding, top_k, filter_ids).await
        }

        async fn upsert(
            &self,
            data: &[(String, Vec<f32>, serde_json::Value)],
        ) -> crate::error::Result<()> {
            self.inner.upsert(data).await
        }

        async fn delete(&self, ids: &[String]) -> crate::error::Result<()> {
            if !self.failed_once.swap(true, AtomicOrdering::SeqCst) {
                return Err(StorageError::Database(
                    "injected delete failure (SPEC-057 P3)".into(),
                ));
            }
            self.inner.delete(ids).await
        }

        async fn delete_entity(&self, entity_name: &str) -> crate::error::Result<()> {
            self.inner.delete_entity(entity_name).await
        }

        async fn delete_entity_relations(&self, entity_name: &str) -> crate::error::Result<()> {
            self.inner.delete_entity_relations(entity_name).await
        }

        async fn get_by_id(&self, id: &str) -> crate::error::Result<Option<Vec<f32>>> {
            self.inner.get_by_id(id).await
        }

        async fn get_by_ids(
            &self,
            ids: &[String],
        ) -> crate::error::Result<Vec<(String, Vec<f32>)>> {
            self.inner.get_by_ids(ids).await
        }

        async fn is_empty(&self) -> crate::error::Result<bool> {
            self.inner.is_empty().await
        }

        async fn count(&self) -> crate::error::Result<usize> {
            self.inner.count().await
        }

        async fn clear(&self) -> crate::error::Result<()> {
            self.inner.clear().await
        }

        async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> crate::error::Result<usize> {
            self.inner.clear_workspace(workspace_id).await
        }

        async fn delete_by_document(&self, document_id: &str) -> crate::error::Result<usize> {
            self.inner.delete_by_document(document_id).await
        }

        async fn query_filtered(
            &self,
            query_embedding: &[f32],
            top_k: usize,
            filter_ids: Option<&[String]>,
            metadata_filter: Option<&crate::traits::MetadataFilter>,
        ) -> crate::error::Result<Vec<crate::traits::VectorSearchResult>> {
            self.inner
                .query_filtered(query_embedding, top_k, filter_ids, metadata_filter)
                .await
        }
    }

    #[tokio::test]
    async fn compensate_merge_failure_rolls_back_new_graph_and_vectors() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vector = MemoryVectorStorage::new("test", 4);
        vector.initialize().await.unwrap();

        vector
            .upsert(&[(
                "doc1-chunk-0".to_string(),
                vec![0.1; 4],
                serde_json::json!({}),
            )])
            .await
            .unwrap();
        vector
            .upsert(&[(
                "entity:NEW_NODE".to_string(),
                vec![0.2; 4],
                serde_json::json!({}),
            )])
            .await
            .unwrap();

        graph
            .upsert_node(
                "NEW_NODE",
                std::collections::HashMap::from([("label".to_string(), serde_json::json!("New"))]),
            )
            .await
            .unwrap();

        super::compensate_merge_failure(
            &graph,
            &vector,
            "doc1",
            &["doc1-chunk-0".to_string()],
            &["entity:NEW_NODE".to_string()],
            &[],
            &["NEW_NODE".to_string()],
            &[],
            "merge failed (test)",
        )
        .await;

        assert!(vector.get_by_id("doc1-chunk-0").await.unwrap().is_none());
        assert!(vector.get_by_id("entity:NEW_NODE").await.unwrap().is_none());
        assert!(graph.get_node("NEW_NODE").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn compensate_twice_is_idempotent() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vector = MemoryVectorStorage::new("test", 4);
        vector.initialize().await.unwrap();

        vector
            .upsert(&[(
                "doc1-chunk-0".to_string(),
                vec![0.1; 4],
                serde_json::json!({}),
            )])
            .await
            .unwrap();
        graph
            .upsert_node(
                "NODE_A",
                std::collections::HashMap::from([("label".to_string(), serde_json::json!("A"))]),
            )
            .await
            .unwrap();

        let chunk_ids = ["doc1-chunk-0".to_string()];
        let nodes = ["NODE_A".to_string()];
        for _ in 0..2 {
            super::compensate_merge_failure(
                &graph,
                &vector,
                "doc1",
                &chunk_ids,
                &[],
                &[],
                &nodes,
                &[],
                "double compensate",
            )
            .await;
        }

        assert!(vector.get_by_id("doc1-chunk-0").await.unwrap().is_none());
        assert!(graph.get_node("NODE_A").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn compensate_deletes_orphan_chunk_and_entity_vectors() {
        let storage = MemoryVectorStorage::new("test", 4);
        storage.initialize().await.unwrap();
        storage
            .upsert(&[
                (
                    "doc1-chunk-0".to_string(),
                    vec![0.1, 0.2, 0.3, 0.4],
                    serde_json::json!({}),
                ),
                (
                    "entity:FOO".to_string(),
                    vec![0.5, 0.6, 0.7, 0.8],
                    serde_json::json!({}),
                ),
            ])
            .await
            .unwrap();

        super::compensate_orphan_vectors(
            &storage,
            "doc1",
            &["doc1-chunk-0".to_string()],
            &["entity:FOO".to_string()],
            "graph merge failure (test)",
        )
        .await;

        assert!(storage.get_by_id("doc1-chunk-0").await.unwrap().is_none());
        assert!(storage.get_by_id("entity:FOO").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn compensate_noop_on_empty() {
        let storage = MemoryVectorStorage::new("test", 4);
        storage.initialize().await.unwrap();
        super::compensate_orphan_vectors(&storage, "doc1", &[], &[], "noop").await;
    }

    #[tokio::test]
    async fn compensate_orphan_kv_deletes_chunk_keys() {
        let kv = MemoryKVStorage::new("test");
        kv.initialize().await.unwrap();
        kv.upsert(&[(
            "doc1-chunk-0".to_string(),
            serde_json::json!({"content": "hello"}),
        )])
        .await
        .unwrap();

        super::compensate_orphan_kv(&kv, "doc1", &["doc1-chunk-0".to_string()], "merge failed")
            .await;

        assert!(kv.get_by_id("doc1-chunk-0").await.unwrap().is_none());
    }

    /// SPEC-058: compensate must not delete a shared entity vector that was
    /// only *updated* (absent from created artifact list).
    #[tokio::test]
    async fn spec058_compensate_preserves_shared_entity_vector() {
        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vector = MemoryVectorStorage::new("test", 4);
        vector.initialize().await.unwrap();

        vector
            .upsert(&[
                (
                    "doc-b-chunk-0".to_string(),
                    vec![0.1; 4],
                    serde_json::json!({"type": "chunk"}),
                ),
                (
                    "entity:SHARED".to_string(),
                    vec![0.9; 4],
                    serde_json::json!({"type": "entity"}),
                ),
            ])
            .await
            .unwrap();

        // Only chunk + *new* node were created for doc B; SHARED entity vector
        // is omitted from the compensate list (pre-existed from doc A).
        super::compensate_merge_failure(
            &graph,
            &vector,
            "doc-b",
            &["doc-b-chunk-0".to_string()],
            &[], // no created entity vectors
            &[],
            &["ONLY_IN_B".to_string()],
            &[],
            "doc-b merge failed",
        )
        .await;

        assert!(vector.get_by_id("doc-b-chunk-0").await.unwrap().is_none());
        assert!(
            vector.get_by_id("entity:SHARED").await.unwrap().is_some(),
            "shared entity embedding must survive compensate"
        );
    }

    #[tokio::test]
    async fn inject_fail_delete_writes_kv_dlq_and_increments_metric() {
        reset_compensation_quarantine_total_for_tests();
        let before = compensation_quarantine_total();

        let graph = MemoryGraphStorage::new("test");
        graph.initialize().await.unwrap();
        let vector = FailOnceDeleteVector {
            inner: MemoryVectorStorage::new("test", 4),
            failed_once: AtomicBool::new(false),
        };
        vector.initialize().await.unwrap();
        vector
            .upsert(&[(
                "doc-inject-chunk-0".to_string(),
                vec![0.1; 4],
                serde_json::json!({}),
            )])
            .await
            .unwrap();

        let kv = MemoryKVStorage::new("test");
        kv.initialize().await.unwrap();

        super::compensate_merge_failure_with_kv(
            &graph,
            &vector,
            Some(&kv),
            "doc-inject",
            &["doc-inject-chunk-0".to_string()],
            &[],
            &[],
            &[],
            &[],
            &[],
            "inject-fail merge",
        )
        .await;

        let after = compensation_quarantine_total();
        assert!(
            after > before,
            "quarantine metric must increment on cleanup failure"
        );

        let prefix = kv_keys::compensation_quarantine_prefix("doc-inject");
        let keys = kv.keys_with_prefix(&prefix).await.unwrap();
        assert!(
            !keys.is_empty(),
            "KV DLQ must contain compensation_quarantine record"
        );
        let record = kv.get_by_id(&keys[0]).await.unwrap().expect("dlq value");
        assert_eq!(record["kind"], "vector");
    }
}
