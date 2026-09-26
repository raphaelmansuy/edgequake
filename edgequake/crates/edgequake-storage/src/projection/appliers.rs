//! Real P0 projection appliers backed by AGE and typed PostgreSQL embeddings.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_storage_contracts::{
    AccessError, AccessResult, DataBindingDescriptor, ProjectionEvent, ProjectionOperation,
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
use crate::traits::{GraphPropertyWriteMode, GraphStorage};

use super::payload::ProjectionApplyReceipt;
use super::worker::{GraphProjectionApplier, VectorProjectionApplier};
use crate::lineage_canon::{
    apply_retained_sources, canonicalize_source_lineage, document_ids_from_properties,
    retained_lineage, union_source_properties,
};
use crate::projection_manifest::{
    canonical_graph_node_id, role_completion_proof, EventManifestItem,
};

/// AGE projection applier with authority-payload hydration from PostgreSQL.
pub struct AgeGraphProjectionApplier {
    pub graph: Arc<dyn GraphStorage>,
    pool: PgPool,
    apply_batch_entries: AtomicU64,
    provider_batch_writes: AtomicU64,
    scoped_batch_deletes: AtomicU64,
    retain_reads: AtomicU64,
    fact_hydration_queries: AtomicU64,
    cleanup_statements: AtomicU64,
}

/// A contribution still names a live document when that document has no
/// committed delete event. Tombstones leave `graph_contributions` in place.
const EXCLUDE_TOMBSTONED_DOCUMENTS: &str = "AND NOT EXISTS ( \
     SELECT 1 FROM public.projection_events tombstone \
     WHERE tombstone.tenant_id = graph_contributions.tenant_id \
       AND tombstone.workspace_id = graph_contributions.workspace_id \
       AND tombstone.object_kind = 'document' \
       AND tombstone.operation = 'delete' \
       AND tombstone.object_id = graph_contributions.source_document_id \
 )";

struct PendingNodeDelete {
    tenant: Uuid,
    workspace: Uuid,
    tenant_s: String,
    workspace_s: String,
    logical: String,
    node_id: String,
    excluding_document: Option<Uuid>,
}

struct PendingEdgeDelete {
    tenant: Uuid,
    workspace: Uuid,
    tenant_s: String,
    workspace_s: String,
    source_logical: String,
    target_logical: String,
    source: String,
    target: String,
    excluding_document: Option<Uuid>,
}

impl AgeGraphProjectionApplier {
    pub fn new(graph: Arc<dyn GraphStorage>, pool: PgPool) -> Self {
        Self {
            graph,
            pool,
            apply_batch_entries: AtomicU64::new(0),
            provider_batch_writes: AtomicU64::new(0),
            scoped_batch_deletes: AtomicU64::new(0),
            retain_reads: AtomicU64::new(0),
            fact_hydration_queries: AtomicU64::new(0),
            cleanup_statements: AtomicU64::new(0),
        }
    }

    pub fn apply_batch_entries(&self) -> u64 {
        self.apply_batch_entries.load(Ordering::Relaxed)
    }

    pub fn provider_batch_writes(&self) -> u64 {
        self.provider_batch_writes.load(Ordering::Relaxed)
    }

    pub fn scoped_batch_deletes(&self) -> u64 {
        self.scoped_batch_deletes.load(Ordering::Relaxed)
    }

    pub fn retain_reads(&self) -> u64 {
        self.retain_reads.load(Ordering::Relaxed)
    }

    pub fn fact_hydration_queries(&self) -> u64 {
        self.fact_hydration_queries.load(Ordering::Relaxed)
    }

    pub fn cleanup_statements(&self) -> u64 {
        self.cleanup_statements.load(Ordering::Relaxed)
    }

