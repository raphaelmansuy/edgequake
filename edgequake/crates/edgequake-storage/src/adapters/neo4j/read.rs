//! Scoped Neo4j incident-edge reads and bounded candidate traversal.

use std::collections::HashSet;

use async_trait::async_trait;
use edgequake_storage_contracts::{
    AccessError, AccessResult, CursorPage, EdgeDirection, GraphEdgeKey, GraphNodeKey,
    IncidentEdgesRequest, ScopedGraphRead, VersionedEdge,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::client::{Neo4jClient, QueryData};
use super::mapping::decode_digest;

const MAX_READ_ROWS: u32 = 1_000;

const INCIDENT_EDGES: &str = r#"
MATCH (edge:EdgeRevision)-[:FROM]->(source:EntityKey)
MATCH (edge)-[:TO]->(target:EntityKey)
WHERE edge.tenant_id = $tenant_id
  AND edge.workspace_id = $workspace_id
  AND source.tenant_id = $tenant_id
  AND source.workspace_id = $workspace_id
  AND target.tenant_id = $tenant_id
  AND target.workspace_id = $workspace_id
  AND (source.logical_id IN $node_ids OR target.logical_id IN $node_ids)
  AND ($cursor = "" OR edge.physical_id > $cursor)
RETURN DISTINCT
  edge.physical_id AS physical_id,
  edge.logical_id AS logical_id,
  source.logical_id AS source_id,
  target.logical_id AS target_id,
  edge.relationship_type AS relationship_type,
  edge.direction AS direction,
  edge.revision AS revision,
  edge.digest AS digest
ORDER BY physical_id
LIMIT $limit
"#;

const REVISION_DIGESTS: &str = r#"
UNWIND $physical_ids AS physical_id
OPTIONAL MATCH (entity:EntityRevision {physical_id: physical_id})
WHERE entity.tenant_id = $tenant_id AND entity.workspace_id = $workspace_id
WITH physical_id, entity, NULL AS edge
UNION ALL
UNWIND $physical_ids AS physical_id
OPTIONAL MATCH (edge:EdgeRevision {physical_id: physical_id})
WHERE edge.tenant_id = $tenant_id AND edge.workspace_id = $workspace_id
WITH physical_id, NULL AS entity, edge
WITH physical_id, coalesce(entity.digest, edge.digest) AS digest
WHERE digest IS NOT NULL
RETURN physical_id, digest
ORDER BY physical_id
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionDigest {
    pub physical_id: Uuid,
    pub digest: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct Neo4jTraversalRequest {
    pub scope: edgequake_storage_contracts::AccessScope,
    pub seeds: Vec<GraphNodeKey>,
    pub max_depth: u32,
    pub max_nodes: u32,
    pub max_edges: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neo4jTraversalResult {
    pub node_ids: Vec<GraphNodeKey>,
    pub edges: Vec<VersionedEdge>,
    pub truncated: bool,
}

#[async_trait]
impl ScopedGraphRead for Neo4jClient {
    async fn incident_edges(
        &self,
        request: &IncidentEdgesRequest,
    ) -> AccessResult<CursorPage<VersionedEdge>> {
        validate_incident_request(request)?;
        if request.node_ids.is_empty() || request.limit == 0 {
            return Ok(CursorPage {
                items: Vec::new(),
                next_cursor: None,
            });
        }
        let cursor = request
            .cursor
            .as_deref()
            .map(|value| {
                Uuid::parse_str(value)
                    .map(|id| id.to_string())
                    .map_err(|_| AccessError::InvalidInput("invalid graph cursor".into()))
            })
            .transpose()?
            .unwrap_or_default();
        let fetch_limit = request.limit.saturating_add(1).min(MAX_READ_ROWS + 1);
        let parameters = json!({
            "tenant_id": request.scope.tenant().to_string(),
            "workspace_id": request.scope.workspace().to_string(),
            "node_ids": request.node_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "cursor": cursor,
            "limit": fetch_limit,
        });
        let data = execute_read(self, INCIDENT_EDGES, parameters).await?;
        let mut items = data
            .object_rows()?
            .into_iter()
            .map(|row| versioned_edge(request, &row))
            .collect::<AccessResult<Vec<_>>>()?;
        let has_more = items.len() > request.limit as usize;
        items.truncate(request.limit as usize);
        let next_cursor = has_more
            .then(|| items.last().map(|edge| edge.key.to_string()))
            .flatten();
        Ok(CursorPage { items, next_cursor })
    }
}

impl Neo4jClient {
    pub async fn revision_digests(
        &self,
        scope: &edgequake_storage_contracts::AccessScope,
        physical_ids: &[Uuid],
    ) -> AccessResult<Vec<RevisionDigest>> {
        if physical_ids.is_empty() {
            return Ok(Vec::new());
        }
        if physical_ids.len() > MAX_READ_ROWS as usize {
            return Err(AccessError::InvalidInput(format!(
                "Neo4j digest read exceeds {MAX_READ_ROWS} rows"
            )));
        }
        let data = execute_read(
            self,
            REVISION_DIGESTS,
            json!({
                "tenant_id": scope.tenant().to_string(),
                "workspace_id": scope.workspace().to_string(),
                "physical_ids": physical_ids.iter().map(Uuid::to_string).collect::<Vec<_>>(),
            }),
        )
        .await?;
        data.object_rows()?.iter().map(revision_digest).collect()
    }

    /// Return provider candidates only. Callers must authority-check revisions
    /// and contribution visibility before using them to expand a user result.
    pub async fn traverse_bounded(
        &self,
        request: &Neo4jTraversalRequest,
    ) -> AccessResult<Neo4jTraversalResult> {
        validate_traversal_request(request)?;
        if request.seeds.is_empty() || request.max_depth == 0 {
            return Ok(Neo4jTraversalResult {
                node_ids: request.seeds.clone(),
                edges: Vec::new(),
                truncated: false,
            });
        }
        let mut seen_nodes: HashSet<GraphNodeKey> = request.seeds.iter().copied().collect();
        if seen_nodes.len() > request.max_nodes as usize {
            return Err(AccessError::InvalidInput(
                "graph seed count exceeds max_nodes".into(),
            ));
        }
        let mut nodes = request.seeds.clone();
        let mut frontier = request.seeds.clone();
        let mut seen_edges = HashSet::new();
        let mut edges = Vec::new();

        for _ in 0..request.max_depth {
            if frontier.is_empty() {
                break;
            }
            let remaining = request.max_edges as usize - edges.len();
            if remaining == 0 {
                return Ok(Neo4jTraversalResult {
                    node_ids: nodes,
                    edges,
                    truncated: true,
                });
            }
            let page = self
                .incident_edges(&IncidentEdgesRequest {
                    scope: request.scope,
                    node_ids: frontier,
                    cursor: None,
                    limit: u32::try_from(remaining.min(MAX_READ_ROWS as usize))
                        .unwrap_or(MAX_READ_ROWS),
                })
                .await?;
            let mut next_frontier = Vec::new();
            for edge in page.items {
                if !seen_edges.insert(edge.key) {
                    continue;
                }
                for endpoint in [edge.source, edge.target] {
                    if !seen_nodes.contains(&endpoint) {
                        if seen_nodes.len() == request.max_nodes as usize {
                            return Ok(Neo4jTraversalResult {
                                node_ids: nodes,
                                edges,
                                truncated: true,
                            });
                        }
                        seen_nodes.insert(endpoint);
                        nodes.push(endpoint);
                        next_frontier.push(endpoint);
                    }
                }
                edges.push(edge);
            }
            if page.next_cursor.is_some() {
                return Ok(Neo4jTraversalResult {
                    node_ids: nodes,
                    edges,
                    truncated: true,
                });
            }
            frontier = next_frontier;
        }
        Ok(Neo4jTraversalResult {
            node_ids: nodes,
            edges,
            truncated: !frontier.is_empty(),
        })
    }
}

async fn execute_read(
    client: &Neo4jClient,
    statement: &str,
    parameters: Value,
) -> AccessResult<QueryData> {
    let mut transaction = client.open_transaction().await?;
    match transaction.execute(statement, parameters).await {
        Ok(response) => {
            transaction.commit().await?;
            Ok(response.data)
        }
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

fn validate_incident_request(request: &IncidentEdgesRequest) -> AccessResult<()> {
    if request.limit > MAX_READ_ROWS {
        return Err(AccessError::InvalidInput(format!(
            "incident-edge limit exceeds {MAX_READ_ROWS}"
        )));
    }
    if request.node_ids.len() > MAX_READ_ROWS as usize {
        return Err(AccessError::InvalidInput(format!(
            "incident-edge node batch exceeds {MAX_READ_ROWS}"
        )));
    }
    Ok(())
}

fn validate_traversal_request(request: &Neo4jTraversalRequest) -> AccessResult<()> {
    if request.max_nodes == 0 || request.max_edges == 0 {
        return Err(AccessError::InvalidInput(
            "graph traversal max_nodes and max_edges must be positive".into(),
        ));
    }
    if request.max_nodes > MAX_READ_ROWS || request.max_edges > MAX_READ_ROWS {
        return Err(AccessError::InvalidInput(format!(
            "graph traversal budgets may not exceed {MAX_READ_ROWS}"
        )));
    }
    Ok(())
}

fn versioned_edge(
    request: &IncidentEdgesRequest,
    row: &Map<String, Value>,
) -> AccessResult<VersionedEdge> {
    let direction = match string(row, "direction")? {
        "directed" => EdgeDirection::Directed,
        "undirected" => EdgeDirection::Undirected,
        value => {
            return Err(AccessError::CorruptData(format!(
                "invalid Neo4j edge direction '{value}'"
            )))
        }
    };
    Ok(VersionedEdge {
        scope: request.scope,
        key: GraphEdgeKey::new(uuid(row, "physical_id")?),
        source: GraphNodeKey::new(uuid(row, "source_id")?),
        target: GraphNodeKey::new(uuid(row, "target_id")?),
        relationship_type: string(row, "relationship_type")?.to_owned(),
        direction,
        revision: integer(row, "revision")?,
        digest: decode_digest(string(row, "digest")?)?,
    })
}

fn revision_digest(row: &Map<String, Value>) -> AccessResult<RevisionDigest> {
    Ok(RevisionDigest {
        physical_id: uuid(row, "physical_id")?,
        digest: decode_digest(string(row, "digest")?)?,
    })
}

fn string<'a>(row: &'a Map<String, Value>, field: &str) -> AccessResult<&'a str> {
    row.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| AccessError::CorruptData(format!("Neo4j row omitted string '{field}'")))
}

fn uuid(row: &Map<String, Value>, field: &str) -> AccessResult<Uuid> {
    Uuid::parse_str(string(row, field)?)
        .map_err(|_| AccessError::CorruptData(format!("Neo4j row has invalid UUID '{field}'")))
}

fn integer(row: &Map<String, Value>, field: &str) -> AccessResult<u64> {
    row.get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| AccessError::CorruptData(format!("Neo4j row omitted integer '{field}'")))
}

#[cfg(test)]
mod tests {
    use edgequake_storage_contracts::{AccessScope, TenantId, WorkspaceId};

    use super::*;

    #[test]
    fn incident_query_returns_distinct_logical_edges_for_self_loops() {
        assert!(INCIDENT_EDGES.contains("RETURN DISTINCT"));
        assert!(INCIDENT_EDGES.contains("edge.physical_id"));
        assert!(!INCIDENT_EDGES.contains("elementId"));
    }

    #[test]
    fn traversal_rejects_unbounded_limits() {
        let request = Neo4jTraversalRequest {
            scope: AccessScope::new(
                TenantId::new(Uuid::from_u128(1)),
                WorkspaceId::new(Uuid::from_u128(2)),
            ),
            seeds: vec![],
            max_depth: 1,
            max_nodes: MAX_READ_ROWS + 1,
            max_edges: 1,
        };
        assert!(matches!(
            validate_traversal_request(&request),
            Err(AccessError::InvalidInput(_))
        ));
    }
}
