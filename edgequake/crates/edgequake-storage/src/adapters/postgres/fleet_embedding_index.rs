//! SPEC-091 IW2 — typed fleet embeddings Postgres adapter (`FleetEmbeddingIndex`).

use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use crate::adapters::postgres::fleet_legacy_absorb::{upsert_with_legacy_absorb, AbsorbBatch};
use crate::adapters::postgres::typed_embedding_dims::{
    validate_ann_dimensions, validate_typed_embedding_batch_dims,
};
use crate::embedding_family::{
    classify_legacy_vector_id, entity_name_from_legacy_id, parse_relationship_legacy_key,
    EmbeddingFamily,
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
    search_session: super::typed_ann_search::TypedAnnSearch,
    model_name: String,
}

impl PgFleetEmbeddingIndex {
    pub fn new(pool: PgPool, model_name: impl Into<String>) -> Self {
        Self {
            pool,
            search_session: Default::default(),
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
        conn: &mut PgConnection,
        name: &str,
        dimensions: i32,
    ) -> Result<Option<ModelId>, StorageError> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM embedding_models WHERE name = $1 AND dimensions = $2",
        )
        .bind(name)
        .bind(dimensions)
        .fetch_optional(conn)
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

    /// `$4 IS NULL OR source_ids && $4` hides the document from a generic plan.
    /// When the caller passes documents, the statement requires the overlap so
    /// `idx_entities_source_ids` can start at those entities.
    const ENTITY_SOURCE_OPTIONAL: &'static str = "AND ($4::uuid[] IS NULL OR e.source_ids && $4)";
    const ENTITY_SOURCE_REQUIRED: &'static str = "AND e.source_ids && $4";

    fn apply_entity_source(sql: String, documents: bool) -> String {
        if documents {
            sql.replace(Self::ENTITY_SOURCE_OPTIONAL, Self::ENTITY_SOURCE_REQUIRED)
        } else {
            sql
        }
    }

    /// `$6 IS NULL OR modality = ANY($6)` hides the modality from a generic
    /// plan, so entity search seq-scans every entity and then keeps one
    /// modality. When the caller passes modalities, the statement requires
    /// the predicate and `idx_entities_modality` starts at those rows.
    const ENTITY_MODALITY_OPTIONAL: &'static str =
        "AND ($6::text[] IS NULL OR e.modality = ANY($6))";
    const ENTITY_MODALITY_REQUIRED: &'static str = "AND e.modality = ANY($6)";

    fn apply_entity_modality(sql: String, modality: bool) -> String {
        if modality {
            sql.replace(
                Self::ENTITY_MODALITY_OPTIONAL,
                Self::ENTITY_MODALITY_REQUIRED,
            )
        } else {
            sql
        }
    }

    /// Split `filter_ids` into entity names and UUIDs. A generic plan cannot
    /// use `id::text = ANY` or `('entity:' || name) = ANY` as an index
    /// condition, so those values are looked up on `entities_pkey` and
    /// `idx_entities_name`.
    fn entity_id_keys(ids: Option<&[String]>) -> (Vec<String>, Vec<Uuid>) {
        let mut names = Vec::new();
        let mut uuids = Vec::new();
        for id in ids.unwrap_or(&[]) {
            if let Ok(parsed) = Uuid::parse_str(id) {
                uuids.push(parsed);
            } else if let Some(name) = entity_name_from_legacy_id(id) {
                names.push(name.to_string());
            }
        }
        (names, uuids)
    }

    fn entity_search_sql(dim: i32, cast: &str, documents: bool, modality: bool) -> String {
        let sql = format!(
            "SELECT ('entity:' || e.name) AS legacy_id, \
                    1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
             FROM entity_embeddings fe \
             JOIN entities e ON e.id = fe.entity_id \
             WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
               AND ($3::uuid IS NULL OR fe.workspace_id = $3) \
               AND ($4::uuid[] IS NULL OR e.source_ids && $4) \
               AND ($5::uuid IS NULL OR e.tenant_id = $5) \
               AND ($6::text[] IS NULL OR e.modality = ANY($6)) \
             ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $7"
        );
        let sql = Self::apply_entity_source(sql, documents);
        Self::apply_entity_modality(sql, modality)
    }

