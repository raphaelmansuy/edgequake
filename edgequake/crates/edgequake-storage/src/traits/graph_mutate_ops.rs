//! Graph mutation operations (SPEC-017 ISP Phase 2b).

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::Result;

/// How conflict updates apply property maps on native AGE upsert.
///
/// - [`MergeSources`](Self::MergeSources): ingest-safe — `eq_merge_graph_properties`
///   unions `source_ids` / `source_chunk_ids` (SPEC-058).
/// - [`Replace`](Self::Replace): cascade prune — set `properties = EXCLUDED.properties`
///   so subtractive `source_ids` writes stick (SPEC-098 / LAW-098-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphPropertyWriteMode {
    /// Concurrent ingest: union source lineage arrays.
    #[default]
    MergeSources,
    /// Document cascade shared-entity prune: full property replace.
    Replace,
}

/// Upsert, delete, and clear graph data.
///
/// # Batch contract (P-G10 / RC-15, LSP)
///
/// `upsert_nodes_batch` and `upsert_edges_batch` are **required** (no default
/// impl). They MUST persist all items in a single storage round-trip (or one
/// logical transaction) — not loop over the per-item methods. This closes the
/// LSP trap where the memory adapter inherited an O(N) default and silently
/// made every "batch" call N round-trips. Callers can now rely on batch
/// performance semantics regardless of backend.
#[async_trait]
pub trait GraphStorageMutateOps: Send + Sync {
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    /// Batch upsert all nodes in one storage operation (required; no default).
    ///
    /// Equivalent to [`upsert_nodes_batch_with_mode`](Self::upsert_nodes_batch_with_mode)
    /// with [`GraphPropertyWriteMode::MergeSources`].
    async fn upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()>;

    /// Batch upsert with explicit property write mode (SPEC-098 cascade prune).
    ///
    async fn upsert_nodes_batch_with_mode(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
        mode: GraphPropertyWriteMode,
    ) -> Result<()>;

    async fn delete_node(&self, node_id: &str) -> Result<()>;

    /// Batch-delete nodes (and incident edges). Default loops `delete_node`;
    /// Postgres native path is O(K log N) one round-trip (SPEC-060 compensate).
    async fn delete_nodes_batch(&self, node_ids: &[String]) -> Result<()> {
        for id in node_ids {
            self.delete_node(id).await?;
        }
        Ok(())
    }

    /// Delete a node only when its stored tenant/workspace match (defense in depth).
    ///
    /// Returns `Ok(true)` when a node was deleted, `Ok(false)` when no matching node
    /// exists (including cross-tenant IDOR attempts — caller should map to 404).
    async fn delete_node_scoped(
        &self,
        node_id: &str,
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<bool>;

    /// Batch scoped node delete: one storage round-trip with tenant/workspace fence
    /// plus `id = ANY`. Unscoped [`delete_nodes_batch`] stays unused on this path.
    async fn delete_nodes_scoped_batch(
        &self,
        node_ids: &[String],
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<usize>;

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    /// Batch upsert all edges in one storage operation (required; no default).
    ///
    /// Equivalent to [`upsert_edges_batch_with_mode`](Self::upsert_edges_batch_with_mode)
    /// with [`GraphPropertyWriteMode::MergeSources`].
    async fn upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()>;

    /// Batch edge upsert with explicit property write mode (SPEC-098 cascade prune).
    async fn upsert_edges_batch_with_mode(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
        mode: GraphPropertyWriteMode,
    ) -> Result<()>;

    async fn delete_edge(&self, source: &str, target: &str) -> Result<()>;

    /// Batch-delete edges by `(source, target, rel_type)` triples (SPEC-098 D-30).
    ///
    /// `rel_type` must be normalized (see [`crate::normalize_rel_type`]). Cascade
    /// exclusive prune deletes one multigraph sister at a time — never all rels
    /// between endpoints.
    async fn delete_edges_batch(&self, edges: &[(String, String, String)]) -> Result<()>;

    /// Delete an edge only when tenant/workspace properties match.
    async fn delete_edge_scoped(
        &self,
        source: &str,
        target: &str,
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<bool>;

    /// Batch scoped edge delete: one storage round-trip with tenant/workspace fence
    /// plus endpoint pairs via `ANY`.
    async fn delete_edges_scoped_batch(
        &self,
        edges: &[(String, String)],
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<usize>;

    async fn clear(&self) -> Result<()>;

    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)>;
}
