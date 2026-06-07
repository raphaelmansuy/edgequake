use std::collections::HashMap;

use async_trait::async_trait;

use super::PostgresAGEGraphStorage;
use crate::error::Result;
use crate::traits::{
    EdgeListFilter, GraphEdge, GraphNode, GraphScanOps, GraphStorage, GraphStorageAnalyticsOps,
    GraphStorageMutateOps, GraphStorageReadOps, KnowledgeGraph, NodeListFilter, PagedGraphResult,
};

#[async_trait]
impl GraphStorage for PostgresAGEGraphStorage {
    fn namespace(&self) -> &str {
        self.pg_namespace()
    }

    async fn initialize(&self) -> Result<()> {
        self.pg_initialize().await
    }

    async fn finalize(&self) -> Result<()> {
        self.pg_finalize().await
    }
}

#[async_trait]
#[allow(deprecated)]
impl GraphStorageReadOps for PostgresAGEGraphStorage {
    async fn has_node(&self, node_id: &str) -> Result<bool> {
        self.pg_has_node(node_id).await
    }

    async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
        self.pg_get_node(node_id).await
    }

    async fn node_degree(&self, node_id: &str) -> Result<usize> {
        self.pg_node_degree(node_id).await
    }

    async fn node_degrees_batch(&self, node_ids: &[String]) -> Result<Vec<(String, usize)>> {
        self.pg_node_degrees_batch(node_ids).await
    }

    async fn get_all_nodes(&self) -> Result<Vec<GraphNode>> {
        self.pg_get_all_nodes().await
    }

    async fn get_nodes_by_ids(&self, node_ids: &[String]) -> Result<Vec<GraphNode>> {
        self.pg_get_nodes_by_ids(node_ids).await
    }

    async fn get_nodes_batch(&self, node_ids: &[String]) -> Result<HashMap<String, GraphNode>> {
        self.pg_get_nodes_batch(node_ids).await
    }

    async fn get_edges_for_nodes_batch(&self, node_ids: &[String]) -> Result<Vec<GraphEdge>> {
        self.pg_get_edges_for_nodes_batch(node_ids).await
    }

    async fn get_nodes_with_degrees_batch(
        &self,
        node_ids: &[String],
    ) -> Result<Vec<(GraphNode, usize, usize)>> {
        self.pg_get_nodes_with_degrees_batch(node_ids).await
    }

    async fn has_edge(&self, source: &str, target: &str) -> Result<bool> {
        self.pg_has_edge(source, target).await
    }

    async fn get_edge(&self, source: &str, target: &str) -> Result<Option<GraphEdge>> {
        self.pg_get_edge(source, target).await
    }

    async fn get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        self.pg_get_node_edges(node_id).await
    }

    async fn get_all_edges(&self) -> Result<Vec<GraphEdge>> {
        self.pg_get_all_edges().await
    }

    async fn get_knowledge_graph(
        &self,
        start_node: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<KnowledgeGraph> {
        self.pg_get_knowledge_graph(start_node, max_depth, max_nodes)
            .await
    }

    async fn get_popular_labels(&self, limit: usize) -> Result<Vec<String>> {
        self.pg_get_popular_labels(limit).await
    }

    async fn search_labels(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        self.pg_search_labels(query, limit).await
    }

    async fn search_nodes(
        &self,
        query: &str,
        limit: usize,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        self.pg_search_nodes(query, limit, entity_type, tenant_id, workspace_id)
            .await
    }

    async fn get_neighbors(&self, node_id: &str, depth: usize) -> Result<Vec<GraphNode>> {
        self.pg_get_neighbors(node_id, depth).await
    }

    async fn get_popular_nodes_with_degree(
        &self,
        limit: usize,
        min_degree: Option<usize>,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        self.pg_get_popular_nodes_with_degree(
            limit,
            min_degree,
            entity_type,
            tenant_id,
            workspace_id,
        )
        .await
    }

    async fn get_edges_for_node_set(
        &self,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        self.pg_get_edges_for_node_set(node_ids, tenant_id, workspace_id)
            .await
    }
}

#[async_trait]
impl GraphStorageMutateOps for PostgresAGEGraphStorage {
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        self.pg_upsert_node(node_id, properties).await
    }

    async fn upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        self.pg_upsert_nodes_batch(nodes).await
    }

    async fn delete_node(&self, node_id: &str) -> Result<()> {
        self.pg_delete_node(node_id).await
    }

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        self.pg_upsert_edge(source, target, properties).await
    }

    async fn upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        self.pg_upsert_edges_batch(edges).await
    }

    async fn delete_edge(&self, source: &str, target: &str) -> Result<()> {
        self.pg_delete_edge(source, target).await
    }

    async fn clear(&self) -> Result<()> {
        self.pg_clear().await
    }

    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)> {
        self.pg_clear_workspace(workspace_id).await
    }
}

#[async_trait]
impl GraphStorageAnalyticsOps for PostgresAGEGraphStorage {
    async fn node_count(&self) -> Result<usize> {
        self.pg_node_count().await
    }

    async fn edge_count(&self) -> Result<usize> {
        self.pg_edge_count().await
    }

    async fn node_count_fast(&self) -> Result<usize> {
        self.pg_node_count_fast().await
    }

    async fn edge_count_fast(&self) -> Result<usize> {
        self.pg_edge_count_fast().await
    }

    async fn node_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        self.pg_node_count_by_workspace(workspace_id).await
    }

    async fn ping(&self) -> Result<()> {
        self.pg_ping().await
    }

    async fn edge_count_by_workspace(&self, workspace_id: &uuid::Uuid) -> Result<usize> {
        self.pg_edge_count_by_workspace(workspace_id).await
    }

    async fn distinct_node_type_count_by_workspace(
        &self,
        workspace_id: &uuid::Uuid,
    ) -> Result<usize> {
        self.pg_distinct_node_type_count_by_workspace(workspace_id)
            .await
    }
}

#[async_trait]
impl GraphScanOps for PostgresAGEGraphStorage {
    async fn list_nodes_filtered(
        &self,
        filter: &NodeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphNode>> {
        self.pg_list_nodes_filtered(filter, offset, limit).await
    }

    async fn list_edges_filtered(
        &self,
        filter: &EdgeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphEdge>> {
        self.pg_list_edges_filtered(filter, offset, limit).await
    }

    async fn find_nodes_by_source_prefixes(
        &self,
        filter: &NodeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphNode>> {
        self.pg_find_nodes_by_source_prefixes(filter, source_prefixes)
            .await
    }

    async fn find_edges_by_source_prefixes(
        &self,
        filter: &EdgeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphEdge>> {
        self.pg_find_edges_by_source_prefixes(filter, source_prefixes)
            .await
    }

    async fn find_edge_by_relationship_id(
        &self,
        filter: &EdgeListFilter,
        relationship_id: &str,
    ) -> Result<Option<GraphEdge>> {
        self.pg_find_edge_by_relationship_id(filter, relationship_id)
            .await
    }
}
