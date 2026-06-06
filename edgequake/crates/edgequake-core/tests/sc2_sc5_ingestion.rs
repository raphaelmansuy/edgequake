#![cfg(feature = "pipeline")]

//! Cross-store ingestion correctness tests (SPEC-016 datalayer audit).
//!
//! Covers two findings that the unit-level string/parser tests cannot reach
//! because they only emerge from the real `insert()` / `insert_batch()` flow:
//!
//! * **SC2 / F4 — cross-store saga compensation.** Vector and graph stores are
//!   independent backends with no shared transaction. When the graph merge (the
//!   last fallible stage) fails, the chunk vectors written first MUST be rolled
//!   back so no orphaned, unreachable embeddings survive. We force a deterministic
//!   merge failure with a graph double and assert (a) the chunk vectors existed
//!   at merge time (proving vectors-first ordering) and (b) they are gone after
//!   the failed insert (proving compensation ran).
//!
//! * **SC5 — batch ordering + error aggregation.** `insert_batch` must preserve
//!   input order and convert a single document's failure into a
//!   `success: false` result instead of aborting the whole batch.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_core::{EdgeQuake, EdgeQuakeConfig, StorageBackend, StorageConfig};
use edgequake_llm::MockProvider;
use edgequake_storage::traits::{EdgeListFilter, GraphScanOps, NodeListFilter, PagedGraphResult};
use edgequake_storage::{
    GraphEdge, GraphNode, GraphStorage, GraphStorageAnalyticsOps, GraphStorageMutateOps,
    GraphStorageReadOps, KnowledgeGraph, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
    StorageError, VectorStorage,
};

const EMBED_DIM: usize = 1536;

/// Extraction response shared by all successful inserts. Any valid response
/// works, so identical responses make concurrency ordering irrelevant.
const EXTRACTION_JSON: &str = r#"{
  "entities": [
    {"name": "Sarah Chen", "type": "PERSON", "description": "Chief architect"},
    {"name": "EdgeQuake", "type": "SYSTEM", "description": "RAG system in Rust"}
  ],
  "relationships": [
    {"source": "Sarah Chen", "target": "EdgeQuake", "type": "LEADS", "description": "Sarah leads EdgeQuake"}
  ]
}"#;

const SMALL_DOCUMENT: &str =
    "Sarah Chen leads the EdgeQuake project. EdgeQuake is a RAG system written in Rust.";

/// Build a mock provider pre-loaded with `n` identical extraction responses.
async fn mock_with_responses(n: usize) -> Arc<MockProvider> {
    let provider = Arc::new(MockProvider::new());
    for _ in 0..n {
        provider.add_response(EXTRACTION_JSON).await;
    }
    provider
}

/// Count vectors currently stored whose metadata `type` == "chunk".
async fn count_chunk_vectors(store: &dyn VectorStorage) -> usize {
    // A uniform query vector still returns every stored vector (brute-force
    // in-memory search), so a large top_k effectively enumerates the store.
    let results = store
        .query(&vec![0.0_f32; EMBED_DIM], 100_000, None)
        .await
        .expect("vector query should succeed");
    results
        .iter()
        .filter(|r| r.metadata.get("type").and_then(|v| v.as_str()) == Some("chunk"))
        .count()
}

/// Graph storage double that fails on the first node upsert, while recording how
/// many chunk vectors were already present in the shared vector store at that
/// moment. Every other method is a benign no-op because the merge aborts at the
/// first `upsert_node` before any of them are exercised.
struct FailingGraphStorage {
    /// Shared handle to the SAME vector store the orchestrator writes to, so we
    /// can observe ordering: were chunk vectors written before the merge ran?
    vector_store: Arc<dyn VectorStorage>,
    /// Chunk vectors observed at the instant the merge attempted its first write.
    chunk_vectors_at_merge: AtomicUsize,
}

