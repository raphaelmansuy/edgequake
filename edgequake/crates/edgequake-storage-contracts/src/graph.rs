//! Scoped, bounded graph read contracts.

use crate::error::AccessResult;
use crate::ids::{GraphEdgeKey, GraphNodeKey};
use crate::relational::{CursorPage, Digest};
use crate::scope::AccessScope;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeDirection {
    Directed,
    Undirected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncidentEdgesRequest {
    pub scope: AccessScope,
    pub node_ids: Vec<GraphNodeKey>,
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionedEdge {
    pub scope: AccessScope,
    pub key: GraphEdgeKey,
    pub source: GraphNodeKey,
    pub target: GraphNodeKey,
    pub relationship_type: String,
    pub direction: EdgeDirection,
    pub revision: u64,
    pub digest: Digest,
}

#[async_trait]
pub trait ScopedGraphRead: Send + Sync {
    async fn incident_edges(
        &self,
        request: &IncidentEdgesRequest,
    ) -> AccessResult<CursorPage<VersionedEdge>>;
}