    /// One statement for many document upsert and delete events.
    async fn load_batch_facts_many(
        &self,
        event_ids: &[Uuid],
    ) -> AccessResult<HashMap<Uuid, (Vec<Value>, Vec<EventManifestItem>)>> {
        if event_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query_as::<_, (Uuid, String, Uuid, i64, Vec<u8>, String, Option<Value>)>(
            "SELECT i.event_id, i.item_kind, i.record_id, i.record_revision, i.digest, \
                    i.logical_key, c.payload \
             FROM public.projection_event_items i \
             LEFT JOIN public.graph_contributions c \
               ON c.contribution_id = i.record_id \
              AND c.payload_digest = i.digest \
             WHERE i.event_id = ANY($1) AND i.role = 'graph' \
             ORDER BY i.event_id, i.ordinal",
        )
        .bind(event_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;
        self.fact_hydration_queries.fetch_add(1, Ordering::Relaxed);

        let mut out: HashMap<Uuid, (Vec<Value>, Vec<EventManifestItem>)> = HashMap::new();
        for event_id in event_ids {
            out.entry(*event_id)
                .or_insert_with(|| (Vec::new(), Vec::new()));
        }
        for (event_id, item_kind, record_id, record_revision, digest, logical_key, payload) in rows
        {
            let payload = payload.ok_or_else(|| {
                AccessError::CorruptData(
                    "graph manifest membership does not match stored contributions".into(),
                )
            })?;
            let digest: [u8; 32] = digest.try_into().map_err(|_| {
                AccessError::CorruptData("projection manifest digest is not 32 bytes".into())
            })?;
            let entry = out.entry(event_id).or_default();
            entry.0.push(payload);
            entry.1.push(EventManifestItem {
                role: "graph".into(),
                item_kind,
                record_id,
                record_revision,
                digest,
                logical_key,
            });
        }
        Ok(out)
    }

    /// One UNNEST load for many object_kind=fact events. Empty is a no-query.
    async fn load_fact_revisions_many(
        &self,
        keys: &[(Uuid, Uuid, Uuid, Uuid, i64)],
    ) -> AccessResult<HashMap<Uuid, Vec<u8>>> {
        if keys.is_empty() {
            return Ok(HashMap::new());
        }
        let event_ids: Vec<Uuid> = keys.iter().map(|k| k.0).collect();
        let tenant_ids: Vec<Uuid> = keys.iter().map(|k| k.1).collect();
        let workspace_ids: Vec<Uuid> = keys.iter().map(|k| k.2).collect();
        let logical_ids: Vec<Uuid> = keys.iter().map(|k| k.3).collect();
        let revisions: Vec<i64> = keys.iter().map(|k| k.4).collect();
        let rows = sqlx::query_as::<_, (Uuid, Option<Vec<u8>>)>(
            "SELECT k.event_id, o.payload \
             FROM UNNEST($1::uuid[], $2::uuid[], $3::uuid[], $4::uuid[], $5::bigint[]) \
               AS k(event_id, tenant_id, workspace_id, logical_id, revision) \
             LEFT JOIN public.object_revisions o \
               ON o.tenant_id = k.tenant_id \
              AND o.workspace_id = k.workspace_id \
              AND o.kind = 'fact' \
              AND o.logical_id = k.logical_id \
              AND o.revision = k.revision",
        )
        .bind(&event_ids)
        .bind(&tenant_ids)
        .bind(&workspace_ids)
        .bind(&logical_ids)
        .bind(&revisions)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;
        self.fact_hydration_queries.fetch_add(1, Ordering::Relaxed);

        let mut out = HashMap::with_capacity(keys.len());
        for (event_id, payload) in rows {
            let payload =
                payload.ok_or_else(|| AccessError::CorruptData("fact payload is absent".into()))?;
            out.insert(event_id, payload);
        }
        if out.len() != keys.len() {
            return Err(AccessError::CorruptData("fact payload is absent".into()));
        }
        Ok(out)
    }

    async fn other_node_contributions_any(
        &self,
        tenant_id: Uuid,
        workspace_id: Uuid,
        logical_node_ids: &[String],
    ) -> AccessResult<Vec<(String, Uuid, Value)>> {
        if logical_node_ids.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "SELECT payload->>'node_id', source_document_id, payload \
             FROM public.graph_contributions \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND payload->>'kind' = 'node' \
               AND payload->>'node_id' = ANY($3) {EXCLUDE_TOMBSTONED_DOCUMENTS}"
        );
        sqlx::query_as(&sql)
            .bind(tenant_id)
            .bind(workspace_id)
            .bind(logical_node_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)
    }

