//! Graph mutation operations (SPEC-017 ISP Phase 2b).

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::Result;

/// Upsert, delete, and clear graph data.
#[async_trait]
pub trait GraphStorageMutateOps: Send + Sync {
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    async fn upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        for (node_id, properties) in nodes {
            self.upsert_node(node_id, properties.clone()).await?;
        }
        Ok(())
    }

    async fn delete_node(&self, node_id: &str) -> Result<()>;

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    async fn upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        for (source, target, properties) in edges {
            self.upsert_edge(source, target, properties.clone()).await?;
        }
        Ok(())
    }

    async fn delete_edge(&self, source: &str, target: &str) -> Result<()>;

    async fn clear(&self) -> Result<()>;

    async fn clear_workspace(&self, workspace_id: &uuid::Uuid) -> Result<(usize, usize)> {
        let _ = workspace_id;
        Ok((0, 0))
    }
}