    /// Id-filtered entity ANN. Each requested id is looked up before the
    /// distance order. `OFFSET 0` keeps that lookup from being pulled into
    /// the HNSW index.
    fn entity_search_sql_in_ids(dim: i32, cast: &str, documents: bool, modality: bool) -> String {
        let sql = format!(
            "SELECT ('entity:' || e.name) AS legacy_id, \
                    1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
             FROM ( \
                SELECT e.id \
                FROM unnest($7::text[]) AS wanted(name) \
                JOIN LATERAL ( \
                    SELECT e.id FROM entities e WHERE e.name = wanted.name OFFSET 0 \
                ) e ON true \
                UNION \
                SELECT e.id \
                FROM unnest($8::uuid[]) AS wanted(id) \
                JOIN LATERAL ( \
                    SELECT e.id FROM entities e WHERE e.id = wanted.id OFFSET 0 \
                ) e ON true \
             ) hit \
             JOIN entity_embeddings fe \
               ON fe.entity_id = hit.id AND fe.model_id = $2 AND fe.dimensions = {dim} \
             JOIN entities e ON e.id = hit.id \
             WHERE ($3::uuid IS NULL OR fe.workspace_id = $3) \
               AND ($4::uuid[] IS NULL OR e.source_ids && $4) \
               AND ($5::uuid IS NULL OR e.tenant_id = $5) \
               AND ($6::text[] IS NULL OR e.modality = ANY($6)) \
             ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $9"
        );
        let sql = Self::apply_entity_source(sql, documents);
        Self::apply_entity_modality(sql, modality)
    }

