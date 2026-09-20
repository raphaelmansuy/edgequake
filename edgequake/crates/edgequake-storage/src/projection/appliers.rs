//! Real P0 projection appliers backed by AGE and typed PostgreSQL embeddings.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_storage_contracts::{
    AccessError, AccessResult, ProjectionEvent, ProjectionOperation,
};
use serde::Deserialize;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::embedding_family::EmbeddingFamily;
use crate::traits::domain::{
    ChunkId, EmbeddingIndex, EmbeddingRow, FleetEmbeddingIndex, FleetEmbeddingKey,
    FleetEmbeddingRow, ModelId, WorkspaceId,
};
use crate::traits::GraphStorage;

use super::payload::ProjectionApplyReceipt;
use super::worker::{GraphProjectionApplier, VectorProjectionApplier};

/// AGE projection applier with authority-payload hydration from PostgreSQL.
pub struct AgeGraphProjectionApplier {
    pub graph: Arc<dyn GraphStorage>,
    pool: PgPool,
}

impl AgeGraphProjectionApplier {
    pub fn new(graph: Arc<dyn GraphStorage>, pool: PgPool) -> Self {
        Self { graph, pool }
    }

    async fn load_batch_facts(&self, event: &ProjectionEvent) -> AccessResult<Vec<Value>> {
        let generation = i64::try_from(event.object_revision)
            .map_err(|_| AccessError::CorruptData("projection revision exceeds i64".into()))?;
        sqlx::query_scalar::<_, Value>(
            "SELECT payload FROM public.graph_contributions \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND source_document_id = $3 AND source_generation = $4 \
             ORDER BY contribution_id",
        )
        .bind(event.scope.tenant().into_uuid())
        .bind(event.scope.workspace().into_uuid())
        .bind(event.object_id)
        .bind(generation)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)
    }

    async fn apply_fact(
        &self,
        event: &ProjectionEvent,
        payload: Value,
        operation: ProjectionOperation,
    ) -> AccessResult<()> {
        let schema = payload
            .get("schema")
            .and_then(Value::as_str)
            .ok_or_else(|| AccessError::CorruptData("graph fact has no schema".into()))?;
        if schema != "edgequake.graph.fact.v1" {
            return Err(AccessError::CorruptData(format!(
                "unknown graph fact schema '{schema}'"
            )));
        }
        let kind = payload
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| AccessError::CorruptData("graph fact has no kind".into()))?;
        let tenant = event.scope.tenant().into_uuid();
        let workspace = event.scope.workspace().into_uuid();
        let tenant_s = tenant.to_string();
        let workspace_s = workspace.to_string();
        match (kind, operation) {
            ("node", ProjectionOperation::Upsert) => {
                let logical = required_string(&payload, "node_id")?;
                let node_id = super::payload::scoped_graph_node_id(tenant, workspace, &logical);
                let mut props = properties(&payload)?;
                props.insert("tenant_id".into(), serde_json::json!(tenant_s));
                props.insert("workspace_id".into(), serde_json::json!(workspace_s));
                props.insert("logical_node_id".into(), serde_json::json!(logical));
                self.graph
                    .upsert_node(&node_id, props)
                    .await
                    .map_err(AccessError::from)
            }
            ("node", ProjectionOperation::Delete) => {
                let logical = required_string(&payload, "node_id")?;
                let node_id = super::payload::scoped_graph_node_id(tenant, workspace, &logical);
                let deleted = self
                    .graph
                    .delete_node_scoped(&node_id, &tenant_s, &workspace_s)
                    .await
                    .map_err(AccessError::from)?;
                if !deleted {
                    return Err(AccessError::CorruptData(format!(
                        "scoped graph node '{logical}' was absent for delete delivery"
                    )));
                }
                Ok(())
            }
            ("edge", ProjectionOperation::Upsert) => {
                let source_logical = required_string(&payload, "source")?;
                let target_logical = required_string(&payload, "target")?;
                let source =
                    super::payload::scoped_graph_node_id(tenant, workspace, &source_logical);
                let target =
                    super::payload::scoped_graph_node_id(tenant, workspace, &target_logical);
                let mut props = properties(&payload)?;
                props.insert("tenant_id".into(), serde_json::json!(tenant_s));
                props.insert("workspace_id".into(), serde_json::json!(workspace_s));
                self.graph
                    .upsert_edge(&source, &target, props)
                    .await
                    .map_err(AccessError::from)
            }
            ("edge", ProjectionOperation::Delete) => {
                let source_logical = required_string(&payload, "source")?;
                let target_logical = required_string(&payload, "target")?;
                let source =
                    super::payload::scoped_graph_node_id(tenant, workspace, &source_logical);
                let target =
                    super::payload::scoped_graph_node_id(tenant, workspace, &target_logical);
                let deleted = self
                    .graph
                    .delete_edge_scoped(&source, &target, &tenant_s, &workspace_s)
                    .await
                    .map_err(AccessError::from)?;
                if !deleted {
                    return Err(AccessError::CorruptData(
                        "scoped graph edge was absent for delete delivery".into(),
                    ));
                }
                Ok(())
            }
            (unknown, _) => Err(AccessError::CorruptData(format!(
                "unknown graph fact kind '{unknown}'"
            ))),
        }
    }
}

