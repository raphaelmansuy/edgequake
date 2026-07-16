//! Graph analytics and statistics operations (SPEC-017 ISP Phase 2b).

use std::collections::HashMap;

use async_trait::async_trait;

use crate::error::Result;

use super::graph_read_ops::GraphStorageReadOps;
use super::graph_scan_ops::GraphScanOps;

/// Counts, estimates, and workspace-scoped statistics.
///
/// # Workspace scoping (P-G12 / RC-17, LSP)
///
/// `node_count_by_workspace` and `edge_count_by_workspace` are **required**
/// (no default). The previous default ignored `workspace_id` and returned the
/// GLOBAL count, which silently leaked cross-workspace counts to any adapter
/// that forgot to override it. Making them required forces every adapter to
/// implement honest workspace scoping (E32: a workspace with zero nodes must
/// return 0, not the global count).
#[async_trait]
pub trait GraphStorageAnalyticsOps: GraphStorageReadOps + GraphScanOps {
    async fn node_count(&self) -> Result<usize>;

    async fn edge_count(&self) -> Result<usize>;

    async fn node_count_fast(&self) -> Result<usize> {
        self.node_count().await
    }

    async fn edge_count_fast(&self) -> Result<usize> {
        self.edge_count().await
    }

    async fn ping(&self) -> Result<()> {
        let _ = self.node_count().await?;
        Ok(())
    }

    /// Count nodes scoped to a single workspace (required; no default).
    async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize>;

    /// Count edges scoped to a single workspace (required; no default).
    async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize>;

    async fn distinct_node_type_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        let _ = workspace_id;
        Err(crate::error::StorageError::InvalidQuery(
            "distinct_node_type_count_by_workspace requires adapter-specific implementation \
             (SPEC-006: no get_all_nodes fallback)"
                .into(),
        ))
    }

    /// Count nodes whose `source_ids` (or legacy `source_id`) contains any
    /// entry starting with `prefix` (SPEC-021 P-A3).
    ///
    /// WHY: the per-document `entity_count` cell in the Documents list must
    /// fall back to the authoritative AGE graph when both the KV metadata and
    /// the relational `documents.entity_count` column are missing/stale. The
    /// chunk-id prefix `{doc_id}-chunk-` uniquely identifies a document's
    /// entities without a full graph scan.
    ///
    /// Default: scans via `find_nodes_by_source_prefixes` and counts —
    /// adapters SHOULD override with a single aggregate Cypher/SQL for O(log N)
    /// instead of materializing the nodes.
    async fn node_count_by_source_prefix(&self, prefix: &str) -> Result<usize> {
        let map = self
            .node_counts_by_source_prefixes(&[prefix.to_string()])
            .await?;
        Ok(map.get(prefix).copied().unwrap_or(0))
    }

    /// Batch variant of [`node_count_by_source_prefix`] — **one** storage
    /// round-trip for D document prefixes (SPEC-054 L1-a).
    ///
    /// Keys in the returned map match the input prefix strings exactly.
    /// Missing keys mean count 0.
    ///
    /// Default: one `find_nodes_by_source_prefixes` scan + in-memory bucketing.
    /// Postgres AGE overrides with a single GIN `@>` SQL query (no materialize).
    async fn node_counts_by_source_prefixes(
        &self,
        prefixes: &[String],
    ) -> Result<HashMap<String, usize>> {
        use super::graph_scan_ops::{collect_source_references, NodeListFilter};
        let mut out = HashMap::with_capacity(prefixes.len());
        if prefixes.is_empty() {
            return Ok(out);
        }
        let filter = NodeListFilter::default();
        let nodes = self
            .find_nodes_by_source_prefixes(&filter, prefixes)
            .await?;
        for prefix in prefixes {
            let count = nodes
                .iter()
                .filter(|n| {
                    collect_source_references(&n.properties)
                        .iter()
                        .any(|s| s.starts_with(prefix.as_str()))
                })
                .count();
            out.insert(prefix.clone(), count);
        }
        Ok(out)
    }
}
