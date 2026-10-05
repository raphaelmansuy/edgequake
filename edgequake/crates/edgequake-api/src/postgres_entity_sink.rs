//! PostgreSQL implementation of `RelationalEntitySink` (SPEC-021 P3-01/P3-02).
//!
//! Under `typed_embeddings`, this sink is always enabled (fleet embeddings FK to
//! `entities` / `relationships`). Entity names are stored **bare** (workspace
//! isolation is the `workspace_id` column) so fleet mirror lookups by
//! `entity:NAME` resolve.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use edgequake_pipeline::{
    EntitySinkRow, NoopEntitySink, RelationalEntitySink, RelationshipSinkReport,
    RelationshipSinkRow,
};
use edgequake_storage::EntityId;
use sqlx::PgPool;
use tracing::{debug, info, warn};

mod queries;

/// PostgreSQL-backed relational entity (and relationship) sink.
pub struct PostgresEntitySink {
    pool: Arc<PgPool>,
    /// When true (typed vector backend), SQL failures fail closed.
    fail_closed: bool,
}

fn scope_uuid(value: Option<&str>, field: &str) -> edgequake_pipeline::Result<Option<uuid::Uuid>> {
    value.map(str::parse).transpose().map_err(|_| {
        edgequake_pipeline::PipelineError::StorageError(
            edgequake_storage::StorageError::InvalidInput(format!("invalid {field} UUID")),
        )
    })
}

impl PostgresEntitySink {
    /// Create a fail-open sink (legacy CQRS dual-write mode).
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            fail_closed: false,
        }
    }

    /// Create a fail-closed sink required for typed fleet FK spine.
    pub fn new_fail_closed(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            fail_closed: true,
        }
    }

    /// Resolve sink for the current vector backend + entity_sync_mode.
    ///
    /// - `typed_embeddings` / `chunk_embeddings`: always `PostgresEntitySink`
    ///   (fail-closed) so fleet mirror has relational FKs.
    /// - otherwise: honor `entity_sync_mode` dual_write|full (fail-open).
    pub async fn create_for_runtime(pool: Arc<PgPool>) -> Arc<dyn RelationalEntitySink> {
        if edgequake_storage::vector_backend_reads_typed(
            edgequake_storage::vector_backend_from_env(),
        ) {
            info!("typed vector backend: forcing PostgresEntitySink (fleet FK spine, fail-closed)");
            return Arc::new(Self::new_fail_closed(pool));
        }
        Self::create_if_enabled(pool).await
    }

    /// Create the appropriate sink based on `entity_sync_mode` in server_config.
    ///
    /// Returns:
    /// - `PostgresEntitySink` when mode is `dual_write` or `full`
    /// - `NoopEntitySink` when mode is `disabled` or config is absent
    pub async fn create_if_enabled(pool: Arc<PgPool>) -> Arc<dyn RelationalEntitySink> {
        let mode: Option<String> = sqlx::query_scalar(
            "SELECT value::text FROM server_config WHERE key = 'entity_sync_mode'",
        )
        .fetch_optional(pool.as_ref())
        .await
        .unwrap_or(None);

        let mode_str = mode.as_deref().unwrap_or("\"disabled\"");
        let enabled = mode_str.contains("dual_write") || mode_str.contains("full");

        if enabled {
            tracing::info!(
                entity_sync_mode = %mode_str,
                "CQRS entity dual-write ENABLED (SPEC-021 P3-01)"
            );
            Arc::new(Self::new(pool))
        } else {
            tracing::info!(
                entity_sync_mode = %mode_str,
                "CQRS entity dual-write disabled (entity_sync_mode != dual_write|full)"
            );
            Arc::new(NoopEntitySink)
        }
    }

    /// Bare names take precedence over the legacy workspace-prefixed form.
    async fn resolve_entity_id(
        &self,
        name: &str,
        tenant_id: Option<uuid::Uuid>,
        workspace_id: Option<uuid::Uuid>,
    ) -> edgequake_pipeline::Result<Option<uuid::Uuid>> {
        let sql = queries::entity_lookup_sql(workspace_id.is_some(), tenant_id.is_some());
        sqlx::query_scalar(&sql)
            .bind(name)
            .bind(workspace_id)
            .bind(tenant_id)
            .fetch_optional(self.pool.as_ref())
            .await
            .map_err(|e| edgequake_pipeline::PipelineError::StorageError(e.into()))
    }

    fn map_sql_result(
        &self,
        label: &str,
        result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>,
    ) -> edgequake_pipeline::Result<()> {
        match result {
            Ok(_) => {
                debug!(target = %label, "Relational sink OK");
                Ok(())
            }
            Err(e) => {
                if self.fail_closed {
                    return Err(edgequake_pipeline::PipelineError::StorageError(e.into()));
                }
                warn!(target = %label, error = %e, "Relational sink failed (best-effort)");
                Ok(())
            }
        }
    }
}