#[async_trait]
impl GraphProjectionApplier for AgeGraphProjectionApplier {
    async fn apply(
        &self,
        event: &ProjectionEvent,
        binding: &edgequake_storage_contracts::DataBindingDescriptor,
    ) -> AccessResult<ProjectionApplyReceipt> {
        binding.require_graph()?;
        binding.require_provider("age")?;
        match (&*event.object_kind, event.operation) {
            ("document_batch", ProjectionOperation::Upsert) => {
                for payload in self.load_batch_facts(event).await? {
                    self.apply_fact(event, payload, ProjectionOperation::Upsert)
                        .await?;
                }
            }
            ("document", ProjectionOperation::Delete) => {
                // Physical graph prune is owned by deletion cascade + cleanup intents.
                // Delivery ack is a visibility fence, not a silent no-op mutation.
            }
            ("fact", operation) => {
                let revision = i64::try_from(event.object_revision).map_err(|_| {
                    AccessError::CorruptData("fact projection revision exceeds i64".into())
                })?;
                let payload = sqlx::query_scalar::<_, Vec<u8>>(
                    "SELECT payload FROM public.object_revisions \
                     WHERE tenant_id = $1 AND workspace_id = $2 AND kind = 'fact' \
                       AND logical_id = $3 AND revision = $4",
                )
                .bind(event.scope.tenant().into_uuid())
                .bind(event.scope.workspace().into_uuid())
                .bind(event.object_id)
                .bind(revision)
                .fetch_optional(&self.pool)
                .await
                .map_err(database_error)?
                .ok_or_else(|| AccessError::CorruptData("fact payload is absent".into()))?;
                let payload = serde_json::from_slice(&payload)
                    .map_err(|error| AccessError::CorruptData(error.to_string()))?;
                self.apply_fact(event, payload, operation).await?;
            }
            (kind, _) => {
                return Err(AccessError::CorruptData(format!(
                    "unknown graph projection object kind '{kind}'"
                )));
            }
        }
        Ok(receipt("age", event, binding.binding_id))
    }
}

/// Typed pgvector projection applier.
pub struct PgvectorProjectionApplier {
    pub chunk_index: Arc<dyn EmbeddingIndex>,
    pub fleet: Option<Arc<dyn FleetEmbeddingIndex>>,
    pool: PgPool,
}

impl PgvectorProjectionApplier {
    pub fn new(
        chunk_index: Arc<dyn EmbeddingIndex>,
        fleet: Option<Arc<dyn FleetEmbeddingIndex>>,
        pool: PgPool,
    ) -> Self {
        Self {
            chunk_index,
            fleet,
            pool,
        }
    }

    async fn load_embeddings(
        &self,
        event: &ProjectionEvent,
    ) -> AccessResult<Vec<EmbeddingPayload>> {
        let revision = i64::try_from(event.object_revision)
            .map_err(|_| AccessError::CorruptData("projection revision exceeds i64".into()))?;
        // Manifests are keyed by ingest generation (= event.object_revision).
        // Do not join live chunks: delayed cleanup must not empty this set.
        let payloads = sqlx::query_scalar::<_, Option<Vec<u8>>>(
            "SELECT payload FROM public.embedding_manifests \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND content_revision = $3 \
             ORDER BY subject_id",
        )
        .bind(event.scope.tenant().into_uuid())
        .bind(event.scope.workspace().into_uuid())
        .bind(revision)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;
        payloads
            .into_iter()
            .map(|payload| {
                let payload = payload.ok_or_else(|| {
                    AccessError::CorruptData("embedding manifest payload is absent".into())
                })?;
                serde_json::from_slice::<EmbeddingPayload>(&payload).map_err(|error| {
                    AccessError::CorruptData(format!("invalid embedding payload: {error}"))
                })
            })
            .collect()
    }