impl FailingGraphStorage {
    fn new(vector_store: Arc<dyn VectorStorage>) -> Self {
        Self {
            vector_store,
            chunk_vectors_at_merge: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl GraphStorage for FailingGraphStorage {
    fn namespace(&self) -> &str {
        "failing-graph"
    }

    async fn initialize(&self) -> Result<(), StorageError> {
        Ok(())
    }

    async fn finalize(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

#[async_trait]
impl GraphStorageReadOps for FailingGraphStorage {
    async fn has_node(&self, _node_id: &str) -> Result<bool, StorageError> {
        Ok(false)
    }

    async fn get_node(&self, _node_id: &str) -> Result<Option<GraphNode>, StorageError> {
        Ok(None)
    }

    async fn node_degree(&self, _node_id: &str) -> Result<usize, StorageError> {
        Ok(0)
    }

    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>, StorageError> {
        Ok(vec![])
    }

    async fn get_nodes_by_ids(&self, _node_ids: &[String]) -> Result<Vec<GraphNode>, StorageError> {
        Ok(vec![])
    }

    async fn has_edge(&self, _source: &str, _target: &str) -> Result<bool, StorageError> {
        Ok(false)
    }

    async fn get_edge(
        &self,
        _source: &str,
        _target: &str,
    ) -> Result<Option<GraphEdge>, StorageError> {
        Ok(None)
    }

    async fn get_node_edges(&self, _node_id: &str) -> Result<Vec<GraphEdge>, StorageError> {
        Ok(vec![])
    }

    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>, StorageError> {
        Ok(vec![])
    }

    async fn get_knowledge_graph(
        &self,
        _start_node: &str,
        _max_depth: usize,
        _max_nodes: usize,
    ) -> Result<KnowledgeGraph, StorageError> {
        Ok(KnowledgeGraph::new())
    }

    async fn get_popular_labels(&self, _limit: usize) -> Result<Vec<String>, StorageError> {
        Ok(vec![])
    }

    async fn search_labels(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<String>, StorageError> {
        Ok(vec![])
    }

    async fn search_nodes(
        &self,
        _query: &str,
        _limit: usize,
        _entity_type: Option<&str>,
        _tenant_id: Option<&str>,
        _workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>, StorageError> {
        Ok(vec![])
    }

    async fn get_neighbors(
        &self,
        _node_id: &str,
        _depth: usize,
    ) -> Result<Vec<GraphNode>, StorageError> {
        Ok(vec![])
    }
}

#[async_trait]
impl GraphStorageMutateOps for FailingGraphStorage {
    async fn upsert_node(
        &self,
        _node_id: &str,
        _properties: HashMap<String, serde_json::Value>,
    ) -> Result<(), StorageError> {
        let seen = count_chunk_vectors(self.vector_store.as_ref()).await;
        self.chunk_vectors_at_merge.store(seen, Ordering::SeqCst);
        Err(StorageError::Transaction(
            "injected graph merge failure".to_string(),
        ))
    }

    async fn delete_node(&self, _node_id: &str) -> Result<(), StorageError> {
        Ok(())
    }

    async fn upsert_edge(
        &self,
        _source: &str,
        _target: &str,
        _properties: HashMap<String, serde_json::Value>,
    ) -> Result<(), StorageError> {
        Ok(())
    }

    async fn delete_edge(&self, _source: &str, _target: &str) -> Result<(), StorageError> {
        Ok(())
    }

    async fn clear(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

#[async_trait]
impl GraphScanOps for FailingGraphStorage {
    async fn list_nodes_filtered(
        &self,
        _filter: &NodeListFilter,
        _offset: usize,
        _limit: usize,
    ) -> Result<PagedGraphResult<GraphNode>, StorageError> {
        Ok(PagedGraphResult::empty(0, 0))
    }

    async fn list_edges_filtered(
        &self,
        _filter: &EdgeListFilter,
        _offset: usize,
        _limit: usize,
    ) -> Result<PagedGraphResult<GraphEdge>, StorageError> {
        Ok(PagedGraphResult::empty(0, 0))
    }

    async fn find_nodes_by_source_prefixes(
        &self,
        _filter: &NodeListFilter,
        _source_prefixes: &[String],
    ) -> Result<Vec<GraphNode>, StorageError> {
        Ok(vec![])
    }

    async fn find_edges_by_source_prefixes(
        &self,
        _filter: &EdgeListFilter,
        _source_prefixes: &[String],
    ) -> Result<Vec<GraphEdge>, StorageError> {
        Ok(vec![])
    }

    async fn find_edge_by_relationship_id(
        &self,
        _filter: &EdgeListFilter,
        _relationship_id: &str,
    ) -> Result<Option<GraphEdge>, StorageError> {
        Ok(None)
    }
}

#[async_trait]
impl GraphStorageAnalyticsOps for FailingGraphStorage {
    async fn node_count(&self) -> Result<usize, StorageError> {
        Ok(0)
    }

    async fn edge_count(&self) -> Result<usize, StorageError> {
        Ok(0)
    }
}

/// SC2: a failed graph merge must roll back the chunk vectors written first,
/// leaving zero orphaned chunk embeddings.
#[tokio::test]
async fn test_merge_failure_compensates_chunk_vectors() {
    let kv = Arc::new(MemoryKVStorage::new("sc2"));
    let vector_store: Arc<dyn VectorStorage> = Arc::new(MemoryVectorStorage::new("sc2", EMBED_DIM));
    let failing_graph = Arc::new(FailingGraphStorage::new(vector_store.clone()));

    let config = EdgeQuakeConfig::new()
        .with_namespace("sc2")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mock = mock_with_responses(4).await;
    let mut eq = EdgeQuake::new(config)
        .with_storage_backends(
            kv,
            vector_store.clone(),
            failing_graph.clone() as Arc<dyn GraphStorage>,
        )
        .with_providers(
            mock.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );
    eq.initialize().await.expect("initialize should succeed");

    let result = eq.insert(SMALL_DOCUMENT, Some("sc2-doc")).await;

    // The insert must surface the merge failure rather than reporting success.
    assert!(
        result.is_err(),
        "insert must fail when the graph merge fails, got: {result:?}"
    );

    // Ordering proof: chunk vectors were already committed when the merge ran.
    let seen_at_merge = failing_graph.chunk_vectors_at_merge.load(Ordering::SeqCst);
    assert!(
        seen_at_merge > 0,
        "chunk vectors must be written BEFORE the graph merge (vectors-first ordering)"
    );

    // Compensation proof: no chunk vectors remain after the failed insert.
    let remaining = count_chunk_vectors(vector_store.as_ref()).await;
    assert_eq!(
        remaining, 0,
        "saga compensation must delete all chunk vectors after a failed merge"
    );
}

/// SC5: `insert_batch` preserves input order and aggregates per-document errors
/// instead of aborting the whole batch on the first failure.
#[tokio::test]
async fn test_insert_batch_preserves_order_and_aggregates_errors() {
    let kv = Arc::new(MemoryKVStorage::new("sc5"));
    let vector_store: Arc<dyn VectorStorage> = Arc::new(MemoryVectorStorage::new("sc5", EMBED_DIM));
    let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("sc5"));

    let config = EdgeQuakeConfig::new()
        .with_namespace("sc5")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mock = mock_with_responses(8).await;
    let mut eq = EdgeQuake::new(config)
        .with_storage_backends(kv, vector_store, graph)
        .with_providers(
            mock.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );
    eq.initialize().await.expect("initialize should succeed");

    // A document over the 10MB hard limit forces a deterministic failure before
    // any provider/storage call, without needing a flaky external error.
    let oversized = "x".repeat(11 * 1024 * 1024);

    let results = eq
        .insert_batch(vec![
            (SMALL_DOCUMENT, Some("ok-1")),
            (oversized.as_str(), Some("too-big")),
            (SMALL_DOCUMENT, Some("ok-2")),
        ])
        .await
        .expect("insert_batch should return aggregated results, not bubble an error");

    // Order preservation: result[i] maps to input[i].
    assert_eq!(results.len(), 3, "one result per input document");
    assert_eq!(results[0].document_id, "ok-1");
    assert_eq!(results[1].document_id, "too-big");
    assert_eq!(results[2].document_id, "ok-2");

    // Error aggregation: the oversized doc fails without taking down its peers.
    assert!(
        results[0].success,
        "first document should ingest successfully"
    );
    assert!(
        !results[1].success,
        "oversized document must be reported as a failure"
    );
    assert!(
        results[1].error.is_some(),
        "failed document must carry an error message"
    );
    assert!(
        results[2].success,
        "document after the failure must still ingest (no early abort)"
    );
}
