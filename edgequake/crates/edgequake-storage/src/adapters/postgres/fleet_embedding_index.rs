//! SPEC-091 IW2 — typed fleet embeddings Postgres adapter (`FleetEmbeddingIndex`).

use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::adapters::postgres::fleet_legacy_absorb::{upsert_with_legacy_absorb, AbsorbBatch};
use crate::adapters::postgres::typed_embedding_dims::{
    validate_ann_dimensions, validate_typed_embedding_batch_dims,
};
use crate::embedding_family::{
    classify_legacy_vector_id, entity_name_from_legacy_id, EmbeddingFamily,
};
use crate::error::StorageError;
use crate::graph_batch_dedupe::normalize_relation_type_str;
use crate::migration_engine::coverage::{
    load_entity_name_index_pool, resolve_relationship_id_pool, EntityNameIndex,
};
use crate::traits::domain::{
    EmbeddingCapabilities, FleetEmbeddingIndex, FleetEmbeddingKey, FleetEmbeddingRow,
    MirrorLegacyReport, ModelId, ScoredFleet, UpsertReport, VectorQuery, WorkspaceId,
};
use std::collections::HashMap;

/// Postgres adapter for entity/relationship/report typed embeddings.
pub struct PgFleetEmbeddingIndex {
    pool: PgPool,
    model_name: String,
}

impl PgFleetEmbeddingIndex {
    pub fn new(pool: PgPool, model_name: impl Into<String>) -> Self {
        Self {
            pool,
            model_name: model_name.into(),
        }
    }

    async fn resolve_model_id(&self, dimensions: i32) -> Result<ModelId, StorageError> {
        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO embedding_models (name, dimensions)
            VALUES ($1, $2)
            ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            "#,
        )
        .bind(&self.model_name)
        .bind(dimensions)
        .fetch_one(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(ModelId(id))
    }

    async fn find_model_id(
        &self,
        name: &str,
        dimensions: i32,
    ) -> Result<Option<ModelId>, StorageError> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM embedding_models WHERE name = $1 AND dimensions = $2",
        )
        .bind(name)
        .bind(dimensions)
        .fetch_optional(&self.pool)
        .await
        .map_err(StorageError::from)?;
        Ok(id.map(ModelId))
    }

    fn format_vector(embedding: &[f32]) -> String {
        let mut s = String::with_capacity(embedding.len() * 8 + 2);
        s.push('[');
        for (i, v) in embedding.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&v.to_string());
        }
        s.push(']');
        s
    }
}

#[async_trait]
impl FleetEmbeddingIndex for PgFleetEmbeddingIndex {
    fn capabilities(&self, _family: EmbeddingFamily) -> EmbeddingCapabilities {
        EmbeddingCapabilities {
            metric: "cosine",
            supports_filters: true,
            supports_rerank: false,
        }
    }

    async fn upsert_batch(
        &self,
        family: EmbeddingFamily,
        _model: ModelId,
        rows: &[FleetEmbeddingRow],
    ) -> Result<UpsertReport, StorageError> {
        if rows.is_empty() {
            return Ok(UpsertReport::default());
        }
        let dimensions = validate_typed_embedding_batch_dims(
            rows.iter().map(|r| r.dimensions),
            rows.iter().map(|r| r.embedding.len()),
        )?;
        let model_id = self.resolve_model_id(dimensions).await?;

        let workspace_ids: Vec<Uuid> = rows.iter().map(|r| r.workspace_id.0).collect();
        let dims: Vec<i32> = rows.iter().map(|r| r.dimensions).collect();
        let vectors: Vec<String> = rows
            .iter()
            .map(|r| Self::format_vector(&r.embedding))
            .collect();

        // Empty string → NULLIF → NULL (serving upserts without provenance).
        let legacy_ids: Vec<String> = rows
            .iter()
            .map(|r| r.legacy_vector_id.clone().unwrap_or_default())
            .collect();

        let (fk_uuids, fk_texts) = match family {
            EmbeddingFamily::Entity => (
                Some(
                    rows.iter()
                        .map(|r| match r.key {
                            FleetEmbeddingKey::Entity(id) => id,
                            _ => Uuid::nil(),
                        })
                        .collect::<Vec<_>>(),
                ),
                None,
            ),
            EmbeddingFamily::Relationship => (
                Some(
                    rows.iter()
                        .map(|r| match r.key {
                            FleetEmbeddingKey::Relationship(id) => id,
                            _ => Uuid::nil(),
                        })
                        .collect::<Vec<_>>(),
                ),
                None,
            ),
            EmbeddingFamily::Report => (
                None,
                Some(
                    rows.iter()
                        .map(|r| match &r.key {
                            FleetEmbeddingKey::Report(id) => id.clone(),
                            _ => String::new(),
                        })
                        .collect::<Vec<_>>(),
                ),
            ),
        };

        let (upserted, absorbed) = upsert_with_legacy_absorb(
            &self.pool,
            &AbsorbBatch {
                family,
                model_id: model_id.0,
                fk_uuids: fk_uuids.as_deref(),
                fk_texts: fk_texts.as_deref(),
                workspace_ids: &workspace_ids,
                vectors: &vectors,
                dims: &dims,
                legacy_ids: &legacy_ids,
            },
        )
        .await?;

        Ok(UpsertReport {
            upserted,
            absorbed_legacy_collisions: absorbed,
        })
    }

