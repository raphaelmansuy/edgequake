//! Graph mutation operations (SPEC-017 ISP Phase 2b).

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::Result;

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
    async fn upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()>;

    async fn delete_node(&self, node_id: &str) -> Result<()>;

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

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    /// Batch upsert all edges in one storage operation (required; no default).
    async fn upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()>;

    async fn delete_edge(&self, source: &str, target: &str) -> Result<()>;

    /// Delete an edge only when tenant/workspace properties match.
    async fn delete_edge_scoped(
        &self,
        source: &str,
        target: &str,
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<bool>;

    async fn clear(&self) -> Result<()>;

    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)> {
        let _ = workspace_id;
        Ok((0, 0))
    }
}