    async fn upsert_payloads(
        &self,
        event: &ProjectionEvent,
        payloads: Vec<EmbeddingPayload>,
    ) -> AccessResult<()> {
        let mut chunk_rows = Vec::new();
        let mut fleet_rows: HashMap<&'static str, Vec<FleetEmbeddingRow>> = HashMap::new();
        for payload in payloads {
            payload.validate()?;
            let workspace_id = WorkspaceId::new(event.scope.workspace().into_uuid());
            match payload.family.as_str() {
                "chunk" => chunk_rows.push(EmbeddingRow {
                    chunk_id: ChunkId(payload.subject_id),
                    workspace_id,
                    dimensions: payload.dimensions,
                    embedding: payload.embedding,
                }),
                "entity" | "relationship" | "report" => {
                    let key = match payload.family.as_str() {
                        "entity" => FleetEmbeddingKey::Entity(payload.subject_id),
                        "relationship" => FleetEmbeddingKey::Relationship(payload.subject_id),
                        "report" => FleetEmbeddingKey::Report(
                            payload
                                .legacy_vector_id
                                .clone()
                                .unwrap_or_else(|| payload.subject_id.to_string()),
                        ),
                        _ => unreachable!(),
                    };
                    fleet_rows
                        .entry(match payload.family.as_str() {
                            "entity" => "entity",
                            "relationship" => "relationship",
                            _ => "report",
                        })
                        .or_default()
                        .push(FleetEmbeddingRow {
                            workspace_id,
                            dimensions: payload.dimensions,
                            embedding: payload.embedding,
                            key,
                            legacy_vector_id: payload.legacy_vector_id,
                        });
                }
                family => {
                    return Err(AccessError::CorruptData(format!(
                        "unknown embedding family '{family}'"
                    )));
                }
            }
        }
        if !chunk_rows.is_empty() {
            self.chunk_index
                .upsert_batch(ModelId(Uuid::nil()), &chunk_rows)
                .await
                .map_err(AccessError::from)?;
        }
        if !fleet_rows.is_empty() {
            let fleet = self.fleet.as_ref().ok_or_else(|| {
                AccessError::UnsupportedCapability("fleet embedding index is not wired".into())
            })?;
            for (family, rows) in fleet_rows {
                let family = match family {
                    "entity" => EmbeddingFamily::Entity,
                    "relationship" => EmbeddingFamily::Relationship,
                    _ => EmbeddingFamily::Report,
                };
                fleet
                    .upsert_batch(family, ModelId(Uuid::nil()), &rows)
                    .await
                    .map_err(AccessError::from)?;
            }
        }
        Ok(())
    }

