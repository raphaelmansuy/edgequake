//! Read-only facade over [`GraphStorage`] (SPEC-017 ISP Phase 2a).
//!
//! Query and analytics paths hold [`GraphReadView`] instead of full storage so
//! mutation APIs are not in scope — ISP without splitting the trait object.

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::Result;
use crate::traits::{GraphEdge, GraphNode, GraphStorage, KnowledgeGraph};

/// Borrowed read-only graph access; no upsert/delete/clear methods.
pub struct GraphReadView<'a> {
    inner: &'a dyn GraphStorage,
}

impl<'a> GraphReadView<'a> {
    pub fn new(inner: &'a dyn GraphStorage) -> Self {
        Self { inner }
    }

    pub fn from_arc(inner: &'a Arc<dyn GraphStorage>) -> Self {
        Self::new(inner.as_ref())
    }

    pub fn namespace(&self) -> &str {
        self.inner.namespace()
    }

    pub async fn has_node(&self, node_id: &str) -> Result<bool> {
        self.inner.has_node(node_id).await
    }

    pub async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
        self.inner.get_node(node_id).await
    }

    pub async fn node_degree(&self, node_id: &str) -> Result<usize> {
        self.inner.node_degree(node_id).await
    }

    pub async fn node_degrees_batch(&self, node_ids: &[String]) -> Result<Vec<(String, usize)>> {
        self.inner.node_degrees_batch(node_ids).await
    }

    #[allow(deprecated)]
    pub async fn get_all_nodes(&self) -> Result<Vec<GraphNode>> {
        self.inner.get_all_nodes().await
    }

    pub async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>> {
        self.inner.get_nodes_by_ids(node_ids).await
    }

    pub async fn get_nodes_batch(&self, node_ids: &[String]) -> Result<HashMap<String, GraphNode>> {
        self.inner.get_nodes_batch(node_ids).await
    }

    pub async fn get_edges_for_nodes_batch(&self, node_ids: &[String]) -> Result<Vec<GraphEdge>> {
        self.inner.get_edges_for_nodes_batch(node_ids).await
    }

    pub async fn get_nodes_with_degrees_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<(GraphNode, usize, usize)>> {
        self.inner.get_nodes_with_degrees_batch(node_ids).await
    }

    pub async fn has_edge(&self, source: &str, target: &str) -> Result<bool> {
        self.inner.has_edge(source, target).await
    }

    pub async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>> {
        self.inner.get_edge(source, target).await
    }

    pub async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        self.inner.get_node_edges(node_id).await
    }

    #[allow(deprecated)]
    pub async fn get_all_edges(&self) -> Result<Vec<GraphEdge>> {
        self.inner.get_all_edges().await
    }

    pub async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph> {
        self.inner
            .get_knowledge_graph(start_node, max_depth, max_nodes)
            .await
    }

    pub async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>> {
        self.inner.get_popular_labels(limit).await
    }

    pub async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        self.inner.search_labels(query, limit).await
    }

    pub async fn search_nodes(
        &self,
        query: &str,
        limit: usize,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        self.inner
            .search_nodes(query, limit, entity_type, tenant_id, workspace_id)
            .await
    }

    pub async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>> {
        self.inner.get_neighbors(node_id, depth).await
    }

    pub async fn get_popular_nodes_with_degree(
        &self,
        limit: usize,
        min_degree: Option<usize>,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        self.inner
            .get_popular_nodes_with_degree(limit, min_degree, entity_type, tenant_id, workspace_id)
            .await
    }

    pub async fn get_edges_for_node_set(
        &self,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        self.inner
            .get_edges_for_node_set(node_ids, tenant_id, workspace_id)
            .await
    }

    pub async fn node_count(&self) -> Result<usize> {
        self.inner.node_count().await
    }

    pub async fn edge_count(&self) -> Result<usize> {
        self.inner.edge_count().await
    }

    pub async fn node_count_fast(&self) -> Result<usize> {
        self.inner.node_count_fast().await
    }

    pub async fn edge_count_fast(&self) -> Result<usize> {
        self.inner.edge_count_fast().await
    }

    pub async fn ping(&self) -> Result<()> {
        self.inner.ping().await
    }

    pub async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        self.inner.node_count_by_workspace(workspace_id).await
    }

    pub async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        self.inner.edge_count_by_workspace(workspace_id).await
    }

    pub async fn distinct_node_type_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        self.inner
            .distinct_node_type_count_by_workspace(workspace_id)
            .await
    }

    /// SPEC-006: bounded node listing (push-down pagination).
    pub async fn list_nodes_filtered(
        &self,
        filter: &super::graph_scan_ops::NodeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<super::graph_scan_ops::PagedGraphResult<super::graph::GraphNode>> {
        self.inner.list_nodes_filtered(filter, offset, limit).await
    }

    /// SPEC-006: bounded edge listing (push-down pagination).
    pub async fn list_edges_filtered(
        &self,
        filter: &super::graph_scan_ops::EdgeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<super::graph_scan_ops::PagedGraphResult<super::graph::GraphEdge>> {
        self.inner.list_edges_filtered(filter, offset, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::memory::MemoryGraphStorage;
    use crate::traits::{GraphStorage, GraphStorageMutateOps};

    #[tokio::test]
    async fn read_view_delegates_to_storage() {
        let storage = MemoryGraphStorage::new("read-view-test");
        storage.initialize().await.unwrap();
        storage.upsert_node("N", Default::default()).await.unwrap();

        let view = GraphReadView::new(&storage);
        assert!(view.has_node("N").await.unwrap());
        assert_eq!(view.node_count().await.unwrap(), 1);
    }
}
