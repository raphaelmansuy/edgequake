//! Immutable Neo4j graph revision mutations.

use std::collections::HashMap;

use async_trait::async_trait;
use edgequake_storage_contracts::{AccessError, AccessResult, AccessScope};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{Result as StorageResult, StorageError};
use crate::traits::{GraphPropertyWriteMode, GraphStorageMutateOps};

use super::client::{Neo4jClient, Neo4jTransaction};
use super::mapping::{EdgeRevision, EntityRevision};

const MAX_MUTATION_ROWS: usize = 1_000;

const UPSERT_ENTITIES: &str = r#"
UNWIND $rows AS row
MERGE (anchor:EntityKey {
  tenant_id: row.tenant_id,
  workspace_id: row.workspace_id,
  logical_id: row.logical_id
})
MERGE (revision:EntityRevision {physical_id: row.physical_id})
ON CREATE SET
  revision.tenant_id = row.tenant_id,
  revision.workspace_id = row.workspace_id,
  revision.logical_id = row.logical_id,
  revision.revision = row.revision,
  revision.digest = row.digest,
  revision.contribution_manifest = row.contribution_manifest,
  revision.properties_json = row.properties_json
WITH row, anchor, revision,
  revision.tenant_id = row.tenant_id AND
  revision.workspace_id = row.workspace_id AND
  revision.logical_id = row.logical_id AND
  revision.revision = row.revision AND
  revision.digest = row.digest AND
  revision.contribution_manifest = row.contribution_manifest AND
  revision.properties_json = row.properties_json AS equivalent
FOREACH (_ IN CASE WHEN equivalent THEN [1] ELSE [] END |
  MERGE (revision)-[:REVISION_OF]->(anchor)
)
RETURN count(*) AS processed
"#;

const VERIFY_ENTITIES: &str = r#"
UNWIND $rows AS row
OPTIONAL MATCH (revision:EntityRevision {physical_id: row.physical_id})
OPTIONAL MATCH (revision)-[link:REVISION_OF]->(anchor:EntityKey)
WITH row, revision, count(link) AS links, collect(anchor.logical_id) AS anchor_ids
RETURN row.physical_id AS physical_id,
  revision IS NOT NULL AND
  revision.tenant_id = row.tenant_id AND
  revision.workspace_id = row.workspace_id AND
  revision.logical_id = row.logical_id AND
  revision.revision = row.revision AND
  revision.digest = row.digest AND
  revision.contribution_manifest = row.contribution_manifest AND
  revision.properties_json = row.properties_json AND
  links = 1 AND anchor_ids = [row.logical_id] AS equivalent
ORDER BY physical_id
"#;

const UPSERT_EDGES: &str = r#"
UNWIND $rows AS row
MERGE (source:EntityKey {
  tenant_id: row.tenant_id,
  workspace_id: row.workspace_id,
  logical_id: row.source_id
})
MERGE (target:EntityKey {
  tenant_id: row.tenant_id,
  workspace_id: row.workspace_id,
  logical_id: row.target_id
})
MERGE (revision:EdgeRevision {physical_id: row.physical_id})
ON CREATE SET
  revision.tenant_id = row.tenant_id,
  revision.workspace_id = row.workspace_id,
  revision.logical_id = row.logical_id,
  revision.revision = row.revision,
  revision.relationship_type = row.relationship_type,
  revision.direction = row.direction,
  revision.digest = row.digest,
  revision.contribution_manifest = row.contribution_manifest,
  revision.properties_json = row.properties_json
WITH row, source, target, revision,
  revision.tenant_id = row.tenant_id AND
  revision.workspace_id = row.workspace_id AND
  revision.logical_id = row.logical_id AND
  revision.revision = row.revision AND
  revision.relationship_type = row.relationship_type AND
  revision.direction = row.direction AND
  revision.digest = row.digest AND
  revision.contribution_manifest = row.contribution_manifest AND
  revision.properties_json = row.properties_json AS equivalent
FOREACH (_ IN CASE WHEN equivalent THEN [1] ELSE [] END |
  MERGE (revision)-[:FROM]->(source)
  MERGE (revision)-[:TO]->(target)
)
RETURN count(*) AS processed
"#;

const VERIFY_EDGES: &str = r#"
UNWIND $rows AS row
OPTIONAL MATCH (revision:EdgeRevision {physical_id: row.physical_id})
OPTIONAL MATCH (revision)-[from_link:FROM]->(source:EntityKey)
WITH row, revision, count(from_link) AS from_links,
     collect(source.logical_id) AS source_ids