    async fn search(
        &self,
        family: EmbeddingFamily,
        req: &VectorQuery,
    ) -> Result<Vec<ScoredFleet>, StorageError> {
        let dim = validate_ann_dimensions(req.embedding.len() as i32)?;
        let model_id = match self.find_model_id(&self.model_name, dim).await? {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };

        // SPEC-146: empty allow-set is fail-closed.
        if let Some(allow) = req.allowed_document_ids.as_ref() {
            if allow.is_empty() {
                return Ok(Vec::new());
            }
        }

        let vector = Self::format_vector(&req.embedding);
        let ws_filter = req.workspace_id.is_some();
        let want = req.limit as usize;
        let fetch_limit = if req.allowed_document_ids.is_some() {
            super::ann_abac::abac_overfetch_limit(req.limit) as i64
        } else {
            req.limit as i64
        };
        let cast = format!("halfvec({dim})");
        let allow = req.allowed_document_ids.as_ref();
        let doc_filter = allow.is_some();
        let allow_len = allow.map(|a| a.len()).unwrap_or(0);
        let allow_uuid_pred = super::ann_abac::uuid_in_allow_sql("left(cid, 36)::uuid", "$ALLOW", allow_len);

        // Provenance ∩ allow: entity/rel rows keep iff source_ids overlaps OR
        // any source_chunk_ids derives a document in the allow-set.
        let entity_allow_sql = format!(" AND (\
            (e.source_ids IS NOT NULL AND e.source_ids && $ALLOW::uuid[]) \
            OR EXISTS (\
                SELECT 1 FROM unnest(COALESCE(e.source_chunk_ids, ARRAY[]::text[])) AS cid \
                WHERE cid ~ '^[0-9a-fA-F]{{8}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{12}}-chunk-' \
                  AND {allow_uuid_pred}\
            )\
        )");
        // relationships table has source_chunk_ids only (no source_ids uuid[]).
        let rel_allow_sql = format!(" AND EXISTS (\
                SELECT 1 FROM unnest(COALESCE(r.source_chunk_ids, ARRAY[]::text[])) AS cid \
                WHERE cid ~ '^[0-9a-fA-F]{{8}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{4}}-[0-9a-fA-F]{{12}}-chunk-' \
                  AND {allow_uuid_pred}\
            )");

        let q = match (family, ws_filter, doc_filter) {
            (EmbeddingFamily::Entity, true, false) => format!(
                "SELECT ('entity:' || e.name) AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM entity_embeddings fe \
                 JOIN entities e ON e.id = fe.entity_id \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} AND fe.workspace_id = $3 \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $4"
            ),
            (EmbeddingFamily::Entity, false, false) => format!(
                "SELECT ('entity:' || e.name) AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM entity_embeddings fe \
                 JOIN entities e ON e.id = fe.entity_id \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $3"
            ),
            (EmbeddingFamily::Entity, true, true) => format!(
                "SELECT ('entity:' || e.name) AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM entity_embeddings fe \
                 JOIN entities e ON e.id = fe.entity_id \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} AND fe.workspace_id = $3 \
                 {entity_allow} \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $5",
                entity_allow = entity_allow_sql.replace("$ALLOW", "$4")
            ),
            (EmbeddingFamily::Entity, false, true) => format!(
                "SELECT ('entity:' || e.name) AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM entity_embeddings fe \
                 JOIN entities e ON e.id = fe.entity_id \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
                 {entity_allow} \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $4",
                entity_allow = entity_allow_sql.replace("$ALLOW", "$3")
            ),
            (EmbeddingFamily::Relationship, true, false) => format!(
                "SELECT (es.name || '->' || et.name || ':' || r.relation_type) AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM relationship_embeddings fe \
                 JOIN relationships r ON r.id = fe.relationship_id \
                 JOIN entities es ON es.id = r.source_id \
                 JOIN entities et ON et.id = r.target_id \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} AND fe.workspace_id = $3 \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $4"
            ),
            (EmbeddingFamily::Relationship, false, false) => format!(
                "SELECT (es.name || '->' || et.name || ':' || r.relation_type) AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM relationship_embeddings fe \
                 JOIN relationships r ON r.id = fe.relationship_id \
                 JOIN entities es ON es.id = r.source_id \
                 JOIN entities et ON et.id = r.target_id \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $3"
            ),
            (EmbeddingFamily::Relationship, true, true) => format!(
                "SELECT (es.name || '->' || et.name || ':' || r.relation_type) AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM relationship_embeddings fe \
                 JOIN relationships r ON r.id = fe.relationship_id \
                 JOIN entities es ON es.id = r.source_id \
                 JOIN entities et ON et.id = r.target_id \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} AND fe.workspace_id = $3 \
                 {rel_allow} \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $5",
                rel_allow = rel_allow_sql.replace("$ALLOW", "$4")
            ),
            (EmbeddingFamily::Relationship, false, true) => format!(
                "SELECT (es.name || '->' || et.name || ':' || r.relation_type) AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM relationship_embeddings fe \
                 JOIN relationships r ON r.id = fe.relationship_id \
                 JOIN entities es ON es.id = r.source_id \
                 JOIN entities et ON et.id = r.target_id \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
                 {rel_allow} \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $4",
                rel_allow = rel_allow_sql.replace("$ALLOW", "$3")
            ),
            // Reports: no document provenance — drop all under ABAC allow filter.
            (EmbeddingFamily::Report, _, true) => {
                return Ok(Vec::new());
            }
            (EmbeddingFamily::Report, true, false) => format!(
                "SELECT fe.report_id AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM report_embeddings fe \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} AND fe.workspace_id = $3 \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $4"
            ),
            (EmbeddingFamily::Report, false, false) => format!(
                "SELECT fe.report_id AS legacy_id, \
                        1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
                 FROM report_embeddings fe \
                 WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $3"
            ),
        };

        let use_tx = doc_filter;
        let mut tx = if use_tx {
            let mut t = self.pool.begin().await.map_err(StorageError::from)?;
            let _ = sqlx::query("SET LOCAL hnsw.iterative_scan = 'relaxed_order'")
                .execute(&mut *t)
                .await;
            Some(t)
        } else {
            None
        };

        let mut query = sqlx::query(&q).bind(&vector).bind(model_id.0);
        if let Some(ws) = req.workspace_id {
            query = query.bind(ws.0);
        }
        if let Some(allow_ids) = allow {
            query = query.bind(allow_ids.as_slice());
        }
        query = query.bind(fetch_limit);

        let rows = match tx.as_mut() {
            Some(t) => query.fetch_all(&mut **t).await.map_err(StorageError::from)?,
            None => query.fetch_all(&self.pool).await.map_err(StorageError::from)?,
        };

        if let Some(t) = tx {
            t.commit().await.map_err(StorageError::from)?;
        }

        let mut out = Vec::with_capacity(rows.len().min(want));
        for row in rows.into_iter().take(want) {
            let legacy_id: String = row.try_get("legacy_id").map_err(StorageError::from)?;
            let score: f64 = row.try_get("score").map_err(StorageError::from)?;
            out.push(ScoredFleet {
                legacy_id,
                score: score as f32,
            });
        }
        Ok(out)
    }

