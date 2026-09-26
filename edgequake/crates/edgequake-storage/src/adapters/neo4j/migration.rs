//! PostgreSQL-authority to Neo4j graph replay and rollback gates.

use std::collections::HashMap;

use edgequake_storage_contracts::{AccessError, AccessResult, AccessScope};
use sqlx::PgPool;
use uuid::Uuid;

use super::client::Neo4jClient;
use super::mapping::{EdgeRevision, EntityRevision, GraphObjectPayload};

const MAX_REPLAY_ROWS: u32 = 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphReplayReport {
    pub entity_revisions: usize,
    pub edge_revisions: usize,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDigestMismatch {
    pub physical_id: Uuid,
    pub authority_digest: [u8; 32],
    pub provider_digest: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphDigestReport {
    pub checked: usize,
    pub mismatches: Vec<GraphDigestMismatch>,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgeRollbackReadiness {
    pub binding_id: Uuid,
    pub pending_events: u64,
}

#[derive(Clone)]
pub struct PgNeo4jGraphMigration {
    authority: PgPool,
    neo4j: Neo4jClient,
}

impl PgNeo4jGraphMigration {
    pub fn new(authority: PgPool, neo4j: Neo4jClient) -> Self {
        Self { authority, neo4j }
    }

    pub async fn replay_page(
        &self,
        scope: &AccessScope,
        after: Option<Uuid>,
        limit: u32,
    ) -> AccessResult<GraphReplayReport> {
        let rows = load_authority_page(&self.authority, scope, after, limit).await?;
        if rows.is_empty() {
            return Ok(GraphReplayReport {
                entity_revisions: 0,
                edge_revisions: 0,
                next_cursor: None,
            });
        }
        let manifests = load_contribution_manifests(&self.authority, scope, &rows).await?;
        let (entities, edges) = map_authority_rows(scope, &rows, &manifests)?;
        self.neo4j.upsert_graph_revisions(&entities, &edges).await?;
        Ok(GraphReplayReport {
            entity_revisions: entities.len(),
            edge_revisions: edges.len(),
            next_cursor: rows.last().map(|row| row.physical_id),
        })
    }

    pub async fn compare_digest_page(
        &self,
        scope: &AccessScope,
        after: Option<Uuid>,
        limit: u32,
    ) -> AccessResult<GraphDigestReport> {
        let rows = load_authority_page(&self.authority, scope, after, limit).await?;
        let physical_ids: Vec<Uuid> = rows.iter().map(|row| row.physical_id).collect();
        let provider: HashMap<Uuid, [u8; 32]> = self
            .neo4j
            .revision_digests(scope, &physical_ids)
            .await?
            .into_iter()
            .map(|item| (item.physical_id, item.digest))
            .collect();
        let mismatches = rows
            .iter()
            .filter_map(|row| {
                let actual = provider.get(&row.physical_id).copied();
                (actual != Some(row.digest)).then_some(GraphDigestMismatch {
                    physical_id: row.physical_id,
                    authority_digest: row.digest,
                    provider_digest: actual,
                })
            })
            .collect();
        Ok(GraphDigestReport {
            checked: rows.len(),
            mismatches,
            next_cursor: rows.last().map(|row| row.physical_id),
        })
    }

    /// Rollback is a binding switch performed elsewhere. This gate only proves
    /// that the proposed AGE target has an applied delivery for every retained
    /// scoped graph event.
    pub async fn ensure_age_rollback_safe(
        &self,
        scope: &AccessScope,
        age_binding_id: Uuid,
    ) -> AccessResult<AgeRollbackReadiness> {
        let binding = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
            "SELECT tenant_id, workspace_id, provider, state \
             FROM public.data_bindings WHERE binding_id = $1",
        )
        .bind(age_binding_id)
        .fetch_optional(&self.authority)
        .await
        .map_err(database_error)?
        .ok_or_else(|| AccessError::NotFound(format!("AGE binding {age_binding_id}")))?;
        if binding.0 != scope.tenant().into_uuid() || binding.1 != scope.workspace().into_uuid() {
            return Err(AccessError::ForbiddenScope(
                "AGE rollback binding belongs to another scope".into(),
            ));
        }
        if !binding.2.eq_ignore_ascii_case("age")
            || !matches!(binding.3.as_str(), "active" | "draining")
        {
            return Err(AccessError::InvalidInput(format!(
                "binding {age_binding_id} is not an active or draining AGE binding"
            )));
        }
        let pending = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT count(*)
            FROM public.projection_events event
            WHERE event.tenant_id = $1
              AND event.workspace_id = $2
              AND event.object_kind IN (
                'document_batch', 'document', 'fact', 'entity', 'edge'
              )
              AND NOT EXISTS (
                SELECT 1
                FROM public.projection_deliveries delivery
                WHERE delivery.event_id = event.event_id
                  AND delivery.binding_id = $3
                  AND delivery.state = 'applied'
              )
            "#,
        )
        .bind(scope.tenant().into_uuid())
        .bind(scope.workspace().into_uuid())
        .bind(age_binding_id)
        .fetch_one(&self.authority)
        .await
        .map_err(database_error)?;
        rollback_lag_result(age_binding_id, pending)
    }
}

#[derive(sqlx::FromRow)]
struct AuthorityRevisionRow {
    logical_id: Uuid,
    revision: i64,
    physical_id: Uuid,
    digest: Vec<u8>,
    payload: Vec<u8>,
}

impl AuthorityRevisionRow {
    fn checked_revision(&self) -> AccessResult<u64> {
        u64::try_from(self.revision)
            .map_err(|_| AccessError::CorruptData("negative graph authority revision".into()))
    }

    fn checked_digest(&self) -> AccessResult<[u8; 32]> {
        self.digest
            .clone()
            .try_into()
            .map_err(|_| AccessError::CorruptData("graph authority digest is not 32 bytes".into()))
    }
}

struct CheckedAuthorityRevision {
    logical_id: Uuid,
    revision: u64,
    physical_id: Uuid,
    digest: [u8; 32],
    payload: Vec<u8>,
}

async fn load_authority_page(
    pool: &PgPool,
    scope: &AccessScope,
    after: Option<Uuid>,
    limit: u32,
) -> AccessResult<Vec<CheckedAuthorityRevision>> {
    validate_page_limit(limit)?;
    let rows = sqlx::query_as::<_, AuthorityRevisionRow>(
        r#"
        SELECT logical_id, revision, physical_id, digest, payload
        FROM public.object_revisions
        WHERE tenant_id = $1
          AND workspace_id = $2
          AND kind = 'fact'
          AND state <> 'tombstoned'
          AND ($3::uuid IS NULL OR physical_id > $3)
        ORDER BY physical_id
        LIMIT $4
        "#,
    )
    .bind(scope.tenant().into_uuid())
    .bind(scope.workspace().into_uuid())
    .bind(after)
    .bind(i64::from(limit))
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    rows.into_iter()
        .map(|row| {
            Ok(CheckedAuthorityRevision {
                logical_id: row.logical_id,
                revision: row.checked_revision()?,
                physical_id: row.physical_id,
                digest: row.checked_digest()?,
                payload: row.payload,
            })
        })
        .collect()
}

async fn load_contribution_manifests(
    pool: &PgPool,
    scope: &AccessScope,
    rows: &[CheckedAuthorityRevision],
) -> AccessResult<HashMap<(Uuid, u64), Vec<Uuid>>> {
    if rows.is_empty() {
        return Ok(HashMap::new());
    }
    let ids: Vec<Uuid> = rows.iter().map(|row| row.logical_id).collect();
    let revisions: Vec<i64> = rows
        .iter()
        .map(|row| i64::try_from(row.revision).expect("authority revision already came from i64"))
        .collect();
    let contributions = sqlx::query_as::<_, (Uuid, i64, Uuid)>(
        r#"
        WITH requested AS (
          SELECT fact_id, fact_revision
          FROM unnest($3::uuid[], $4::bigint[]) AS r(fact_id, fact_revision)
        )
        SELECT contribution.fact_id, contribution.fact_revision,
               contribution.contribution_id
        FROM public.graph_contributions contribution
        JOIN requested
          ON requested.fact_id = contribution.fact_id
         AND requested.fact_revision = contribution.fact_revision
        WHERE contribution.tenant_id = $1
          AND contribution.workspace_id = $2
        ORDER BY contribution.fact_id, contribution.fact_revision,
                 contribution.contribution_id
        "#,
    )
    .bind(scope.tenant().into_uuid())
    .bind(scope.workspace().into_uuid())
    .bind(ids)
    .bind(revisions)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    let mut manifests: HashMap<(Uuid, u64), Vec<Uuid>> = HashMap::new();
    for (fact_id, revision, contribution_id) in contributions {
        let revision = u64::try_from(revision)
            .map_err(|_| AccessError::CorruptData("negative contribution revision".into()))?;
        manifests
            .entry((fact_id, revision))
            .or_default()
            .push(contribution_id);
    }
    Ok(manifests)
}

fn map_authority_rows(
    scope: &AccessScope,
    rows: &[CheckedAuthorityRevision],
    manifests: &HashMap<(Uuid, u64), Vec<Uuid>>,
) -> AccessResult<(Vec<EntityRevision>, Vec<EdgeRevision>)> {
    let mut entities = Vec::new();
    let mut edges = Vec::new();
    for row in rows {
        let payload: GraphObjectPayload =
            serde_json::from_slice(&row.payload).map_err(|error| {
                AccessError::CorruptData(format!(
                    "graph fact {} revision {} has invalid projection payload: {error}",
                    row.logical_id, row.revision
                ))
            })?;
        let contribution_manifest = manifests
            .get(&(row.logical_id, row.revision))
            .cloned()
            .unwrap_or_default();
        match payload {
            GraphObjectPayload::Entity {
                logical_id,
                properties,
            } => {
                require_logical_id(row.logical_id, logical_id.into_uuid())?;
                entities.push(EntityRevision {
                    scope: *scope,
                    logical_id,
                    physical_id: row.physical_id,
                    revision: row.revision,
                    digest: row.digest,
                    contribution_manifest,
                    properties,
                });
            }
            GraphObjectPayload::Edge {
                logical_id,
                source,
                target,
                relationship_type,
                direction,
                properties,
            } => {
                require_logical_id(row.logical_id, logical_id.into_uuid())?;
                edges.push(EdgeRevision {
                    scope: *scope,
                    logical_id,
                    physical_id: row.physical_id,
                    source,
                    target,
                    relationship_type,
                    direction,
                    revision: row.revision,
                    digest: row.digest,
                    contribution_manifest,
                    properties,
                });
            }
        }
    }
    Ok((entities, edges))
}

fn require_logical_id(authority: Uuid, payload: Uuid) -> AccessResult<()> {
    if authority == payload {
        Ok(())
    } else {
        Err(AccessError::CorruptData(format!(
            "graph payload logical ID {payload} disagrees with authority {authority}"
        )))
    }
}

fn validate_page_limit(limit: u32) -> AccessResult<()> {
    if limit == 0 || limit > MAX_REPLAY_ROWS {
        return Err(AccessError::InvalidInput(format!(
            "graph replay limit must be between 1 and {MAX_REPLAY_ROWS}"
        )));
    }
    Ok(())
}

fn rollback_lag_result(binding_id: Uuid, pending: i64) -> AccessResult<AgeRollbackReadiness> {
    let pending = u64::try_from(pending)
        .map_err(|_| AccessError::CorruptData("negative AGE projection lag".into()))?;
    if pending > 0 {
        return Err(AccessError::Conflict(format!(
            "AGE rollback binding {binding_id} is lagging by {pending} graph events"
        )));
    }
    Ok(AgeRollbackReadiness {
        binding_id,
        pending_events: 0,
    })
}

fn database_error(error: sqlx::Error) -> AccessError {
    crate::error::StorageError::from(error).into()
}

#[cfg(test)]
mod tests {
    use edgequake_storage_contracts::{
        EdgeDirection, GraphEdgeKey, GraphNodeKey, TenantId, WorkspaceId,
    };
    use serde_json::Map;

    use super::*;

    #[test]
    fn rollback_refuses_lagging_age_binding() {
        let error = rollback_lag_result(Uuid::from_u128(1), 1).unwrap_err();
        assert!(matches!(error, AccessError::Conflict(_)));
        assert!(error.to_string().contains("lagging by 1"));
    }

    #[test]
    fn replay_mapping_preserves_edge_application_identity() {
        let logical_id = Uuid::from_u128(3);
        let payload = GraphObjectPayload::Edge {
            logical_id: GraphEdgeKey::new(logical_id),
            source: GraphNodeKey::new(Uuid::from_u128(4)),
            target: GraphNodeKey::new(Uuid::from_u128(4)),
            relationship_type: "SELF".into(),
            direction: EdgeDirection::Directed,
            properties: Map::new(),
        };
        let rows = vec![CheckedAuthorityRevision {
            logical_id,
            revision: 1,
            physical_id: Uuid::from_u128(5),
            digest: [6; 32],
            payload: serde_json::to_vec(&payload).unwrap(),
        }];
        let scope = AccessScope::new(
            TenantId::new(Uuid::from_u128(1)),
            WorkspaceId::new(Uuid::from_u128(2)),
        );
        let (_, edges) = map_authority_rows(&scope, &rows, &HashMap::new()).unwrap();
        assert_eq!(edges[0].logical_id, GraphEdgeKey::new(logical_id));
        assert_eq!(edges[0].source, edges[0].target);
    }
}