#[async_trait]
impl RelationalEntitySink for PostgresEntitySink {
    async fn upsert_entity(
        &self,
        name: &str,
        entity_type: &str,
        description: &str,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
        source_chunk_ids: &[String],
    ) -> edgequake_pipeline::Result<()> {
        let bare = EntityId::bare_name_from_graph_node_id(name);
        if bare.is_empty() {
            return Ok(());
        }
        let tenant_uuid = scope_uuid(tenant_id, "tenant_id")?;
        let workspace_uuid = scope_uuid(workspace_id, "workspace_id")?;

        let result = sqlx::query(
            r#"INSERT INTO entities
                   (name, entity_type, description, tenant_id, workspace_id,
                    source_chunk_ids, sync_status, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, 'synced', NOW(), NOW())
               ON CONFLICT (tenant_id, workspace_id, name) DO UPDATE SET
                   entity_type      = EXCLUDED.entity_type,
                   description      = EXCLUDED.description,
                   source_chunk_ids = (
                       SELECT array_agg(DISTINCT elem)
                       FROM unnest(entities.source_chunk_ids || EXCLUDED.source_chunk_ids) AS t(elem)
                   ),
                   sync_status = 'synced',
                   updated_at  = NOW()"#,
        )
        .bind(bare)
        .bind(entity_type)
        .bind(description)
        .bind(tenant_uuid)
        .bind(workspace_uuid)
        .bind(source_chunk_ids)
        .execute(self.pool.as_ref())
        .await;

        self.map_sql_result(bare, result)
    }

    /// SPEC-091 IP1: one UNNEST upsert for the whole entity batch (LAW-IP2).
    async fn upsert_entities_batch(
        &self,
        rows: &[EntitySinkRow],
    ) -> edgequake_pipeline::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        // Collapse duplicate (tenant, workspace, name) keys — Postgres rejects
        // "ON CONFLICT DO UPDATE cannot affect row a second time" in one INSERT.
        let mut by_key: HashMap<(Option<uuid::Uuid>, Option<uuid::Uuid>, String), EntitySinkRow> =
            HashMap::new();
        for row in rows {
            let bare = EntityId::bare_name_from_graph_node_id(&row.name);
            if bare.is_empty() {
                continue;
            }
            let tenant = scope_uuid(row.tenant_id.as_deref(), "tenant_id")?;
            let workspace = scope_uuid(row.workspace_id.as_deref(), "workspace_id")?;
            let key = (tenant, workspace, bare.to_string());
            by_key
                .entry(key)
                .and_modify(|existing| {
                    existing.entity_type = row.entity_type.clone();
                    existing.description = row.description.clone();
                    for s in &row.source_chunk_ids {
                        if !existing.source_chunk_ids.contains(s) {
                            existing.source_chunk_ids.push(s.clone());
                        }
                    }
                })
                .or_insert_with(|| EntitySinkRow {
                    name: bare.to_string(),
                    entity_type: row.entity_type.clone(),
                    description: row.description.clone(),
                    tenant_id: row.tenant_id.clone(),
                    workspace_id: row.workspace_id.clone(),
                    source_chunk_ids: row.source_chunk_ids.clone(),
                });
        }
        if by_key.is_empty() {
            return Ok(());
        }

        let mut names: Vec<String> = Vec::with_capacity(by_key.len());
        let mut types: Vec<String> = Vec::with_capacity(by_key.len());
        let mut descs: Vec<String> = Vec::with_capacity(by_key.len());
        let mut tenants: Vec<Option<uuid::Uuid>> = Vec::with_capacity(by_key.len());
        let mut workspaces: Vec<Option<uuid::Uuid>> = Vec::with_capacity(by_key.len());
        let mut sources_json: Vec<serde_json::Value> = Vec::with_capacity(by_key.len());
        for ((tenant, workspace, name), row) in by_key {
            names.push(name);
            types.push(row.entity_type);
            descs.push(row.description);
            tenants.push(tenant);
            workspaces.push(workspace);
            sources_json.push(serde_json::json!(row.source_chunk_ids));
        }