    async fn delete_for_workspace(
        &self,
        family: EmbeddingFamily,
        workspace: WorkspaceId,
    ) -> Result<u64, StorageError> {
        let table = family.typed_table();
        let deleted = sqlx::query(&format!("DELETE FROM {table} WHERE workspace_id = $1"))
            .bind(workspace.0)
            .execute(&self.pool)
            .await
            .map_err(StorageError::from)?
            .rows_affected();
        Ok(deleted)
    }

    async fn mirror_legacy_batch(
        &self,
        rows: &[(String, Vec<f32>, Value)],
        count_as_entities: bool,
        known_relationship_ids: Option<&HashMap<String, Uuid>>,
    ) -> Result<MirrorLegacyReport, StorageError> {
        let mut entity_rows: Vec<FleetEmbeddingRow> = Vec::new();
        let mut rel_rows: Vec<FleetEmbeddingRow> = Vec::new();
        let mut report_rows: Vec<FleetEmbeddingRow> = Vec::new();
        let mut report = MirrorLegacyReport::default();
        let mut index_cache: HashMap<Uuid, EntityNameIndex> = HashMap::new();

        for (id, embedding, meta) in rows {
            let Some(ws) = meta
                .get("workspace_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
            else {
                report.push_invalid_workspace(id);
                continue;
            };
            if let std::collections::hash_map::Entry::Vacant(e) = index_cache.entry(ws) {
                e.insert(load_entity_name_index_pool(&self.pool, ws).await?);
            }
            let index = index_cache.get(&ws).expect("just inserted");
            let row_template = |key: FleetEmbeddingKey| FleetEmbeddingRow {
                workspace_id: WorkspaceId(ws),
                embedding: embedding.clone(),
                dimensions: embedding.len() as i32,
                key,
                legacy_vector_id: Some(id.clone()),
            };

            if count_as_entities {
                let Some(name) = entity_name_from_legacy_id(id) else {
                    continue;
                };
                report.eligible += 1;
                let Some(eid) = index.resolve(name) else {
                    report.push_miss(id);
                    continue;
                };
                entity_rows.push(row_template(FleetEmbeddingKey::Entity(eid)));
                continue;
            }

            match classify_legacy_vector_id(id) {
                Some(EmbeddingFamily::Report) => {
                    report.eligible += 1;
                    report_rows.push(row_template(FleetEmbeddingKey::Report(id.clone())));
                }
                Some(EmbeddingFamily::Relationship) => {
                    report.eligible += 1;
                    // SPEC-130: prefer sink-returned UUID (same-session identity).
                    let rid = if let Some(known) = known_relationship_ids {
                        known.get(id).copied()
                    } else {
                        None
                    };
                    let rid = match rid {
                        Some(rid) => Some(rid),
                        None => {
                            // SPEC-133: index-guided parse when names contain `->`.
                            let Some((src, tgt, rel_type)) =
                                index.parse_relationship_legacy_key(id)
                            else {
                                report.push_miss(id);
                                continue;
                            };
                            let rel_type = normalize_relation_type_str(&rel_type);
                            resolve_relationship_id_pool(
                                &self.pool, ws, &src, &tgt, &rel_type, index,
                            )
                            .await?
                        }
                    };
                    let Some(rid) = rid else {
                        report.push_miss(id);
                        continue;
                    };
                    rel_rows.push(row_template(FleetEmbeddingKey::Relationship(rid)));
                }
                _ => {}
            }
        }

        report.resolved = (entity_rows.len() + rel_rows.len() + report_rows.len()) as u64;
        if !entity_rows.is_empty() {
            let ur = self
                .upsert_batch(EmbeddingFamily::Entity, ModelId(Uuid::nil()), &entity_rows)
                .await?;
            report.absorbed_legacy_collisions += ur.absorbed_legacy_collisions;
        }
        if !rel_rows.is_empty() {
            let ur = self
                .upsert_batch(
                    EmbeddingFamily::Relationship,
                    ModelId(Uuid::nil()),
                    &rel_rows,
                )
                .await?;
            report.absorbed_legacy_collisions += ur.absorbed_legacy_collisions;
        }
        if !report_rows.is_empty() {
            let ur = self
                .upsert_batch(EmbeddingFamily::Report, ModelId(Uuid::nil()), &report_rows)
                .await?;
            report.absorbed_legacy_collisions += ur.absorbed_legacy_collisions;
        }
        Ok(report)
    }
}
