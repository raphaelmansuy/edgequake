//! Cross-store saga compensation helpers (SPEC-021 P-C1).
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
//! - **Observable**: on cleanup failure, emits a structured `quarantine` log
//!   so an operator or reconciliation job can remove residue out of band.

use crate::traits::{GraphStorage, VectorStorage};

/// Roll back chunk vectors (and optionally entity vectors) written earlier in
/// the ingestion saga after the graph merge failed.
///
/// `chunk_vector_ids` are the exact IDs written in Stage 2 (chunk embeddings).
/// `entity_vector_ids` are the `entity:{name}` IDs written in Stage 3 (entity
/// embeddings) — pass `&[]` when compensating from the orchestrator path which
/// does not write entity vectors in the same scope.
///
/// Deletion is best-effort and idempotent. Failures are logged as `quarantine`
/// events; the original ingestion error must be surfaced separately by the
/// caller.
pub async fn compensate_orphan_vectors(
    vector_storage: &dyn VectorStorage,
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
            tracing::error!(
                document_id = %doc_id,
                orphan_chunk_vectors = chunk_n,
                orphan_entity_vectors = entity_n,
                merge_cause = %cause,
                cleanup_error = %cleanup_err,
                "quarantine: failed to roll back orphan vectors after graph failure; \
                 manual or reconciliation cleanup required"
            );
        }
    }
}

/// Roll back graph nodes and edges created during a failed merge attempt (P-G5).
///
/// Best-effort and idempotent — only deletes IDs recorded as newly created in the
/// current ingest session (never touches pre-existing merged nodes).
pub async fn compensate_orphan_graph_writes(
    graph_storage: &dyn GraphStorage,
    doc_id: &str,
    nodes_created: &[String],
    edges_created: &[(String, String)],
    cause: &str,
) {
    for (source, target) in edges_created {
        if let Err(e) = graph_storage.delete_edge(source, target).await {
            tracing::error!(
                document_id = %doc_id,
                source = %source,
                target = %target,
                merge_cause = %cause,
                cleanup_error = %e,
                "quarantine: failed to roll back orphan edge after merge failure"
            );
        }
    }

    for node_id in nodes_created {
        if let Err(e) = graph_storage.delete_node(node_id).await {
            tracing::error!(
                document_id = %doc_id,
                node_id = %node_id,
                merge_cause = %cause,
                cleanup_error = %e,
                "quarantine: failed to roll back orphan node after merge failure"
            );
        }
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
/// vectors, and newly created graph nodes/edges (P-G5 SSOT).
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
    compensate_orphan_vectors(
        vector_storage,
        doc_id,
        chunk_vector_ids,
        entity_vector_ids,
        cause,
    )
    .await;

    if !relationship_vector_ids.is_empty() {
        compensate_orphan_vectors(vector_storage, doc_id, &[], relationship_vector_ids, cause)
            .await;
    }

    compensate_orphan_graph_writes(graph_storage, doc_id, nodes_created, edges_created, cause)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryVectorStorage;

    #[tokio::test]
    async fn compensate_merge_failure_rolls_back_new_graph_and_vectors() {
        use crate::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};
        use crate::traits::{GraphStorageMutateOps, GraphStorageReadOps};

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
    async fn compensate_deletes_orphan_chunk_and_entity_vectors() {
        let storage = MemoryVectorStorage::new("test", 4);
        storage.initialize().await.unwrap();
        // Seed two vectors we claim were orphaned by a graph failure.
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

        // Both orphaned vectors must be gone.
        assert!(storage.get_by_id("doc1-chunk-0").await.unwrap().is_none());
        assert!(storage.get_by_id("entity:FOO").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn compensate_noop_on_empty() {
        let storage = MemoryVectorStorage::new("test", 4);
        storage.initialize().await.unwrap();
        // No IDs → must not panic and must not delete anything.
        super::compensate_orphan_vectors(&storage, "doc1", &[], &[], "noop").await;
    }
}