    /// `$4 IS NULL OR source_chunk_ids && (chunks of $4)` hides the document
    /// from a generic plan. When the caller passes documents, the overlap is
    /// required so `idx_relationships_source_chunk_ids` can start at those rows.
    /// The chunk ids stay an uncorrelated InitPlan: a lookup from each
    /// relationship back into `chunks` misses the ANN deadline on a large
    /// workspace.
    fn relationship_source_predicate(documents: bool) -> &'static str {
        if documents {
            "AND r.source_chunk_ids && ARRAY(SELECT c.id FROM chunks c WHERE c.document_id = ANY($4))"
        } else {
            "AND ($4::uuid[] IS NULL OR r.source_chunk_ids && ARRAY(SELECT c.id FROM chunks c WHERE c.document_id = ANY($4)))"
        }
    }

    fn relationship_search_sql(dim: i32, cast: &str, documents: bool) -> String {
        let source = Self::relationship_source_predicate(documents);
        // No id filter on the document statement: the caller did not pass ids,
        // so the limit bind stays $7. The unscoped statement keeps the optional
        // id predicate and binds it as $7.
        let tail = if documents {
            format!(
                "AND $6::text[] IS NULL ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $7"
            )
        } else {
            format!(
                "AND $6::text[] IS NULL \
                 AND ($7::text[] IS NULL OR r.id::text = ANY($7) \
                      OR (es.name || '->' || et.name || ':' || r.relation_type) = ANY($7)) \
                 ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $8"
            )
        };
        format!(
            "SELECT (es.name || '->' || et.name || ':' || r.relation_type) AS legacy_id, \
                    1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
             FROM relationship_embeddings fe \
             JOIN relationships r ON r.id = fe.relationship_id \
             JOIN entities es ON es.id = r.source_id \
             JOIN entities et ON et.id = r.target_id \
             WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
               AND ($3::uuid IS NULL OR fe.workspace_id = $3) \
               {source} \
               AND ($5::uuid IS NULL OR r.tenant_id = $5) \
               {tail}"
        )
    }

    /// Id-filtered relationship ANN. A UUID uses `relationships_pkey`. A legacy
    /// `source->target:type` key uses `idx_entities_name` and then
    /// `idx_relationships_source`. `OFFSET 0` keeps that lookup from being
    /// pulled into the distance order.
    fn relationship_search_sql_in_ids(dim: i32, cast: &str, documents: bool) -> String {
        let source = Self::relationship_source_predicate(documents);
        format!(
            "SELECT (es.name || '->' || et.name || ':' || r.relation_type) AS legacy_id, \
                    1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
             FROM ( \
                SELECT r.id \
                FROM unnest($7::uuid[]) AS wanted(id) \
                JOIN LATERAL ( \
                    SELECT r.id FROM relationships r WHERE r.id = wanted.id OFFSET 0 \
                ) r ON true \
                UNION \
                SELECT r.id \
                FROM unnest($8::text[], $9::text[], $10::text[]) AS wanted(src, tgt, rel) \
                JOIN LATERAL ( \
                    SELECT r.id \
                    FROM entities es \
                    JOIN entities et ON et.name = wanted.tgt \
                    JOIN relationships r \
                      ON r.source_id = es.id AND r.target_id = et.id \
                     AND r.relation_type = wanted.rel \
                    WHERE es.name = wanted.src \
                    OFFSET 0 \
                ) r ON true \
             ) hit \
             JOIN relationship_embeddings fe \
               ON fe.relationship_id = hit.id AND fe.model_id = $2 AND fe.dimensions = {dim} \
             JOIN relationships r ON r.id = hit.id \
             JOIN entities es ON es.id = r.source_id \
             JOIN entities et ON et.id = r.target_id \
             WHERE ($3::uuid IS NULL OR fe.workspace_id = $3) \
               {source} \
               AND ($5::uuid IS NULL OR r.tenant_id = $5) \
               AND $6::text[] IS NULL \
             ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $11"
        )
    }

    /// Split `filter_ids` into relationship UUIDs and legacy
    /// `source->target:type` keys.
    fn relationship_id_keys(
        ids: Option<&[String]>,
    ) -> (Vec<Uuid>, Vec<String>, Vec<String>, Vec<String>) {
        let mut uuids = Vec::new();
        let mut sources = Vec::new();
        let mut targets = Vec::new();
        let mut rel_types = Vec::new();
        for id in ids.unwrap_or(&[]) {
            if let Ok(parsed) = Uuid::parse_str(id) {
                uuids.push(parsed);
            } else if let Some((source, target, rel_type)) = parse_relationship_legacy_key(id) {
                sources.push(source);
                targets.push(target);
                rel_types.push(rel_type);
            }
        }
        (uuids, sources, targets, rel_types)
    }

    /// `$7 IS NULL OR report_id = ANY($7)` hides the id from a generic plan.
    /// That plan walks the HNSW index and can stop before the requested id,
    /// so a row that exists is missing from the answer. When the caller passes
    /// ids, the statement requires `report_id = ANY($7)` and
    /// `report_embeddings_pkey` starts at those rows.
    fn report_search_sql(dim: i32, cast: &str, in_ids: bool) -> String {
        let id_pred = if in_ids {
            "AND fe.report_id = ANY($7)"
        } else {
            "AND ($7::text[] IS NULL OR fe.report_id = ANY($7))"
        };
        format!(
            "SELECT fe.report_id AS legacy_id, \
                    1.0 - ((fe.embedding::{cast}) <=> $1::{cast}) AS score \
             FROM report_embeddings fe \
             WHERE fe.model_id = $2 AND fe.dimensions = {dim} \
               AND ($3::uuid IS NULL OR fe.workspace_id = $3) \
               AND $4::uuid[] IS NULL \
               AND $5::uuid IS NULL \
               AND $6::text[] IS NULL \
               {id_pred} \
             ORDER BY (fe.embedding::{cast}) <=> $1::{cast} LIMIT $8"
        )
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

        let workspace_ids: Vec<Uuid> = rows.iter().map(|r| r.workspace_id.into_uuid()).collect();
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
        let type_matches = req.vector_type.as_deref().is_none_or(|kind| match family {
            EmbeddingFamily::Entity => kind.eq_ignore_ascii_case("entity"),
            EmbeddingFamily::Relationship => {
                kind.eq_ignore_ascii_case("relationship") || kind.eq_ignore_ascii_case("relation")
            }
            EmbeddingFamily::Report => kind.eq_ignore_ascii_case("report"),
        });
        if req.limit == 0
            || !type_matches
            || req.document_ids.as_ref().is_some_and(Vec::is_empty)
            || req.modalities.as_ref().is_some_and(Vec::is_empty)
            || req.filter_ids.as_ref().is_some_and(Vec::is_empty)
            || matches!(
                family,
                EmbeddingFamily::Relationship | EmbeddingFamily::Report
            ) && req.modalities.is_some()
            || matches!(family, EmbeddingFamily::Report)
                && (req.document_ids.is_some() || req.tenant_id.is_some())
        {
            return Ok(Vec::new());
        }

        let dim = validate_ann_dimensions(req.embedding.len() as i32)?;
        let candidate_limit =
            super::typed_ann_search::TypedAnnSearch::candidate_limit(req.limit as usize);
        let mut conn = self.pool.acquire().await.map_err(StorageError::from)?;
        let mut tx = self
            .search_session
            .begin(
                &mut conn,
                candidate_limit,
                req.tenant_id.map(|id| id.into_uuid()),
                req.workspace_id.map(|id| id.into_uuid()),
            )
            .await?;
        let model_id = match self
            .find_model_id(tx.as_mut(), &self.model_name, dim)
            .await?
        {
            Some(id) => id,
            None => {
                tx.commit().await?;
                return Ok(Vec::new());
            }
        };

        let vector = Self::format_vector(&req.embedding);
        let cast = format!("halfvec({dim})");

        let documents = req.document_ids.as_ref().is_some();
        let modality = req.modalities.as_ref().is_some();
        let in_ids = req.filter_ids.as_ref().is_some();
        let (entity_names, entity_ids) = Self::entity_id_keys(req.filter_ids.as_deref());
        let (rel_uuids, rel_sources, rel_targets, rel_types) =
            if matches!(family, EmbeddingFamily::Relationship) && in_ids {
                Self::relationship_id_keys(req.filter_ids.as_deref())
            } else {
                (Vec::new(), Vec::new(), Vec::new(), Vec::new())
            };
        let q = match family {
            EmbeddingFamily::Entity if in_ids => {
                Self::entity_search_sql_in_ids(dim, &cast, documents, modality)
            }
            EmbeddingFamily::Entity => Self::entity_search_sql(dim, &cast, documents, modality),
            EmbeddingFamily::Relationship if in_ids => {
                Self::relationship_search_sql_in_ids(dim, &cast, documents)
            }
            EmbeddingFamily::Relationship => Self::relationship_search_sql(dim, &cast, documents),
            EmbeddingFamily::Report => Self::report_search_sql(dim, &cast, in_ids),
        };

        let q = super::typed_ann_search::TypedAnnSearch::ordered_sql(&q, req.limit);
        let query = sqlx::query(&q)
            .bind(&vector)
            .bind(model_id.0)
            .bind(req.workspace_id.map(|id| id.into_uuid()))
            .bind(req.document_ids.as_deref())
            .bind(req.tenant_id.map(|id| id.into_uuid()))
            .bind(req.modalities.as_deref());
        let query = if matches!(family, EmbeddingFamily::Entity) && in_ids {
            query
                .bind(&entity_names)
                .bind(&entity_ids)
                .bind(candidate_limit as i64)
        } else if matches!(family, EmbeddingFamily::Entity) {
            query.bind(candidate_limit as i64)
        } else if matches!(family, EmbeddingFamily::Relationship) && in_ids {
            query
                .bind(&rel_uuids)
                .bind(&rel_sources)
                .bind(&rel_targets)
                .bind(&rel_types)
                .bind(candidate_limit as i64)
        } else if matches!(family, EmbeddingFamily::Relationship) && documents {
            query.bind(candidate_limit as i64)
        } else {
            query
                .bind(req.filter_ids.as_deref())
                .bind(candidate_limit as i64)
        };
        let rows = query
            .fetch_all(tx.as_mut())
            .await
            .map_err(StorageError::from)?;
        tx.commit().await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
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
            .bind(workspace.into_uuid())
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
                workspace_id: WorkspaceId::new(ws),
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

#[cfg(test)]
mod tests {
    use super::PgFleetEmbeddingIndex;

    #[test]
    fn entity_search_requires_source_ids_when_documents_are_passed() {
        let open = PgFleetEmbeddingIndex::entity_search_sql(768, "halfvec(768)", false, false);
        assert!(open.contains("$4::uuid[] IS NULL OR e.source_ids && $4"));
        assert!(open.contains("$3::uuid IS NULL OR fe.workspace_id = $3"));
        assert!(open.contains("$5::uuid IS NULL OR e.tenant_id = $5"));
        assert!(open.contains("$6::text[] IS NULL OR e.modality = ANY($6)"));
        assert!(!open.contains("id::text = ANY"));
        assert!(open.contains("LIMIT $7"));

        let scoped = PgFleetEmbeddingIndex::entity_search_sql(768, "halfvec(768)", true, false);
        assert!(scoped.contains("AND e.source_ids && $4"));
        assert!(!scoped.contains("$4::uuid[] IS NULL"));
        assert!(scoped.contains("$6::text[] IS NULL OR e.modality = ANY($6)"));
    }

    #[test]
    fn entity_search_requires_modality_when_passed() {
        let open = PgFleetEmbeddingIndex::entity_search_sql(768, "halfvec(768)", false, false);
        assert!(open.contains("$6::text[] IS NULL OR e.modality = ANY($6)"));

        let scoped = PgFleetEmbeddingIndex::entity_search_sql(768, "halfvec(768)", false, true);
        assert!(scoped.contains("AND e.modality = ANY($6)"));
        assert!(!scoped.contains("$6::text[] IS NULL"));
        assert!(scoped.contains("$3::uuid IS NULL OR fe.workspace_id = $3"));

        let by_id =
            PgFleetEmbeddingIndex::entity_search_sql_in_ids(768, "halfvec(768)", false, true);
        assert!(by_id.contains("AND e.modality = ANY($6)"));
        assert!(!by_id.contains("$6::text[] IS NULL"));
        assert!(by_id.contains("OFFSET 0"));
    }

    #[test]
    fn entity_search_by_id_looks_up_name_and_uuid() {
        let sql = PgFleetEmbeddingIndex::entity_search_sql_in_ids(768, "halfvec(768)", true, false);
        assert!(sql.contains("OFFSET 0"));
        assert!(sql.contains("e.name = wanted.name"));
        assert!(sql.contains("e.id = wanted.id"));
        assert!(sql.contains("AND e.source_ids && $4"));
        assert!(!sql.contains("$7::text[] IS NULL"));
        assert!(!sql.contains("id::text = ANY"));
        assert!(sql.contains("LIMIT $9"));
    }

    #[test]
    fn relationship_search_requires_chunk_overlap_when_documents_are_passed() {
        let open = PgFleetEmbeddingIndex::relationship_search_sql(768, "halfvec(768)", false);
        assert!(open.contains("$4::uuid[] IS NULL OR r.source_chunk_ids && ARRAY"));
        assert!(open.contains("$7::text[] IS NULL OR r.id::text = ANY($7)"));
        assert!(open.contains("LIMIT $8"));

        let scoped = PgFleetEmbeddingIndex::relationship_search_sql(768, "halfvec(768)", true);
        assert!(scoped.contains(
            "AND r.source_chunk_ids && ARRAY(SELECT c.id FROM chunks c WHERE c.document_id = ANY($4))"
        ));
        assert!(!scoped.contains("$4::uuid[] IS NULL"));
        assert!(!scoped.contains("r.id::text = ANY"));
        assert!(scoped.contains("LIMIT $7"));
    }

    #[test]
    fn relationship_search_by_id_looks_up_uuid_and_legacy_key() {
        let sql = PgFleetEmbeddingIndex::relationship_search_sql_in_ids(768, "halfvec(768)", true);
        assert!(sql.contains("OFFSET 0"));
        assert!(sql.contains("r.id = wanted.id"));
        assert!(sql.contains("es.name = wanted.src"));
        assert!(sql.contains("et.name = wanted.tgt"));
        assert!(sql.contains("r.relation_type = wanted.rel"));
        assert!(sql.contains(
            "AND r.source_chunk_ids && ARRAY(SELECT c.id FROM chunks c WHERE c.document_id = ANY($4))"
        ));
        assert!(!sql.contains("$7::text[] IS NULL"));
        assert!(!sql.contains("id::text = ANY"));
        assert!(sql.contains("LIMIT $11"));
    }

    #[test]
    fn report_search_requires_report_id_when_ids_are_passed() {
        let open = PgFleetEmbeddingIndex::report_search_sql(768, "halfvec(768)", false);
        assert!(open.contains("$7::text[] IS NULL OR fe.report_id = ANY($7)"));
        assert!(open.contains("$3::uuid IS NULL OR fe.workspace_id = $3"));
        assert!(open.contains("LIMIT $8"));

        let scoped = PgFleetEmbeddingIndex::report_search_sql(768, "halfvec(768)", true);
        assert!(scoped.contains("AND fe.report_id = ANY($7)"));
        assert!(!scoped.contains("$7::text[] IS NULL"));
        assert!(scoped.contains("LIMIT $8"));
    }
}