OPTIONAL MATCH (revision)-[to_link:TO]->(target:EntityKey)
WITH row, revision, from_links, source_ids, count(to_link) AS to_links,
     collect(target.logical_id) AS target_ids
RETURN row.physical_id AS physical_id,
  revision IS NOT NULL AND
  revision.tenant_id = row.tenant_id AND
  revision.workspace_id = row.workspace_id AND
  revision.logical_id = row.logical_id AND
  revision.revision = row.revision AND
  revision.relationship_type = row.relationship_type AND
  revision.direction = row.direction AND
  revision.digest = row.digest AND
  revision.contribution_manifest = row.contribution_manifest AND
  revision.properties_json = row.properties_json AND
  from_links = 1 AND source_ids = [row.source_id] AND
  to_links = 1 AND target_ids = [row.target_id] AS equivalent
ORDER BY physical_id
"#;

const DELETE_EDGE_REVISIONS: &str = r#"
UNWIND $rows AS row
OPTIONAL MATCH (revision:EdgeRevision {
  physical_id: row.physical_id,
  tenant_id: row.tenant_id,
  workspace_id: row.workspace_id
})
WITH revision
WHERE revision IS NOT NULL
DETACH DELETE revision
RETURN count(*) AS deleted
"#;

const DELETE_ENTITY_REVISIONS: &str = r#"
UNWIND $rows AS row
OPTIONAL MATCH (revision:EntityRevision {
  physical_id: row.physical_id,
  tenant_id: row.tenant_id,
  workspace_id: row.workspace_id
})
WITH revision
WHERE revision IS NOT NULL
DETACH DELETE revision
RETURN count(*) AS deleted
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neo4jMutationReceipt {
    pub entity_revisions: usize,
    pub edge_revisions: usize,
    pub bookmarks: Vec<String>,
}

impl Neo4jClient {
    /// Apply one bounded immutable graph batch in one Neo4j transaction.
    pub async fn upsert_graph_revisions(
        &self,
        entities: &[EntityRevision],
        edges: &[EdgeRevision],
    ) -> AccessResult<Neo4jMutationReceipt> {
        validate_batch(entities.len(), edges.len())?;
        if entities.is_empty() && edges.is_empty() {
            return Ok(Neo4jMutationReceipt {
                entity_revisions: 0,
                edge_revisions: 0,
                bookmarks: Vec::new(),
            });
        }
        let entity_rows = entities
            .iter()
            .map(EntityRevision::to_row)
            .collect::<AccessResult<Vec<_>>>()?;
        let edge_rows = edges
            .iter()
            .map(EdgeRevision::to_row)
            .collect::<AccessResult<Vec<_>>>()?;
        let mut transaction = self.open_transaction().await?;
        let outcome = apply_revisions(&mut transaction, &entity_rows, &edge_rows).await;
        match outcome {
            Ok(()) => {
                let bookmarks = transaction.commit().await?;
                Ok(Neo4jMutationReceipt {
                    entity_revisions: entities.len(),
                    edge_revisions: edges.len(),
                    bookmarks,
                })
            }
            Err(error) => {
                if let Err(rollback_error) = transaction.rollback().await {
                    return Err(AccessError::UnknownOutcome(format!(
                        "{error}; Neo4j rollback was not confirmed: {rollback_error}"
                    )));
                }
                Err(error)
            }
        }
    }

    pub async fn delete_edge_revisions(
        &self,
        scope: &AccessScope,
        physical_ids: &[Uuid],
    ) -> AccessResult<usize> {
        delete_revisions(self, scope, physical_ids, DELETE_EDGE_REVISIONS).await
    }

    pub async fn delete_entity_revisions(
        &self,
        scope: &AccessScope,
        physical_ids: &[Uuid],
    ) -> AccessResult<usize> {
        delete_revisions(self, scope, physical_ids, DELETE_ENTITY_REVISIONS).await
    }
}

async fn apply_revisions<T: serde::Serialize, U: serde::Serialize>(
    transaction: &mut Neo4jTransaction,
    entities: &[T],
    edges: &[U],
) -> AccessResult<()> {
    if !entities.is_empty() {
        let parameters = json!({ "rows": entities });
        transaction
            .execute(UPSERT_ENTITIES, parameters.clone())
            .await?;
        let verification = transaction.execute(VERIFY_ENTITIES, parameters).await?;
        verify_equivalence(&verification.data.object_rows()?, entities.len(), "entity")?;
    }
    if !edges.is_empty() {
        let parameters = json!({ "rows": edges });
        transaction
            .execute(UPSERT_EDGES, parameters.clone())
            .await?;
        let verification = transaction.execute(VERIFY_EDGES, parameters).await?;
        verify_equivalence(&verification.data.object_rows()?, edges.len(), "edge")?;
    }
    Ok(())
}