        let result = sqlx::query(
            r#"
            INSERT INTO entities
                (name, entity_type, description, tenant_id, workspace_id,
                 source_chunk_ids, sync_status, created_at, updated_at)
            SELECT
                n, t, d, tn, ws,
                COALESCE(
                    (SELECT array_agg(x) FROM jsonb_array_elements_text(src) AS x),
                    '{}'::text[]
                ),
                'synced', NOW(), NOW()
            FROM unnest(
                $1::text[],
                $2::text[],
                $3::text[],
                $4::uuid[],
                $5::uuid[],
                $6::jsonb[]
            ) AS u(n, t, d, tn, ws, src)
            ON CONFLICT (tenant_id, workspace_id, name) DO UPDATE SET
                entity_type      = EXCLUDED.entity_type,
                description      = EXCLUDED.description,
                source_chunk_ids = (
                    SELECT array_agg(DISTINCT elem)
                    FROM unnest(entities.source_chunk_ids || EXCLUDED.source_chunk_ids) AS t(elem)
                ),
                sync_status = 'synced',
                updated_at  = NOW()
            "#,
        )
        .bind(&names)
        .bind(&types)
        .bind(&descs)
        .bind(&tenants)
        .bind(&workspaces)
        .bind(&sources_json)
        .execute(self.pool.as_ref())
        .await;

