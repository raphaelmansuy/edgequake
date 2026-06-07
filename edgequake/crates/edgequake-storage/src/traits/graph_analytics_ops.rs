//! Graph analytics and statistics operations (SPEC-017 ISP Phase 2b).

use async_trait::async_trait;

use crate::error::Result;

use super::graph_read_ops::GraphStorageReadOps;

/// Counts, estimates, and workspace-scoped statistics.
#[async_trait]
pub trait GraphStorageAnalyticsOps: GraphStorageReadOps {
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

    async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let _ = workspace_id;
        self.node_count().await
    }

    async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        let _ = workspace_id;
        self.edge_count().await
    }

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
}