fn verify_equivalence(
    rows: &[serde_json::Map<String, Value>],
    expected: usize,
    kind: &str,
) -> AccessResult<()> {
    if rows.len() != expected {
        return Err(AccessError::Conflict(format!(
            "Neo4j {kind} verification returned {} rows for {expected} revisions",
            rows.len()
        )));
    }
    let conflicts: Vec<&str> = rows
        .iter()
        .filter(|row| row.get("equivalent").and_then(Value::as_bool) != Some(true))
        .filter_map(|row| row.get("physical_id").and_then(Value::as_str))
        .collect();
    if conflicts.is_empty() {
        Ok(())
    } else {
        Err(AccessError::Conflict(format!(
            "Neo4j immutable {kind} revision conflict: {}",
            conflicts.join(", ")
        )))
    }
}

async fn delete_revisions(
    client: &Neo4jClient,
    scope: &AccessScope,
    physical_ids: &[Uuid],
    statement: &str,
) -> AccessResult<usize> {
    if physical_ids.is_empty() {
        return Ok(0);
    }
    if physical_ids.len() > MAX_MUTATION_ROWS {
        return Err(AccessError::InvalidInput(format!(
            "Neo4j delete batch exceeds {MAX_MUTATION_ROWS} rows"
        )));
    }
    let rows: Vec<Value> = physical_ids
        .iter()
        .map(|physical_id| {
            json!({
                "tenant_id": scope.tenant().to_string(),
                "workspace_id": scope.workspace().to_string(),
                "physical_id": physical_id.to_string(),
            })
        })
        .collect();
    let mut transaction = client.open_transaction().await?;
    let outcome = transaction
        .execute(statement, json!({ "rows": rows }))
        .await
        .and_then(|response| scalar_usize(&response.data, "deleted"));
    match outcome {
        Ok(deleted) => {
            transaction.commit().await?;
            Ok(deleted)
        }
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

fn scalar_usize(data: &super::client::QueryData, field: &str) -> AccessResult<usize> {
    let row = data
        .object_rows()?
        .into_iter()
        .next()
        .ok_or_else(|| AccessError::CorruptData(format!("Neo4j omitted '{field}' result")))?;
    let value = row
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| AccessError::CorruptData(format!("Neo4j '{field}' is not an integer")))?;
    usize::try_from(value)
        .map_err(|_| AccessError::CorruptData(format!("Neo4j '{field}' exceeds usize")))
}

fn validate_batch(entities: usize, edges: usize) -> AccessResult<()> {
    let total = entities
        .checked_add(edges)
        .ok_or_else(|| AccessError::InvalidInput("graph batch size overflow".into()))?;
    if total > MAX_MUTATION_ROWS {
        return Err(AccessError::InvalidInput(format!(
            "Neo4j graph batch contains {total} rows; maximum is {MAX_MUTATION_ROWS}"
        )));
    }
    Ok(())
}

#[async_trait]
impl GraphStorageMutateOps for Neo4jClient {
    async fn upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, Value>,
    ) -> StorageResult<()> {
        self.upsert_nodes_batch(&[(node_id.to_owned(), properties)])
            .await
    }

    async fn upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, Value>)],
    ) -> StorageResult<()> {
        let entities = nodes
            .iter()
            .map(|(id, properties)| EntityRevision::from_legacy(id, properties.clone()))
            .collect::<AccessResult<Vec<_>>>()
            .map_err(StorageError::from)?;
        self.upsert_graph_revisions(&entities, &[])
            .await
            .map(|_| ())
            .map_err(StorageError::from)
    }

    async fn upsert_nodes_batch_with_mode(
        &self,
        nodes: &[(String, HashMap<String, Value>)],
        _mode: GraphPropertyWriteMode,
    ) -> StorageResult<()> {
        self.upsert_nodes_batch(nodes).await
    }

    async fn delete_node(&self, node_id: &str) -> StorageResult<()> {
        let _physical_id = Uuid::parse_str(node_id).map_err(|_| {
            StorageError::InvalidInput(
                "Neo4j delete_node requires an exact physical revision UUID".into(),
            )
        })?;
        Err(StorageError::InvalidInput(
            "Neo4j delete_node requires scope; use delete_entity_revisions".into(),
        ))
    }

    async fn delete_node_scoped(
        &self,
        node_id: &str,
        tenant_id: &str,
        workspace_id: &str,
    ) -> StorageResult<bool> {
        use edgequake_storage_contracts::{TenantId, WorkspaceId};
        let physical_id = Uuid::parse_str(node_id)
            .map_err(|_| StorageError::InvalidInput("invalid physical revision UUID".into()))?;
        let tenant = Uuid::parse_str(tenant_id)
            .map_err(|_| StorageError::InvalidInput("invalid tenant UUID".into()))?;
        let workspace = Uuid::parse_str(workspace_id)
            .map_err(|_| StorageError::InvalidInput("invalid workspace UUID".into()))?;
        let scope = AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace));
        self.delete_entity_revisions(&scope, &[physical_id])
            .await
            .map(|deleted| deleted == 1)
            .map_err(StorageError::from)
    }

    async fn delete_nodes_scoped_batch(
        &self,
        node_ids: &[String],
        tenant_id: &str,
        workspace_id: &str,
    ) -> StorageResult<usize> {
        use edgequake_storage_contracts::{TenantId, WorkspaceId};
        if node_ids.is_empty() {
            return Ok(0);
        }
        let tenant = Uuid::parse_str(tenant_id)
            .map_err(|_| StorageError::InvalidInput("invalid tenant UUID".into()))?;
        let workspace = Uuid::parse_str(workspace_id)
            .map_err(|_| StorageError::InvalidInput("invalid workspace UUID".into()))?;
        let scope = AccessScope::new(TenantId::new(tenant), WorkspaceId::new(workspace));
        let mut physical = Vec::with_capacity(node_ids.len());
        for id in node_ids {
            physical.push(Uuid::parse_str(id).map_err(|_| {
                StorageError::InvalidInput("invalid physical revision UUID".into())
            })?);
        }
        self.delete_entity_revisions(&scope, &physical)
            .await
            .map_err(StorageError::from)
    }

    async fn upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, Value>,
    ) -> StorageResult<()> {
        self.upsert_edges_batch(&[(source.to_owned(), target.to_owned(), properties)])
            .await
    }

    async fn upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, Value>)],
    ) -> StorageResult<()> {
        let edges = edges
            .iter()
            .map(|(source, target, properties)| {
                EdgeRevision::from_legacy(source, target, properties.clone())
            })
            .collect::<AccessResult<Vec<_>>>()
            .map_err(StorageError::from)?;
        self.upsert_graph_revisions(&[], &edges)
            .await
            .map(|_| ())
            .map_err(StorageError::from)
    }

    async fn upsert_edges_batch_with_mode(
        &self,
        edges: &[(String, String, HashMap<String, Value>)],
        _mode: GraphPropertyWriteMode,
    ) -> StorageResult<()> {
        self.upsert_edges_batch(edges).await
    }

    async fn delete_edge(&self, _source: &str, _target: &str) -> StorageResult<()> {
        Err(exact_edge_delete_required())
    }

    async fn delete_edges_batch(&self, _edges: &[(String, String, String)]) -> StorageResult<()> {
        Err(exact_edge_delete_required())
    }

    async fn delete_edge_scoped(
        &self,
        _source: &str,
        _target: &str,
        _tenant_id: &str,
        _workspace_id: &str,
    ) -> StorageResult<bool> {
        Err(exact_edge_delete_required())
    }

    async fn delete_edges_scoped_batch(
        &self,
        _edges: &[(String, String)],
        _tenant_id: &str,
        _workspace_id: &str,
    ) -> StorageResult<usize> {
        Err(exact_edge_delete_required())
    }

    async fn clear(&self) -> StorageResult<()> {
        Err(StorageError::UnsupportedCapability(
            "Neo4j global clear is intentionally unavailable".into(),
        ))
    }

    async fn clear_workspace(&self, _workspace_id: &Uuid) -> StorageResult<(usize, usize)> {
        Err(StorageError::UnsupportedCapability(
            "Neo4j workspace clear must use exact revision manifests".into(),
        ))
    }
}

fn exact_edge_delete_required() -> StorageError {
    StorageError::UnsupportedCapability(
        "Neo4j edges must be deleted by exact physical revision UUID, never endpoint pair".into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cypher_uses_fixed_structural_relationship_types() {
        assert!(UPSERT_EDGES.contains("[:FROM]"));
        assert!(UPSERT_EDGES.contains("[:TO]"));
        assert!(UPSERT_EDGES.contains("row.relationship_type"));
        assert!(!UPSERT_EDGES.contains("[:$"));
    }

    #[test]
    fn endpoint_pair_delete_is_refused() {
        assert!(matches!(
            exact_edge_delete_required(),
            StorageError::UnsupportedCapability(_)
        ));
    }
}