        self.map_sql_result("entities_batch", result)
    }

    async fn upsert_relationship(
        &self,
        source_name: &str,
        target_name: &str,
        relation_type: &str,
        description: &str,
        weight: f32,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> edgequake_pipeline::Result<()> {
        let src = EntityId::bare_name_from_graph_node_id(source_name);
        let tgt = EntityId::bare_name_from_graph_node_id(target_name);
        if src.is_empty() || tgt.is_empty() {
            return Ok(());
        }
        let rel_type = edgequake_storage::normalize_relation_type_str(relation_type);
        let tenant_uuid = scope_uuid(tenant_id, "tenant_id")?;
        let workspace_uuid = scope_uuid(workspace_id, "workspace_id")?;

        let src_id = self
            .resolve_entity_id(src, tenant_uuid, workspace_uuid)
            .await?;
        let tgt_id = self
            .resolve_entity_id(tgt, tenant_uuid, workspace_uuid)
            .await?;

        let (Some(source_id), Some(target_id)) = (src_id, tgt_id) else {
            let msg = format!(
                "relational relationship upsert skipped: missing entity FK \
                 src={src} tgt={tgt} workspace={workspace_uuid:?}"
            );
            if self.fail_closed {
                return Err(edgequake_pipeline::PipelineError::StorageError(
                    edgequake_storage::error::StorageError::Database(msg),
                ));
            }
            warn!("{msg}");
            return Ok(());
        };

        let result = sqlx::query(
            r#"INSERT INTO relationships
                   (source_id, target_id, tenant_id, workspace_id, relation_type,
                    description, weight, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
               ON CONFLICT (tenant_id, workspace_id, source_id, target_id, relation_type)
               DO UPDATE SET
                   description = EXCLUDED.description,
                   weight = EXCLUDED.weight,
                   updated_at = NOW()"#,
        )
        .bind(source_id)
        .bind(target_id)
        .bind(tenant_uuid)
        .bind(workspace_uuid)
        .bind(&rel_type)
        .bind(description)
        .bind(weight)
        .execute(self.pool.as_ref())
        .await;

        self.map_sql_result(&format!("{src}->{tgt}:{rel_type}"), result)
    }

    /// SPEC-091 IP1: resolve endpoints once, then one UNNEST relationship upsert.
    /// SPEC-130: RETURNING ids keyed by legacy `SRC->TGT:TYPE` for RelVectors mirror.
    async fn upsert_relationships_batch(
        &self,
        rows: &[RelationshipSinkRow],
    ) -> edgequake_pipeline::Result<RelationshipSinkReport> {
        if rows.is_empty() {
            return Ok(RelationshipSinkReport::default());
        }

        // LAW-098-8: collapse duplicate arbiter keys before ON CONFLICT DO UPDATE
        // (parity with upsert_entities_batch — Postgres rejects affecting a row twice).
        let mut by_key: HashMap<
            (
                Option<uuid::Uuid>,
                Option<uuid::Uuid>,
                String,
                String,
                String,
            ),
            RelationshipSinkRow,
        > = HashMap::new();
        let mut batch_scope = None;
        for row in rows {
            let src = EntityId::bare_name_from_graph_node_id(&row.source_name);
            let tgt = EntityId::bare_name_from_graph_node_id(&row.target_name);
            if src.is_empty() || tgt.is_empty() {
                continue;
            }
            let tenant = scope_uuid(row.tenant_id.as_deref(), "tenant_id")?;
            let workspace = scope_uuid(row.workspace_id.as_deref(), "workspace_id")?;
            let scope = (tenant, workspace);
            if batch_scope.is_some_and(|expected| expected != scope) {
                return Err(edgequake_pipeline::PipelineError::StorageError(
                    edgequake_storage::StorageError::InvalidInput(
                        "relationship batch must use one tenant/workspace scope".into(),
                    ),
                ));
            }
            batch_scope = Some(scope);
            let rel_type = edgequake_storage::normalize_relation_type_str(&row.relation_type);
            let key = (
                tenant,
                workspace,
                src.to_string(),
                tgt.to_string(),
                rel_type.clone(),
            );
            by_key
                .entry(key)
                .and_modify(|existing| {
                    existing.description = row.description.clone();
                    existing.weight = row.weight;
                    existing.relation_type = rel_type.clone();
                })
                .or_insert_with(|| RelationshipSinkRow {
                    source_name: src.to_string(),
                    target_name: tgt.to_string(),
                    relation_type: rel_type,
                    description: row.description.clone(),
                    weight: row.weight,
                    tenant_id: row.tenant_id.clone(),
                    workspace_id: row.workspace_id.clone(),
                });
        }
        if by_key.is_empty() {
            return Ok(RelationshipSinkReport::default());
        }
        let rows: Vec<RelationshipSinkRow> = by_key.into_values().collect();

        // Scope is validated before deduplication and before any SQL execution.
        let (tenant_uuid, workspace_uuid) = batch_scope.expect("nonempty batch has a scope");

        let mut bare_names: Vec<String> = Vec::new();
        for row in &rows {
            let src = EntityId::bare_name_from_graph_node_id(&row.source_name);
            let tgt = EntityId::bare_name_from_graph_node_id(&row.target_name);
            if !src.is_empty() {
                bare_names.push(src.to_string());
            }
            if !tgt.is_empty() {
                bare_names.push(tgt.to_string());
            }
        }
        bare_names.sort();
        bare_names.dedup();

        let sql = queries::entity_batch_lookup_sql(workspace_uuid.is_some(), tenant_uuid.is_some());
        let id_rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(&sql)
            .bind(&bare_names)
            .bind(workspace_uuid)
            .bind(tenant_uuid)
            .fetch_all(self.pool.as_ref())
            .await
            .map_err(|e| edgequake_pipeline::PipelineError::StorageError(e.into()))?;

        let mut by_name: HashMap<String, uuid::Uuid> = HashMap::new();
        for (id, name) in id_rows {
            by_name.insert(name, id);
        }

        let mut source_ids: Vec<uuid::Uuid> = Vec::new();
        let mut target_ids: Vec<uuid::Uuid> = Vec::new();
        let mut tenants: Vec<Option<uuid::Uuid>> = Vec::new();
        let mut workspaces: Vec<Option<uuid::Uuid>> = Vec::new();
        let mut rel_types: Vec<String> = Vec::new();
        let mut descs: Vec<String> = Vec::new();
        let mut weights: Vec<f32> = Vec::new();
        let mut missing = 0u64;

        for row in &rows {
            let src = EntityId::bare_name_from_graph_node_id(&row.source_name);
            let tgt = EntityId::bare_name_from_graph_node_id(&row.target_name);
            if src.is_empty() || tgt.is_empty() {
                continue;
            }
            let (Some(source_id), Some(target_id)) = (by_name.get(src), by_name.get(tgt)) else {
                missing += 1;
                continue;
            };
            let rel_type = edgequake_storage::normalize_relation_type_str(&row.relation_type);
            source_ids.push(*source_id);
            target_ids.push(*target_id);
            tenants.push(tenant_uuid);
            workspaces.push(workspace_uuid);
            rel_types.push(rel_type);
            descs.push(row.description.clone());
            weights.push(row.weight);
        }

        if missing > 0 {
            let msg = format!(
                "relational relationship batch: {missing} row(s) missing entity FK \
                 workspace={workspace_uuid:?}"
            );
            if self.fail_closed {
                return Err(edgequake_pipeline::PipelineError::StorageError(
                    edgequake_storage::error::StorageError::Database(msg),
                ));
            }
            warn!("{msg}");
        }

        if source_ids.is_empty() {
            return Ok(RelationshipSinkReport {
                ids: HashMap::new(),
                missing_fk: missing,
            });
        }

        let returned: Result<Vec<(uuid::Uuid, uuid::Uuid, uuid::Uuid, String)>, sqlx::Error> =
            sqlx::query_as(
                r#"
            INSERT INTO relationships
                (source_id, target_id, tenant_id, workspace_id, relation_type,
                 description, weight, created_at, updated_at)
            SELECT s, t, tn, ws, rt, d, w, NOW(), NOW()
            FROM unnest(
                $1::uuid[],
                $2::uuid[],
                $3::uuid[],
                $4::uuid[],
                $5::text[],
                $6::text[],
                $7::real[]
            ) AS u(s, t, tn, ws, rt, d, w)
            ON CONFLICT (tenant_id, workspace_id, source_id, target_id, relation_type)
            DO UPDATE SET
                description = EXCLUDED.description,
                weight = EXCLUDED.weight,
                updated_at = NOW()
            RETURNING id, source_id, target_id, relation_type
            "#,
            )
            .bind(&source_ids)
            .bind(&target_ids)
            .bind(&tenants)
            .bind(&workspaces)
            .bind(&rel_types)
            .bind(&descs)
            .bind(&weights)
            .fetch_all(self.pool.as_ref())
            .await;

        let returned = match returned {
            Ok(rows) => {
                debug!(
                    target = "relationships_batch",
                    n = rows.len(),
                    "Relational sink OK"
                );
                rows
            }
            Err(e) => {
                if self.fail_closed {
                    return Err(edgequake_pipeline::PipelineError::StorageError(e.into()));
                }
                warn!(
                    target = "relationships_batch",
                    error = %e,
                    "Relational sink failed (best-effort)"
                );
                return Ok(RelationshipSinkReport {
                    ids: HashMap::new(),
                    missing_fk: missing,
                });
            }
        };

        let mut id_to_name: HashMap<uuid::Uuid, String> = HashMap::new();
        for (name, id) in &by_name {
            id_to_name.insert(*id, name.clone());
        }

        let mut ids = HashMap::new();
        for (rel_id, source_id, target_id, relation_type) in returned {
            let Some(src_name) = id_to_name.get(&source_id) else {
                continue;
            };
            let Some(tgt_name) = id_to_name.get(&target_id) else {
                continue;
            };
            let key = edgequake_storage::format_relationship_legacy_key(
                src_name,
                tgt_name,
                &relation_type,
            );
            ids.insert(key, rel_id);
        }

        Ok(RelationshipSinkReport {
            ids,
            missing_fk: missing,
        })
    }

    async fn remove_entity_sources(
        &self,
        name: &str,
        workspace_id: Option<&str>,
        _sources_to_remove: &[String],
        remaining_sources: &[String],
    ) -> edgequake_pipeline::Result<()> {
        let bare = EntityId::bare_name_from_graph_node_id(name);
        let workspace_uuid = scope_uuid(workspace_id, "workspace_id")?;

        let result = if remaining_sources.is_empty() {
            sqlx::query(
                "DELETE FROM entities \
                 WHERE (name = $1 OR name = (COALESCE(($2::uuid)::text, '') || '::' || $1)) \
                   AND (workspace_id = $2 OR ($2 IS NULL AND workspace_id IS NULL))",
            )
            .bind(bare)
            .bind(workspace_uuid)
            .execute(self.pool.as_ref())
            .await
        } else {
            sqlx::query(
                "UPDATE entities SET source_chunk_ids = $1, sync_status = 'synced', updated_at = NOW() \
                 WHERE (name = $2 OR name = (COALESCE(($3::uuid)::text, '') || '::' || $2)) \
                   AND (workspace_id = $3 OR ($3 IS NULL AND workspace_id IS NULL))",
            )
            .bind(remaining_sources)
            .bind(bare)
            .bind(workspace_uuid)
            .execute(self.pool.as_ref())
            .await
        };

        self.map_sql_result(bare, result)
    }
}