    async fn delete_document_embeddings(&self, event: &ProjectionEvent) -> AccessResult<()> {
        let tombstone_revision = i64::try_from(event.object_revision)
            .map_err(|_| AccessError::CorruptData("projection revision exceeds i64".into()))?;
        // Authority rows must still exist when cleanup runs. Empty chunk lists mean
        // the document was already erased — that is not a successful no-op.
        let chunk_ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM public.chunks WHERE document_id = $1 ORDER BY id")
                .bind(event.object_id)
                .fetch_all(&self.pool)
                .await
                .map_err(database_error)?;
        if chunk_ids.is_empty() {
            return Err(AccessError::CorruptData(
                "delete delivery found zero authority chunks; refuse empty vector cleanup".into(),
            ));
        }

        // Manifests are keyed by ingest generation, not the tombstone revision.
        let subjects: Vec<Uuid> = sqlx::query_scalar(
            "SELECT DISTINCT subject_id FROM public.embedding_manifests \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND subject_id = ANY($3) AND content_revision < $4",
        )
        .bind(event.scope.tenant().into_uuid())
        .bind(event.scope.workspace().into_uuid())
        .bind(&chunk_ids)
        .bind(tombstone_revision)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;

        if subjects.is_empty() {
            // Document never projected embeddings for these chunks — visibility
            // fence still applies; physical store has nothing revision-scoped.
            return Ok(());
        }

        let removed_embeddings = sqlx::query(
            "DELETE FROM public.chunk_embeddings WHERE chunk_id = ANY($1) \
             AND workspace_id = $2",
        )
        .bind(&subjects)
        .bind(event.scope.workspace().into_uuid())
        .execute(&self.pool)
        .await
        .map_err(database_error)?
        .rows_affected();

        let removed_manifests = sqlx::query(
            "DELETE FROM public.embedding_manifests \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND subject_id = ANY($3) AND content_revision < $4",
        )
        .bind(event.scope.tenant().into_uuid())
        .bind(event.scope.workspace().into_uuid())
        .bind(&subjects)
        .bind(tombstone_revision)
        .execute(&self.pool)
        .await
        .map_err(database_error)?
        .rows_affected();

        if removed_manifests == 0 {
            return Err(AccessError::CorruptData(
                "delete delivery removed zero embedding manifests".into(),
            ));
        }
        // Embeddings may already be gone after a prior partial cleanup; manifests
        // are the authority proof that this revision-scoped delete did work.
        let _ = removed_embeddings;
        Ok(())
    }
}

#[async_trait]
impl VectorProjectionApplier for PgvectorProjectionApplier {
    async fn apply(
        &self,
        event: &ProjectionEvent,
        binding: &edgequake_storage_contracts::DataBindingDescriptor,
    ) -> AccessResult<ProjectionApplyReceipt> {
        binding.require_vector()?;
        binding.require_provider("pgvector")?;
        match (&*event.object_kind, event.operation) {
            ("document_batch", ProjectionOperation::Upsert) => {
                let payloads = self.load_embeddings(event).await?;
                if payloads.is_empty() {
                    // Empty embedding set is valid only when the batch had none.
                    // Still produce a digest-matching receipt for visibility.
                } else {
                    self.upsert_payloads(event, payloads).await?;
                }
            }
            ("document", ProjectionOperation::Delete) => {
                self.delete_document_embeddings(event).await?;
            }
            (kind, _) => {
                return Err(AccessError::CorruptData(format!(
                    "unknown vector projection object kind '{kind}'"
                )));
            }
        }
        Ok(receipt("pgvector", event, binding.binding_id))
    }
}

#[derive(Debug, Deserialize)]
struct EmbeddingPayload {
    schema: String,
    family: String,
    subject_id: Uuid,
    dimensions: i32,
    embedding: Vec<f32>,
    #[serde(default)]
    legacy_vector_id: Option<String>,
}

impl EmbeddingPayload {
    fn validate(&self) -> AccessResult<()> {
        if self.schema != "edgequake.embedding.v1" {
            return Err(AccessError::CorruptData(format!(
                "unknown embedding schema '{}'",
                self.schema
            )));
        }
        if self.dimensions <= 0 || self.embedding.len() != self.dimensions as usize {
            return Err(AccessError::CorruptData(format!(
                "embedding dimension {} does not match payload length {}",
                self.dimensions,
                self.embedding.len()
            )));
        }
        Ok(())
    }
}

fn properties(payload: &Value) -> AccessResult<HashMap<String, Value>> {
    serde_json::from_value(
        payload
            .get("properties")
            .cloned()
            .ok_or_else(|| AccessError::CorruptData("graph fact has no properties".into()))?,
    )
    .map_err(|error| AccessError::CorruptData(format!("invalid graph properties: {error}")))
}

fn required_string(payload: &Value, key: &str) -> AccessResult<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| AccessError::CorruptData(format!("graph fact has no {key}")))
}

fn receipt(provider: &str, event: &ProjectionEvent, binding_id: Uuid) -> ProjectionApplyReceipt {
    ProjectionApplyReceipt {
        provider_receipt: format!("{provider}:{}:{binding_id}", event.event_id),
        completion_proof: event.payload_digest.to_vec(),
    }
}

fn database_error(error: sqlx::Error) -> AccessError {
    AccessError::from(crate::StorageError::from(error))
}