    async fn other_edge_contributions_any(
        &self,
        tenant_id: Uuid,
        workspace_id: Uuid,
        pairs: &[(String, String)],
    ) -> AccessResult<Vec<(String, String, Uuid, Value)>> {
        if pairs.is_empty() {
            return Ok(Vec::new());
        }
        let sources: Vec<String> = pairs.iter().map(|(s, _)| s.clone()).collect();
        let targets: Vec<String> = pairs.iter().map(|(_, t)| t.clone()).collect();
        sqlx::query_as(&format!(
            "SELECT payload->>'source', payload->>'target', source_document_id, payload \
             FROM public.graph_contributions \
             WHERE tenant_id = $1 AND workspace_id = $2 \
               AND payload->>'kind' = 'edge' \
               AND (payload->>'source', payload->>'target') IN (
                   SELECT * FROM UNNEST($3::text[], $4::text[]) AS t(src, tgt)
               ) {EXCLUDE_TOMBSTONED_DOCUMENTS}"
        ))
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(&sources)
        .bind(&targets)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)
    }

    fn plan_upsert_node(
        event: &ProjectionEvent,
        payload: &Value,
        node_upserts: &mut HashMap<String, HashMap<String, Value>>,
    ) -> AccessResult<()> {
        let logical = required_string(payload, "node_id")?;
        let workspace = event.scope.workspace().into_uuid();
        let node_id = canonical_graph_node_id(workspace, &logical);
        let mut props = properties(payload)?;
        props.insert(
            "tenant_id".into(),
            serde_json::json!(event.scope.tenant().into_uuid().to_string()),
        );
        props.insert(
            "workspace_id".into(),
            serde_json::json!(workspace.to_string()),
        );
        props.insert("logical_node_id".into(), serde_json::json!(logical));
        canonicalize_source_lineage(&mut props);
        if let Some(existing) = node_upserts.get(&node_id) {
            let mut incoming = props;
            union_source_properties(existing, &mut incoming);
            node_upserts.insert(node_id, incoming);
        } else {
            node_upserts.insert(node_id, props);
        }
        Ok(())
    }

    fn plan_upsert_edge(
        event: &ProjectionEvent,
        payload: &Value,
        edge_upserts: &mut HashMap<(String, String), HashMap<String, Value>>,
    ) -> AccessResult<()> {
        let workspace = event.scope.workspace().into_uuid();
        let source_logical = required_string(payload, "source")?;
        let target_logical = required_string(payload, "target")?;
        let source = canonical_graph_node_id(workspace, &source_logical);
        let target = canonical_graph_node_id(workspace, &target_logical);
        let mut props = properties(payload)?;
        props.insert(
            "tenant_id".into(),
            serde_json::json!(event.scope.tenant().into_uuid().to_string()),
        );
        props.insert(
            "workspace_id".into(),
            serde_json::json!(workspace.to_string()),
        );
        canonicalize_source_lineage(&mut props);
        let key = (source, target);
        if let Some(existing) = edge_upserts.get(&key) {
            let mut incoming = props;
            union_source_properties(existing, &mut incoming);
            edge_upserts.insert(key, incoming);
        } else {
            edge_upserts.insert(key, props);
        }
        Ok(())
    }

    fn collect_delete_node(
        event: &ProjectionEvent,
        payload: &Value,
        pending: &mut Vec<PendingNodeDelete>,
    ) -> AccessResult<()> {
        let tenant = event.scope.tenant().into_uuid();
        let workspace = event.scope.workspace().into_uuid();
        let logical = required_string(payload, "node_id")?;
        let node_id = canonical_graph_node_id(workspace, &logical);
        pending.push(PendingNodeDelete {
            tenant,
            workspace,
            tenant_s: tenant.to_string(),
            workspace_s: workspace.to_string(),
            logical,
            node_id,
            excluding_document: excluding_document_id(event, ProjectionOperation::Delete, payload),
        });
        Ok(())
    }

    fn collect_delete_edge(
        event: &ProjectionEvent,
        payload: &Value,
        pending: &mut Vec<PendingEdgeDelete>,
    ) -> AccessResult<()> {
        let tenant = event.scope.tenant().into_uuid();
        let workspace = event.scope.workspace().into_uuid();
        let source_logical = required_string(payload, "source")?;
        let target_logical = required_string(payload, "target")?;
        let source = canonical_graph_node_id(workspace, &source_logical);
        let target = canonical_graph_node_id(workspace, &target_logical);
        pending.push(PendingEdgeDelete {
            tenant,
            workspace,
            tenant_s: tenant.to_string(),
            workspace_s: workspace.to_string(),
            source_logical,
            target_logical,
            source,
            target,
            excluding_document: excluding_document_id(event, ProjectionOperation::Delete, payload),
        });
        Ok(())
    }

    /// Document ids that exist in `public.documents` and have no
    /// `graph_contributions` row in this scope (legacy merger writes).
    async fn live_legacy_document_ids(
        &self,
        tenant_id: Uuid,
        workspace_id: Uuid,
        candidates: &[Uuid],
    ) -> AccessResult<std::collections::HashSet<String>> {
        use std::collections::HashSet;
        if candidates.is_empty() {
            return Ok(HashSet::new());
        }
        let rows: Vec<Uuid> = sqlx::query_scalar(
            "SELECT d.id FROM public.documents d \
             WHERE d.id = ANY($1) \
               AND d.tenant_id = $2 AND d.workspace_id = $3 \
               AND NOT EXISTS ( \
                 SELECT 1 FROM public.graph_contributions c \
                 WHERE c.tenant_id = $2 AND c.workspace_id = $3 \
                   AND c.source_document_id = d.id \
               )",
        )
        .bind(candidates)
        .bind(tenant_id)
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(rows.into_iter().map(|id| id.to_string()).collect())
    }

    async fn resolve_node_deletes(
        &self,
        pending: Vec<PendingNodeDelete>,
        node_deletes: &mut Vec<(String, String, String)>,
        node_retains: &mut Vec<(String, HashMap<String, Value>)>,
    ) -> AccessResult<()> {
        use std::collections::HashSet;

        if pending.is_empty() {
            return Ok(());
        }

        // Always load candidate graph rows when an excluding document is set —
        // legacy (no contribution) docs may still share the entity.
        let candidate_ids: Vec<String> = pending
            .iter()
            .filter(|p| p.excluding_document.is_some())
            .map(|p| p.node_id.clone())
            .collect();
        let existing = if candidate_ids.is_empty() {
            HashMap::new()
        } else {
            self.retain_reads.fetch_add(1, Ordering::Relaxed);
            self.graph
                .get_nodes_batch(&candidate_ids)
                .await
                .map_err(AccessError::from)?
        };

        let mut by_scope: HashMap<(Uuid, Uuid), Vec<usize>> = HashMap::new();
        for (idx, item) in pending.iter().enumerate() {
            by_scope
                .entry((item.tenant, item.workspace))
                .or_default()
                .push(idx);
        }

        for ((tenant, workspace), indices) in &by_scope {
            let mut logicals: Vec<String> = indices
                .iter()
                .map(|&i| pending[i].logical.clone())
                .collect();
            logicals.sort();
            logicals.dedup();
            let contribs = self
                .other_node_contributions_any(*tenant, *workspace, &logicals)
                .await?;

            // Collect candidate legacy doc ids across this scope's pending deletes.
            let mut legacy_candidates: Vec<Uuid> = Vec::new();
            for &i in indices {
                let item = &pending[i];
                let Some(excluding) = item.excluding_document else {
                    continue;
                };
                let Some(node) = existing.get(&item.node_id) else {
                    continue;
                };
                let contrib_docs: HashSet<Uuid> = contribs
                    .iter()
                    .filter(|(logical, doc, _)| logical == &item.logical && *doc != excluding)
                    .map(|(_, doc, _)| *doc)
                    .collect();
                for doc_s in document_ids_from_properties(&node.properties) {
                    if doc_s == excluding.to_string() {
                        continue;
                    }
                    if let Ok(doc_u) = Uuid::parse_str(&doc_s) {
                        if !contrib_docs.contains(&doc_u) {
                            legacy_candidates.push(doc_u);
                        }
                    }
                }
            }
            legacy_candidates.sort();
            legacy_candidates.dedup();
            let live_legacy = self
                .live_legacy_document_ids(*tenant, *workspace, &legacy_candidates)
                .await?;

            for &i in indices {
                let item = &pending[i];
                let Some(excluding) = item.excluding_document else {
                    node_deletes.push((
                        item.node_id.clone(),
                        item.tenant_s.clone(),
                        item.workspace_s.clone(),
                    ));
                    continue;
                };
                let others: Vec<(Uuid, Value)> = contribs
                    .iter()
                    .filter(|(logical, doc, _)| logical == &item.logical && *doc != excluding)
                    .map(|(_, doc, payload)| (*doc, payload.clone()))
                    .collect();
                let props = existing
                    .get(&item.node_id)
                    .map(|n| n.properties.clone())
                    .unwrap_or_default();
                match retained_lineage(&props, excluding, &others, &live_legacy) {
                    Some(kept) => {
                        let mut retained = props;
                        apply_retained_sources(&mut retained, &kept.chunks, &kept.documents);
                        node_retains.push((item.node_id.clone(), retained));
                    }
                    None => {
                        node_deletes.push((
                            item.node_id.clone(),
                            item.tenant_s.clone(),
                            item.workspace_s.clone(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    async fn resolve_edge_deletes(
        &self,
        pending: Vec<PendingEdgeDelete>,
        edge_deletes: &mut Vec<(String, String, String, String)>,
        edge_retains: &mut Vec<(String, String, HashMap<String, Value>)>,
    ) -> AccessResult<()> {
        use std::collections::HashSet;

        if pending.is_empty() {
            return Ok(());
        }

        let mut endpoint_ids: Vec<String> = Vec::new();
        for item in &pending {
            if item.excluding_document.is_some() {
                endpoint_ids.push(item.source.clone());
                endpoint_ids.push(item.target.clone());
            }
        }
        endpoint_ids.sort();
        endpoint_ids.dedup();
        let existing_edges = if endpoint_ids.is_empty() {
            Vec::new()
        } else {
            self.retain_reads.fetch_add(1, Ordering::Relaxed);
            self.graph
                .get_edges_for_nodes_batch(&endpoint_ids)
                .await
                .map_err(AccessError::from)?
        };

        let mut by_scope: HashMap<(Uuid, Uuid), Vec<usize>> = HashMap::new();
        for (idx, item) in pending.iter().enumerate() {
            by_scope
                .entry((item.tenant, item.workspace))
                .or_default()
                .push(idx);
        }

        for ((tenant, workspace), indices) in &by_scope {
            let mut pairs: Vec<(String, String)> = indices
                .iter()
                .map(|&i| {
                    (
                        pending[i].source_logical.clone(),
                        pending[i].target_logical.clone(),
                    )
                })
                .collect();
            pairs.sort();
            pairs.dedup();
            let contribs = self
                .other_edge_contributions_any(*tenant, *workspace, &pairs)
                .await?;

            let mut legacy_candidates: Vec<Uuid> = Vec::new();
            for &i in indices {
                let item = &pending[i];
                let Some(excluding) = item.excluding_document else {
                    continue;
                };
                let Some(edge) = existing_edges
                    .iter()
                    .find(|e| e.source == item.source && e.target == item.target)
                else {
                    continue;
                };
                let contrib_docs: HashSet<Uuid> = contribs
                    .iter()
                    .filter(|(src, tgt, doc, _)| {
                        src == &item.source_logical
                            && tgt == &item.target_logical
                            && *doc != excluding
                    })
                    .map(|(_, _, doc, _)| *doc)
                    .collect();
                for doc_s in document_ids_from_properties(&edge.properties) {
                    if doc_s == excluding.to_string() {
                        continue;
                    }
                    if let Ok(doc_u) = Uuid::parse_str(&doc_s) {
                        if !contrib_docs.contains(&doc_u) {
                            legacy_candidates.push(doc_u);
                        }
                    }
                }
            }
            legacy_candidates.sort();
            legacy_candidates.dedup();
            let live_legacy = self
                .live_legacy_document_ids(*tenant, *workspace, &legacy_candidates)
                .await?;

            for &i in indices {
                let item = &pending[i];
                let Some(excluding) = item.excluding_document else {
                    edge_deletes.push((
                        item.source.clone(),
                        item.target.clone(),
                        item.tenant_s.clone(),
                        item.workspace_s.clone(),
                    ));
                    continue;
                };
                let others: Vec<(Uuid, Value)> = contribs
                    .iter()
                    .filter(|(src, tgt, doc, _)| {
                        src == &item.source_logical
                            && tgt == &item.target_logical
                            && *doc != excluding
                    })
                    .map(|(_, _, doc, payload)| (*doc, payload.clone()))
                    .collect();
                let props = existing_edges
                    .iter()
                    .find(|e| e.source == item.source && e.target == item.target)
                    .map(|e| e.properties.clone())
                    .unwrap_or_default();
                match retained_lineage(&props, excluding, &others, &live_legacy) {
                    Some(kept) => {
                        let mut retained = props;
                        apply_retained_sources(&mut retained, &kept.chunks, &kept.documents);
                        edge_retains.push((item.source.clone(), item.target.clone(), retained));
                    }
                    None => {
                        edge_deletes.push((
                            item.source.clone(),
                            item.target.clone(),
                            item.tenant_s.clone(),
                            item.workspace_s.clone(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    async fn flush_graph_mutations(
        &self,
        mut node_upserts: HashMap<String, HashMap<String, Value>>,
        mut edge_upserts: HashMap<(String, String), HashMap<String, Value>>,
        node_deletes: Vec<(String, String, String)>,
        edge_deletes: Vec<(String, String, String, String)>,
        node_retains: Vec<(String, HashMap<String, Value>)>,
        edge_retains: Vec<(String, String, HashMap<String, Value>)>,
    ) -> AccessResult<()> {
        if !node_upserts.is_empty() {
            let ids: Vec<String> = node_upserts.keys().cloned().collect();
            let existing = self
                .graph
                .get_nodes_batch(&ids)
                .await
                .map_err(AccessError::from)?;
            for (id, node) in existing {
                if let Some(props) = node_upserts.get_mut(&id) {
                    union_source_properties(&node.properties, props);
                }
            }
            let batch: Vec<(String, HashMap<String, Value>)> = node_upserts.into_iter().collect();
            self.graph
                .upsert_nodes_batch(&batch)
                .await
                .map_err(AccessError::from)?;
            self.provider_batch_writes.fetch_add(1, Ordering::Relaxed);
        }

        if !edge_upserts.is_empty() {
            let endpoint_ids: Vec<String> = edge_upserts
                .keys()
                .flat_map(|(s, t)| [s.clone(), t.clone()])
                .collect();
            let existing_edges = self
                .graph
                .get_edges_for_nodes_batch(&endpoint_ids)
                .await
                .map_err(AccessError::from)?;
            for edge in existing_edges {
                let key = (edge.source.clone(), edge.target.clone());
                if let Some(props) = edge_upserts.get_mut(&key) {
                    union_source_properties(&edge.properties, props);
                }
            }
            let batch: Vec<(String, String, HashMap<String, Value>)> = edge_upserts
                .into_iter()
                .map(|((s, t), props)| (s, t, props))
                .collect();
            self.graph
                .upsert_edges_batch(&batch)
                .await
                .map_err(AccessError::from)?;
            self.provider_batch_writes.fetch_add(1, Ordering::Relaxed);
        }

        if !node_retains.is_empty() {
            self.graph
                .upsert_nodes_batch_with_mode(&node_retains, GraphPropertyWriteMode::Replace)
                .await
                .map_err(AccessError::from)?;
            self.provider_batch_writes.fetch_add(1, Ordering::Relaxed);
        }
        if !edge_retains.is_empty() {
            self.graph
                .upsert_edges_batch_with_mode(&edge_retains, GraphPropertyWriteMode::Replace)
                .await
                .map_err(AccessError::from)?;
            self.provider_batch_writes.fetch_add(1, Ordering::Relaxed);
        }

        if !node_deletes.is_empty() {
            // Group by (tenant, workspace) — one scoped batch statement per scope.
            let mut by_scope: HashMap<(String, String), Vec<String>> = HashMap::new();
            for (node_id, tenant_s, workspace_s) in node_deletes {
                by_scope
                    .entry((tenant_s, workspace_s))
                    .or_default()
                    .push(node_id);
            }
            for ((tenant_s, workspace_s), ids) in by_scope {
                let _ = self
                    .graph
                    .delete_nodes_scoped_batch(&ids, &tenant_s, &workspace_s)
                    .await
                    .map_err(AccessError::from)?;
                self.scoped_batch_deletes.fetch_add(1, Ordering::Relaxed);
                self.provider_batch_writes.fetch_add(1, Ordering::Relaxed);
            }
        }
        if !edge_deletes.is_empty() {
            let mut by_scope: HashMap<(String, String), Vec<(String, String)>> = HashMap::new();
            for (source, target, tenant_s, workspace_s) in edge_deletes {
                by_scope
                    .entry((tenant_s, workspace_s))
                    .or_default()
                    .push((source, target));
            }
            for ((tenant_s, workspace_s), pairs) in by_scope {
                let _ = self
                    .graph
                    .delete_edges_scoped_batch(&pairs, &tenant_s, &workspace_s)
                    .await
                    .map_err(AccessError::from)?;
                self.scoped_batch_deletes.fetch_add(1, Ordering::Relaxed);
                self.provider_batch_writes.fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl GraphProjectionApplier for AgeGraphProjectionApplier {
    async fn apply_batch(
        &self,
        items: &[(&ProjectionEvent, &DataBindingDescriptor)],
    ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
        self.apply_batch_entries.fetch_add(1, Ordering::Relaxed);
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let mut node_upserts: HashMap<String, HashMap<String, Value>> = HashMap::new();
        let mut edge_upserts: HashMap<(String, String), HashMap<String, Value>> = HashMap::new();
        let mut pending_node_deletes = Vec::new();
        let mut pending_edge_deletes = Vec::new();
        let mut cleanup_marks: Vec<(Uuid, Uuid)> = Vec::new();
        let mut receipts = Vec::with_capacity(items.len());

        let hydrate_event_ids: Vec<Uuid> = items
            .iter()
            .filter(|(event, _)| {
                matches!(
                    (event.object_kind.as_str(), event.operation),
                    ("document_batch", ProjectionOperation::Upsert)
                        | ("document", ProjectionOperation::Delete)
                )
            })
            .map(|(event, _)| event.event_id)
            .collect();
        let hydrated_facts = self.load_batch_facts_many(&hydrate_event_ids).await?;

        let mut fact_keys = Vec::new();
        for &(event, _) in items {
            if event.object_kind == "fact" {
                let revision = i64::try_from(event.object_revision).map_err(|_| {
                    AccessError::CorruptData("fact projection revision exceeds i64".into())
                })?;
                fact_keys.push((
                    event.event_id,
                    event.scope.tenant().into_uuid(),
                    event.scope.workspace().into_uuid(),
                    event.object_id,
                    revision,
                ));
            }
        }
        let fact_revisions = self.load_fact_revisions_many(&fact_keys).await?;

        for &(event, binding) in items {
            binding.require_graph()?;
            binding.require_provider("age")?;
            match (&*event.object_kind, event.operation) {
                ("document_batch", ProjectionOperation::Upsert) => {
                    let (payloads, manifest_items) = hydrated_facts
                        .get(&event.event_id)
                        .cloned()
                        .ok_or_else(|| {
                        AccessError::CorruptData("missing graph facts for event".into())
                    })?;
                    for payload in &payloads {
                        let schema =
                            payload
                                .get("schema")
                                .and_then(Value::as_str)
                                .ok_or_else(|| {
                                    AccessError::CorruptData("graph fact has no schema".into())
                                })?;
                        if schema != "edgequake.graph.fact.v1" {
                            return Err(AccessError::CorruptData(format!(
                                "unknown graph fact schema '{schema}'"
                            )));
                        }
                        let kind =
                            payload.get("kind").and_then(Value::as_str).ok_or_else(|| {
                                AccessError::CorruptData("graph fact has no kind".into())
                            })?;
                        match kind {
                            "node" => Self::plan_upsert_node(event, payload, &mut node_upserts)?,
                            "edge" => Self::plan_upsert_edge(event, payload, &mut edge_upserts)?,
                            unknown => {
                                return Err(AccessError::CorruptData(format!(
                                    "unknown graph fact kind '{unknown}'"
                                )));
                            }
                        }
                    }
                    receipts.push(receipt(
                        "age",
                        event,
                        binding.binding_id,
                        role_completion_proof(&manifest_items),
                    ));
                }
                ("document", ProjectionOperation::Delete) => {
                    let (payloads, manifest_items) = hydrated_facts
                        .get(&event.event_id)
                        .cloned()
                        .ok_or_else(|| {
                        AccessError::CorruptData("missing graph facts for delete event".into())
                    })?;
                    for payload in &payloads {
                        let kind =
                            payload.get("kind").and_then(Value::as_str).ok_or_else(|| {
                                AccessError::CorruptData("graph fact has no kind".into())
                            })?;
                        match kind {
                            "node" => {
                                Self::collect_delete_node(
                                    event,
                                    payload,
                                    &mut pending_node_deletes,
                                )?;
                            }
                            "edge" => {
                                Self::collect_delete_edge(
                                    event,
                                    payload,
                                    &mut pending_edge_deletes,
                                )?;
                            }
                            unknown => {
                                return Err(AccessError::CorruptData(format!(
                                    "unknown graph fact kind '{unknown}'"
                                )));
                            }
                        }
                    }
                    cleanup_marks.push((event.object_id, binding.binding_id));
                    receipts.push(receipt(
                        "age",
                        event,
                        binding.binding_id,
                        role_completion_proof(&manifest_items),
                    ));
                }
                ("fact", operation) => {
                    let revision = i64::try_from(event.object_revision).map_err(|_| {
                        AccessError::CorruptData("fact projection revision exceeds i64".into())
                    })?;
                    let payload_bytes = fact_revisions
                        .get(&event.event_id)
                        .ok_or_else(|| AccessError::CorruptData("fact payload is absent".into()))?;
                    let payload = serde_json::from_slice::<Value>(payload_bytes)
                        .map_err(|error| AccessError::CorruptData(error.to_string()))?;
                    let kind = payload
                        .get("kind")
                        .and_then(Value::as_str)
                        .ok_or_else(|| AccessError::CorruptData("graph fact has no kind".into()))?;
                    match (kind, operation) {
                        ("node", ProjectionOperation::Upsert) => {
                            Self::plan_upsert_node(event, &payload, &mut node_upserts)?;
                        }
                        ("edge", ProjectionOperation::Upsert) => {
                            Self::plan_upsert_edge(event, &payload, &mut edge_upserts)?;
                        }
                        ("node", ProjectionOperation::Delete) => {
                            Self::collect_delete_node(event, &payload, &mut pending_node_deletes)?;
                        }
                        ("edge", ProjectionOperation::Delete) => {
                            Self::collect_delete_edge(event, &payload, &mut pending_edge_deletes)?;
                        }
                        (unknown, _) => {
                            return Err(AccessError::CorruptData(format!(
                                "unknown graph fact kind '{unknown}'"
                            )));
                        }
                    }
                    let item = EventManifestItem {
                        role: "graph".into(),
                        item_kind: "fact".into(),
                        record_id: event.object_id,
                        record_revision: revision,
                        digest: edgequake_storage_contracts::payload_digest(payload_bytes),
                        logical_key: event.object_id.to_string(),
                    };
                    receipts.push(receipt(
                        "age",
                        event,
                        binding.binding_id,
                        role_completion_proof(&[item]),
                    ));
                }
                (kind, _) => {
                    return Err(AccessError::CorruptData(format!(
                        "unknown graph projection object kind '{kind}'"
                    )));
                }
            }
        }

        let mut node_deletes = Vec::new();
        let mut edge_deletes = Vec::new();
        let mut node_retains = Vec::new();
        let mut edge_retains = Vec::new();
        self.resolve_node_deletes(pending_node_deletes, &mut node_deletes, &mut node_retains)
            .await?;
        self.resolve_edge_deletes(pending_edge_deletes, &mut edge_deletes, &mut edge_retains)
            .await?;

        self.flush_graph_mutations(
            node_upserts,
            edge_upserts,
            node_deletes,
            edge_deletes,
            node_retains,
            edge_retains,
        )
        .await?;

        if !cleanup_marks.is_empty() {
            mark_cleanups_applied(&self.pool, &cleanup_marks).await?;
            self.cleanup_statements.fetch_add(1, Ordering::Relaxed);
        }

        Ok(receipts)
    }
}

/// Typed pgvector projection applier.
pub struct PgvectorProjectionApplier {
    pub chunk_index: Arc<dyn EmbeddingIndex>,
    pub fleet: Option<Arc<dyn FleetEmbeddingIndex>>,
    pool: PgPool,
    apply_batch_entries: AtomicU64,
    provider_batch_writes: AtomicU64,
    hydration_queries: AtomicU64,
    delete_statements: AtomicU64,
    cleanup_statements: AtomicU64,
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
            apply_batch_entries: AtomicU64::new(0),
            provider_batch_writes: AtomicU64::new(0),
            hydration_queries: AtomicU64::new(0),
            delete_statements: AtomicU64::new(0),
            cleanup_statements: AtomicU64::new(0),
        }
    }

    pub fn apply_batch_entries(&self) -> u64 {
        self.apply_batch_entries.load(Ordering::Relaxed)
    }

    pub fn provider_batch_writes(&self) -> u64 {
        self.provider_batch_writes.load(Ordering::Relaxed)
    }

    pub fn hydration_queries(&self) -> u64 {
        self.hydration_queries.load(Ordering::Relaxed)
    }

    pub fn delete_statements(&self) -> u64 {
        self.delete_statements.load(Ordering::Relaxed)
    }

    pub fn cleanup_statements(&self) -> u64 {
        self.cleanup_statements.load(Ordering::Relaxed)
    }

    /// One hydration statement for many document_batch upsert events.
    async fn load_embeddings_many(
        &self,
        event_ids: &[Uuid],
    ) -> AccessResult<HashMap<Uuid, (Vec<EmbeddingPayload>, Vec<EventManifestItem>)>> {
        if event_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows =
            sqlx::query_as::<_, (Uuid, String, Uuid, i64, Vec<u8>, String, Option<Vec<u8>>)>(
                "SELECT i.event_id, i.item_kind, i.record_id, i.record_revision, i.digest, \
                    i.logical_key, m.payload \
             FROM public.projection_event_items i \
             JOIN public.projection_events e ON e.event_id = i.event_id \
             LEFT JOIN public.embedding_manifests m \
               ON m.tenant_id = e.tenant_id AND m.workspace_id = e.workspace_id \
              AND m.subject_id = i.record_id \
              AND m.content_revision = i.record_revision \
              AND m.digest = i.digest \
             WHERE i.event_id = ANY($1) AND i.role = 'vector' \
             ORDER BY i.event_id, i.ordinal",
            )
            .bind(event_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)?;
        self.hydration_queries.fetch_add(1, Ordering::Relaxed);

        let mut out: HashMap<Uuid, (Vec<EmbeddingPayload>, Vec<EventManifestItem>)> =
            HashMap::new();
        for event_id in event_ids {
            out.entry(*event_id)
                .or_insert_with(|| (Vec::new(), Vec::new()));
        }
        for (event_id, item_kind, record_id, record_revision, digest, logical_key, payload) in rows
        {
            let payload = payload.ok_or_else(|| {
                AccessError::CorruptData(
                    "vector manifest membership does not match stored embeddings".into(),
                )
            })?;
            let parsed = serde_json::from_slice::<EmbeddingPayload>(&payload).map_err(|error| {
                AccessError::CorruptData(format!("invalid embedding payload: {error}"))
            })?;
            let digest: [u8; 32] = digest.try_into().map_err(|_| {
                AccessError::CorruptData("projection manifest digest is not 32 bytes".into())
            })?;
            let entry = out.entry(event_id).or_default();
            entry.0.push(parsed);
            entry.1.push(EventManifestItem {
                role: "vector".into(),
                item_kind,
                record_id,
                record_revision,
                digest,
                logical_key,
            });
        }
        Ok(out)
    }

    async fn upsert_payloads(
        &self,
        workspace: Uuid,
        payloads: Vec<EmbeddingPayload>,
    ) -> AccessResult<()> {
        let mut chunk_rows = Vec::new();
        let mut fleet_rows: HashMap<&'static str, Vec<FleetEmbeddingRow>> = HashMap::new();
        for payload in payloads {
            payload.validate()?;
            let workspace_id = WorkspaceId::new(workspace);
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
        let mut wrote = false;
        if !chunk_rows.is_empty() {
            self.chunk_index
                .upsert_batch(ModelId(Uuid::nil()), &chunk_rows)
                .await
                .map_err(AccessError::from)?;
            wrote = true;
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
                wrote = true;
            }
        }
        if wrote {
            self.provider_batch_writes.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    /// One manifest read plus one delete statement per table per workspace.
    async fn delete_embeddings_many(
        &self,
        events: &[&ProjectionEvent],
    ) -> AccessResult<HashMap<Uuid, Vec<EventManifestItem>>> {
        let mut out: HashMap<Uuid, Vec<EventManifestItem>> = HashMap::new();
        if events.is_empty() {
            return Ok(out);
        }
        let event_ids: Vec<Uuid> = events.iter().map(|event| event.event_id).collect();
        for event in events {
            out.insert(event.event_id, Vec::new());
        }
        let rows = sqlx::query_as::<_, (Uuid, String, Uuid, i64, Vec<u8>, String)>(
            "SELECT event_id, item_kind, record_id, record_revision, digest, logical_key \
             FROM public.projection_event_items \
             WHERE event_id = ANY($1) AND role = 'vector' \
             ORDER BY event_id, ordinal",
        )
        .bind(&event_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;
        self.delete_statements.fetch_add(1, Ordering::Relaxed);

        let mut subjects_by_scope: HashMap<(Uuid, Uuid), Vec<Uuid>> = HashMap::new();
        let scope_of: HashMap<Uuid, (Uuid, Uuid)> = events
            .iter()
            .map(|event| {
                (
                    event.event_id,
                    (
                        event.scope.tenant().into_uuid(),
                        event.scope.workspace().into_uuid(),
                    ),
                )
            })
            .collect();
        for (event_id, item_kind, record_id, record_revision, digest, logical_key) in rows {
            let digest: [u8; 32] = digest.try_into().map_err(|_| {
                AccessError::CorruptData("projection manifest digest is not 32 bytes".into())
            })?;
            if let Some(scope) = scope_of.get(&event_id) {
                subjects_by_scope.entry(*scope).or_default().push(record_id);
            }
            out.entry(event_id).or_default().push(EventManifestItem {
                role: "vector".into(),
                item_kind,
                record_id,
                record_revision,
                digest,
                logical_key,
            });
        }

        for ((tenant, workspace), subjects) in subjects_by_scope {
            if subjects.is_empty() {
                continue;
            }
            sqlx::query(
                "DELETE FROM public.chunk_embeddings \
                 WHERE chunk_id = ANY($1) AND workspace_id = $2",
            )
            .bind(&subjects)
            .bind(workspace)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
            self.delete_statements.fetch_add(1, Ordering::Relaxed);
            sqlx::query(
                "DELETE FROM public.embedding_manifests \
                 WHERE tenant_id = $1 AND workspace_id = $2 AND subject_id = ANY($3)",
            )
            .bind(tenant)
            .bind(workspace)
            .bind(&subjects)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
            self.delete_statements.fetch_add(1, Ordering::Relaxed);
            self.provider_batch_writes.fetch_add(1, Ordering::Relaxed);
        }
        Ok(out)
    }
}

#[async_trait]
impl VectorProjectionApplier for PgvectorProjectionApplier {
    async fn apply_batch(
        &self,
        items: &[(&ProjectionEvent, &DataBindingDescriptor)],
    ) -> AccessResult<Vec<ProjectionApplyReceipt>> {
        self.apply_batch_entries.fetch_add(1, Ordering::Relaxed);
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let mut receipts = Vec::with_capacity(items.len());
        let mut upsert_by_workspace: HashMap<Uuid, Vec<EmbeddingPayload>> = HashMap::new();
        let mut cleanup_marks: Vec<(Uuid, Uuid)> = Vec::new();

        let upsert_event_ids: Vec<Uuid> = items
            .iter()
            .filter(|(event, _)| {
                event.object_kind == "document_batch"
                    && event.operation == ProjectionOperation::Upsert
            })
            .map(|(event, _)| event.event_id)
            .collect();
        let upsert_embeddings = self.load_embeddings_many(&upsert_event_ids).await?;
        let delete_events: Vec<&ProjectionEvent> = items
            .iter()
            .filter(|(event, _)| {
                event.object_kind == "document" && event.operation == ProjectionOperation::Delete
            })
            .map(|(event, _)| *event)
            .collect();
        let deleted_embeddings = self.delete_embeddings_many(&delete_events).await?;

        for &(event, binding) in items {
            binding.require_vector()?;
            binding.require_provider("pgvector")?;
            match (&*event.object_kind, event.operation) {
                ("document_batch", ProjectionOperation::Upsert) => {
                    let (payloads, manifest_items) = upsert_embeddings
                        .get(&event.event_id)
                        .cloned()
                        .ok_or_else(|| {
                            AccessError::CorruptData("missing vector embeddings for event".into())
                        })?;
                    upsert_by_workspace
                        .entry(event.scope.workspace().into_uuid())
                        .or_default()
                        .extend(payloads);
                    receipts.push(receipt(
                        "pgvector",
                        event,
                        binding.binding_id,
                        role_completion_proof(&manifest_items),
                    ));
                }
                ("document", ProjectionOperation::Delete) => {
                    let manifest_items = deleted_embeddings
                        .get(&event.event_id)
                        .cloned()
                        .unwrap_or_default();
                    cleanup_marks.push((event.object_id, binding.binding_id));
                    receipts.push(receipt(
                        "pgvector",
                        event,
                        binding.binding_id,
                        role_completion_proof(&manifest_items),
                    ));
                }
                (kind, _) => {
                    return Err(AccessError::CorruptData(format!(
                        "unknown vector projection object kind '{kind}'"
                    )));
                }
            }
        }

        for (workspace, payloads) in upsert_by_workspace {
            if !payloads.is_empty() {
                self.upsert_payloads(workspace, payloads).await?;
            }
        }
        if !cleanup_marks.is_empty() {
            mark_cleanups_applied(&self.pool, &cleanup_marks).await?;
            self.cleanup_statements.fetch_add(1, Ordering::Relaxed);
        }
        Ok(receipts)
    }
}

#[derive(Debug, Clone, Deserialize)]
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

fn excluding_document_id(
    event: &ProjectionEvent,
    operation: ProjectionOperation,
    payload: &Value,
) -> Option<Uuid> {
    if operation != ProjectionOperation::Delete {
        return None;
    }
    if event.object_kind == "document" {
        return Some(event.object_id);
    }
    payload
        .pointer("/properties/source_document_id")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
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

fn receipt(
    provider: &str,
    event: &ProjectionEvent,
    binding_id: Uuid,
    completion_proof: [u8; 32],
) -> ProjectionApplyReceipt {
    ProjectionApplyReceipt {
        provider_receipt: format!("{provider}:{}:{binding_id}", event.event_id),
        completion_proof: completion_proof.to_vec(),
    }
}

async fn mark_cleanups_applied(pool: &PgPool, marks: &[(Uuid, Uuid)]) -> AccessResult<()> {
    if marks.is_empty() {
        return Ok(());
    }
    let document_ids: Vec<Uuid> = marks.iter().map(|(document_id, _)| *document_id).collect();
    let binding_ids: Vec<Uuid> = marks.iter().map(|(_, binding_id)| *binding_id).collect();
    sqlx::query(
        "UPDATE public.projection_cleanup_intents \
         SET state = 'applied' \
         WHERE (document_id, binding_id) IN ( \
             SELECT * FROM UNNEST($1::uuid[], $2::uuid[]) AS t(document_id, binding_id) \
         )",
    )
    .bind(&document_ids)
    .bind(&binding_ids)
    .execute(pool)
    .await
    .map_err(database_error)?;
    Ok(())
}

fn database_error(error: sqlx::Error) -> AccessError {
    AccessError::from(crate::StorageError::from(error))
}
