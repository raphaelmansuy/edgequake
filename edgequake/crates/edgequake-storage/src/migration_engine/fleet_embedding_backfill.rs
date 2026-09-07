//! SPEC-091 IW2 descriptor: legacy `eq_*_vectors` fleet rows → typed
//! entity/relationship/report embeddings (migration 130).
//!
//! Generalizes the W3 chunk backfill machinery: fleet-wide table enumeration,
//! keyset cursor `(family, table, last_id)`, idempotent UNNEST +
//! `ON CONFLICT DO UPDATE` with **within-batch arbiter dedupe** (SPEC-139 /
//! LAW-139-1: Postgres 21000 if the same conflict key is proposed twice) and a
//! SQL `DISTINCT ON` belt (SPEC-396) so a missed Rust collapse cannot 21000.

use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha384};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::embedding_family::{
    classify_legacy_vector_id, entity_name_from_legacy_id, EmbeddingFamily,
};
use crate::error::StorageError;
use crate::graph_batch_dedupe::normalize_relation_type_str;

use super::coverage::{
    list_vector_tables, list_vector_tables_ex, load_entity_name_index, resolve_relationship_id,
    EntityNameIndex,
};
use super::runner::{BackfillJob, BatchOutcome, VerifyReport};

const DESCRIPTOR_DEF: &str = concat!(
    "iw2-fleet-embedding-backfill/v2:",
    "source=legacy_vectors_fleet:keyset_per_table;families=entity,relationship,report;",
    "join=entities+relationships+ensure_spine;insert=unnest+on_conflict;",
    "verify=coverage+sampled_vector_equality_fleet"
);

fn family_prefix(family: EmbeddingFamily) -> &'static str {
    match family {
        EmbeddingFamily::Entity => "entity:",
        EmbeddingFamily::Relationship => "%->%:%",
        EmbeddingFamily::Report => "community_report:%",
    }
}

async fn count_family_rows(
    pool: &PgPool,
    table: &str,
    family: EmbeddingFamily,
) -> Result<i64, StorageError> {
    let pattern = family_prefix(family);
    let sql = match family {
        EmbeddingFamily::Relationship => format!(
            "SELECT COUNT(*) FROM public.{table} WHERE id LIKE '%->%:%' \
             AND id NOT LIKE 'entity:%' AND id NOT LIKE 'community_report:%'"
        ),
        _ => format!("SELECT COUNT(*) FROM public.{table} WHERE id LIKE '{pattern}'"),
    };
    match sqlx::query_scalar::<_, i64>(&sql).fetch_one(pool).await {
        Ok(n) => Ok(n),
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => Ok(0),
        Err(e) => Err(StorageError::Database(format!(
            "iw2 count({table},{family:?}) failed: {e}"
        ))),
    }
}

fn parse_vector_text(raw: &str) -> Option<Vec<f32>> {
    let inner = raw.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    for part in inner.split(',') {
        out.push(part.trim().parse::<f32>().ok()?);
    }
    Some(out)
}

fn parse_workspace_uuid(meta: &Value) -> Option<Uuid> {
    meta.get("workspace_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// One proposed typed-embedding write; arbiter key lives outside so
/// [`super::conflict_dedupe::dedupe_last_write_wins`] can collapse it.
struct FleetWrite {
    workspace_id: Uuid,
    legacy_id: String,
    vector: String,
    dim: i32,
}

struct UnnestedBatch<K> {
    keys: Vec<K>,
    workspace_ids: Vec<Uuid>,
    vectors: Vec<String>,
    dims: Vec<i32>,
    legacy_ids: Vec<String>,
}

fn collapse_writes<K: Eq + std::hash::Hash + Clone>(
    rows: Vec<(K, FleetWrite)>,
) -> UnnestedBatch<K> {
    let kept = super::conflict_dedupe::dedupe_last_write_wins(rows);
    let mut out = UnnestedBatch {
        keys: Vec::with_capacity(kept.len()),
        workspace_ids: Vec::with_capacity(kept.len()),
        vectors: Vec::with_capacity(kept.len()),
        dims: Vec::with_capacity(kept.len()),
        legacy_ids: Vec::with_capacity(kept.len()),
    };
    for (key, row) in kept {
        out.keys.push(key);
        out.workspace_ids.push(row.workspace_id);
        out.vectors.push(row.vector);
        out.dims.push(row.dim);
        out.legacy_ids.push(row.legacy_id);
    }
    out
}

/// SQL belt (SPEC-396 / SPEC-110 pattern): `DISTINCT ON` the ON CONFLICT
/// arbiter so a missed Rust collapse cannot propose the same key twice.
/// `WITH ORDINALITY` + `ORDER BY arbiter, ord DESC` is last-write-wins.
const ENTITY_FLEET_UPSERT_SQL: &str = r#"
INSERT INTO entity_embeddings
  (model_id, entity_id, workspace_id, embedding, dimensions, legacy_vector_id)
SELECT $1, e, w, v::halfvec, d, lid
FROM (
  SELECT DISTINCT ON (e) e, w, v, d, lid
  FROM unnest($2::uuid[], $3::uuid[], $4::text[], $5::int[], $6::text[])
    WITH ORDINALITY AS t(e, w, v, d, lid, ord)
  ORDER BY e, ord DESC
) s
ON CONFLICT (model_id, entity_id) DO UPDATE
  SET legacy_vector_id = COALESCE(entity_embeddings.legacy_vector_id, EXCLUDED.legacy_vector_id)
"#;

const RELATIONSHIP_FLEET_UPSERT_SQL: &str = r#"
INSERT INTO relationship_embeddings
  (model_id, relationship_id, workspace_id, embedding, dimensions, legacy_vector_id)
SELECT $1, r, w, v::halfvec, d, lid
FROM (
  SELECT DISTINCT ON (r) r, w, v, d, lid
  FROM unnest($2::uuid[], $3::uuid[], $4::text[], $5::int[], $6::text[])
    WITH ORDINALITY AS t(r, w, v, d, lid, ord)
  ORDER BY r, ord DESC
) s
ON CONFLICT (model_id, relationship_id) DO UPDATE
  SET legacy_vector_id = COALESCE(relationship_embeddings.legacy_vector_id, EXCLUDED.legacy_vector_id)
"#;

const REPORT_FLEET_UPSERT_SQL: &str = r#"
INSERT INTO report_embeddings
  (model_id, report_id, workspace_id, embedding, dimensions, legacy_vector_id)
SELECT $1, r, w, v::halfvec, d, lid
FROM (
  SELECT DISTINCT ON (r) r, w, v, d, lid
  FROM unnest($2::text[], $3::uuid[], $4::text[], $5::int[], $6::text[])
    WITH ORDINALITY AS t(r, w, v, d, lid, ord)
  ORDER BY r, ord DESC
) s
ON CONFLICT (model_id, report_id) DO UPDATE
  SET legacy_vector_id = COALESCE(report_embeddings.legacy_vector_id, EXCLUDED.legacy_vector_id)
"#;

const ENTITY_ROW_UPSERT_SQL: &str = r#"
INSERT INTO entity_embeddings
  (model_id, entity_id, workspace_id, embedding, dimensions, legacy_vector_id)
VALUES ($1, $2, $3, $4::halfvec, $5, $6)
ON CONFLICT (model_id, entity_id) DO UPDATE
  SET legacy_vector_id = COALESCE(entity_embeddings.legacy_vector_id, EXCLUDED.legacy_vector_id)
"#;

const RELATIONSHIP_ROW_UPSERT_SQL: &str = r#"
INSERT INTO relationship_embeddings
  (model_id, relationship_id, workspace_id, embedding, dimensions, legacy_vector_id)
VALUES ($1, $2, $3, $4::halfvec, $5, $6)
ON CONFLICT (model_id, relationship_id) DO UPDATE
  SET legacy_vector_id = COALESCE(relationship_embeddings.legacy_vector_id, EXCLUDED.legacy_vector_id)
"#;

const REPORT_ROW_UPSERT_SQL: &str = r#"
INSERT INTO report_embeddings
  (model_id, report_id, workspace_id, embedding, dimensions, legacy_vector_id)
VALUES ($1, $2, $3, $4::halfvec, $5, $6)
ON CONFLICT (model_id, report_id) DO UPDATE
  SET legacy_vector_id = COALESCE(report_embeddings.legacy_vector_id, EXCLUDED.legacy_vector_id)
"#;

struct UpsertBatchResult {
    written: i64,
    extra_failed: i64,
    /// SPEC-396: 21000 survived DISTINCT ON + per-row — do not advance last_id.
    hold_cursor: bool,
}

/// Keyset resume id: hold keeps the incoming cursor so the batch is retried.
pub(crate) fn resume_last_id(
    scanned_last_id: &str,
    incoming_last_id: &str,
    hold_cursor: bool,
) -> String {
    if hold_cursor {
        incoming_last_id.to_string()
    } else {
        scanned_last_id.to_string()
    }
}

fn conflict_sqlstate(err: &sqlx::Error) -> Option<&'static str> {
    match err {
        sqlx::Error::Database(db) => match db.code().as_deref() {
            Some("21000") => Some("21000"),
            Some("23505") => Some("23505"),
            _ => None,
        },
        _ => None,
    }
}

async fn savepoint(tx: &mut Transaction<'_, Postgres>, name: &str) -> Result<(), StorageError> {
    sqlx::query(&format!("SAVEPOINT {name}"))
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("iw2 savepoint {name}: {e}")))
}

async fn rollback_to_savepoint(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
) -> Result<(), StorageError> {
    sqlx::query(&format!("ROLLBACK TO SAVEPOINT {name}"))
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("iw2 rollback to {name}: {e}")))
}

async fn release_savepoint(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
) -> Result<(), StorageError> {
    sqlx::query(&format!("RELEASE SAVEPOINT {name}"))
        .execute(&mut **tx)
        .await
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("iw2 release {name}: {e}")))
}

fn vector_literal(embedding: &[f32]) -> String {
    format!(
        "[{}]",
        embedding
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn workspace_from_row(row: &sqlx::postgres::PgRow, meta: &Value) -> Option<Uuid> {
    parse_workspace_uuid(meta).or_else(|| {
        row.try_get::<Option<String>, _>("col_workspace_id")
            .ok()
            .flatten()
            .and_then(|s| Uuid::parse_str(s.trim()).ok())
    })
}

/// Ensure a relational entity spine row for a legacy fleet key (SPEC-111).
///
/// When typed-only ingest / wipe left `eq_*_vectors` without `public.entities`,
/// iw2 must create the FK parent before inserting `entity_embeddings`.
///
/// Only auto-creates when metadata carries extract signals (`entity_name`,
/// non-empty `description`, or `entity_type`). Bare `{workspace_id}` ghosts
/// remain durable misses (`failed_count`) — see E2E-111 unresolved test.
async fn ensure_entity_spine(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    meta: &Value,
    legacy_name: &str,
) -> Result<Option<Uuid>, StorageError> {
    if legacy_name.is_empty() {
        return Ok(None);
    }
    let has_signal = meta
        .get("entity_name")
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.is_empty())
        || meta
            .get("description")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty())
        || meta
            .get("entity_type")
            .and_then(|v| v.as_str())
            .is_some_and(|s| !s.is_empty());
    if !has_signal {
        return Ok(None);
    }
    let tenant_id = meta
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());
    let entity_type = meta
        .get("entity_type")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("UNKNOWN");
    let description = meta
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Prefer display name from metadata; fall back to legacy key suffix.
    let name = meta
        .get("entity_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(legacy_name);

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO public.entities (name, entity_type, workspace_id, tenant_id, description) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (tenant_id, workspace_id, name) DO UPDATE \
           SET updated_at = now() \
         RETURNING id",
    )
    .bind(name)
    .bind(entity_type)
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(description)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| StorageError::Database(format!("iw2 ensure entity spine: {e}")))?;
    Ok(Some(id))
}

async fn ensure_relationship_spine(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    meta: &Value,
    src: &str,
    tgt: &str,
    rel_type: &str,
    index: &EntityNameIndex,
) -> Result<Option<Uuid>, StorageError> {
    // Fabricate entity_name signals so ensure_entity_spine accepts endpoint keys
    // even when relationship metadata only carries type/src_id/tgt_id.
    let mut src_meta = meta.clone();
    if let Some(obj) = src_meta.as_object_mut() {
        obj.insert("entity_name".into(), json!(src));
    }
    let mut tgt_meta = meta.clone();
    if let Some(obj) = tgt_meta.as_object_mut() {
        obj.insert("entity_name".into(), json!(tgt));
    }
    let src_id = match index.resolve(src) {
        Some(id) => id,
        None => match ensure_entity_spine(tx, workspace_id, &src_meta, src).await? {
            Some(id) => id,
            None => return Ok(None),
        },
    };
    // Refresh resolve for tgt after possible src insert (index may be stale).
    let tgt_id = match index.resolve(tgt) {
        Some(id) => id,
        None => match ensure_entity_spine(tx, workspace_id, &tgt_meta, tgt).await? {
            Some(id) => id,
            None => return Ok(None),
        },
    };
    let tenant_id = meta
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());
    let description = meta
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO public.relationships \
           (source_id, target_id, relation_type, workspace_id, tenant_id, description) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (tenant_id, workspace_id, source_id, target_id, relation_type) DO UPDATE \
           SET updated_at = now() \
         RETURNING id",
    )
    .bind(src_id)
    .bind(tgt_id)
    .bind(rel_type)
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(description)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| StorageError::Database(format!("iw2 ensure relationship spine: {e}")))?;
    Ok(Some(id))
}

pub struct FleetEmbeddingBackfillJob {
    model_name: String,
}

impl FleetEmbeddingBackfillJob {
    pub fn new(model_name: String) -> Self {
        Self { model_name }
    }
}

#[async_trait]
impl BackfillJob for FleetEmbeddingBackfillJob {
    fn step_id(&self) -> &'static str {
        "iw2-fleet-embedding-backfill"
    }

    fn step_sha384(&self) -> String {
        Sha384::digest(DESCRIPTOR_DEF.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn schema_generation(&self) -> i32 {
        1
    }

    fn initial_cursor(&self) -> Value {
        json!({
            "family": EmbeddingFamily::Entity.backfill_family_key(),
            "table": Value::Null,
            "last_id": ""
        })
    }

    async fn estimate_total(&self, pool: &PgPool) -> Result<i64, StorageError> {
        let tables = list_vector_tables(pool).await?;
        let mut total = 0;
        for t in &tables {
            for family in EmbeddingFamily::FLEET_BACKFILL_FAMILIES {
                total += count_family_rows(pool, t, family).await?;
            }
        }
        Ok(total)
    }

    async fn run_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        cursor: &Value,
        limit: i64,
    ) -> Result<BatchOutcome, StorageError> {
        let family_key = cursor
            .get("family")
            .and_then(Value::as_str)
            .unwrap_or("entity");
        let family = match family_key {
            "relationship" => EmbeddingFamily::Relationship,
            "report" => EmbeddingFamily::Report,
            _ => EmbeddingFamily::Entity,
        };

        let tables = list_vector_tables_ex(&mut **tx).await?;
        if tables.is_empty() {
            return Ok(BatchOutcome {
                scanned: 0,
                written: 0,
                failed: 0,
                next_cursor: None,
            });
        }

        let cur_table = cursor
            .get("table")
            .and_then(Value::as_str)
            .map(str::to_string);
        let last_id = cursor
            .get("last_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let active_idx = match cur_table.as_deref() {
            Some(t) => match tables.iter().position(|x| x == t) {
                Some(i) => i,
                None => match tables.iter().position(|x| x.as_str() > t) {
                    Some(i) => i,
                    None => return advance_family_or_finish(family, tables),
                },
            },
            None => 0,
        };
        let start_id = if cur_table.as_deref() == tables.get(active_idx).map(String::as_str) {
            last_id.clone()
        } else {
            String::new()
        };
        let table = &tables[active_idx];

        let filter = match family {
            EmbeddingFamily::Entity => "id LIKE 'entity:%'",
            EmbeddingFamily::Relationship => {
                "id LIKE '%->%:%' AND id NOT LIKE 'entity:%' AND id NOT LIKE 'community_report:%'"
            }
            EmbeddingFamily::Report => "id LIKE 'community_report:%'",
        };
        let has_col = match sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = $1
                  AND column_name = 'workspace_id'
             )",
        )
        .bind(table)
        .fetch_one(&mut **tx)
        .await
        {
            Ok(v) => v,
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => {
                return Ok(BatchOutcome {
                    scanned: 0,
                    written: 0,
                    failed: 0,
                    next_cursor: Some(json!({
                        "family": family.backfill_family_key(),
                        "table": table,
                        "last_id": ""
                    })),
                });
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "iw2 workspace_id probe({table}): {e}"
                )))
            }
        };
        let scan_sql = if has_col {
            format!(
                "SELECT id, embedding::text, metadata, workspace_id::text AS col_workspace_id \
                 FROM public.{table} WHERE {filter} AND id > $1 ORDER BY id LIMIT $2"
            )
        } else {
            format!(
                "SELECT id, embedding::text, metadata FROM public.{table} \
                 WHERE {filter} AND id > $1 ORDER BY id LIMIT $2"
            )
        };

        let rows = match sqlx::query(&scan_sql)
            .bind(&start_id)
            .bind(limit)
            .fetch_all(&mut **tx)
            .await
        {
            Ok(rows) => rows,
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => {
                return Ok(BatchOutcome {
                    scanned: 0,
                    written: 0,
                    failed: 0,
                    next_cursor: Some(json!({
                        "family": family.backfill_family_key(),
                        "table": table,
                        "last_id": ""
                    })),
                });
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "iw2 scan({table},{family:?}) failed: {e}"
                )))
            }
        };

        if rows.is_empty() {
            if active_idx + 1 < tables.len() {
                return Ok(BatchOutcome {
                    scanned: 0,
                    written: 0,
                    failed: 0,
                    next_cursor: Some(json!({
                        "family": family.backfill_family_key(),
                        "table": tables[active_idx + 1],
                        "last_id": ""
                    })),
                });
            }
            return advance_family_or_finish(family, tables);
        }

        let scanned = rows.len() as i64;
        let next_id = rows
            .last()
            .and_then(|r| r.try_get::<String, _>("id").ok())
            .unwrap_or_default();

        let (written, failed, hold_cursor) = match family {
            EmbeddingFamily::Entity => self.write_entity_batch(tx, &rows).await?,
            EmbeddingFamily::Relationship => self.write_relationship_batch(tx, &rows).await?,
            EmbeddingFamily::Report => self.write_report_batch(tx, &rows).await?,
        };
        if hold_cursor {
            // SPEC-396: never swallow-and-advance. Returning Err rolls the
            // batch TX back so `last_id` stays put for the next claim.
            tracing::error!(
                table,
                family = family.backfill_family_key(),
                incoming_last_id = %start_id,
                scanned_last_id = %next_id,
                written,
                failed,
                "iw2 21000 survived DISTINCT ON + per-row — cursor not advanced"
            );
            return Err(StorageError::Database(format!(
                "iw2 {} insert 21000 after DISTINCT ON and per-row; \
                 keyset cursor not advanced (last_id stays {:?})",
                family.backfill_family_key(),
                start_id
            )));
        }
        let resume_id = resume_last_id(&next_id, &start_id, false);

        Ok(BatchOutcome {
            scanned,
            written,
            failed,
            next_cursor: Some(json!({
                "family": family.backfill_family_key(),
                "table": table,
                "last_id": resume_id
            })),
        })
    }

    async fn verify(&self, pool: &PgPool) -> Result<VerifyReport, StorageError> {
        let tables = list_vector_tables(pool).await?;
        let mut agg = VerifyReport {
            metric: "iw2-fleet-embedding".to_string(),
            expected: 0,
            actual: 0,
            sampled: 0,
            mismatches: 0,
        };
        for table in &tables {
            for family in EmbeddingFamily::FLEET_BACKFILL_FAMILIES {
                let r = super::verify::verify_fleet_embedding_backfill(
                    pool,
                    table,
                    family,
                    &self.model_name,
                )
                .await?;
                agg.expected += r.expected;
                agg.actual += r.actual;
                agg.sampled += r.sampled;
                agg.mismatches += r.mismatches;
            }
        }
        Ok(agg)
    }
}

fn advance_family_or_finish(
    family: EmbeddingFamily,
    tables: Vec<String>,
) -> Result<BatchOutcome, StorageError> {
    let next_family = match family {
        EmbeddingFamily::Entity => Some(EmbeddingFamily::Relationship),
        EmbeddingFamily::Relationship => Some(EmbeddingFamily::Report),
        EmbeddingFamily::Report => None,
    };
    if let Some(next) = next_family {
        let first_table = tables.first().cloned().unwrap_or_default();
        return Ok(BatchOutcome {
            scanned: 0,
            written: 0,
            failed: 0,
            next_cursor: Some(json!({
                "family": next.backfill_family_key(),
                "table": if first_table.is_empty() { Value::Null } else { json!(first_table) },
                "last_id": ""
            })),
        });
    }
    Ok(BatchOutcome {
        scanned: 0,
        written: 0,
        failed: 0,
        next_cursor: None,
    })
}

impl FleetEmbeddingBackfillJob {
    async fn upsert_model(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        dimensions: i32,
    ) -> Result<Uuid, StorageError> {
        sqlx::query_scalar(
            "INSERT INTO embedding_models (name, dimensions) VALUES ($1, $2) \
             ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name RETURNING id",
        )
        .bind(&self.model_name)
        .bind(dimensions)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("iw2 model upsert failed: {e}")))
    }

    /// Returns `(written, failed, hold_cursor)` — failed = durable unresolved
    /// joins / bad rows. `hold_cursor` means 21000 survived the SQL belt.
    async fn write_entity_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        rows: &[sqlx::postgres::PgRow],
    ) -> Result<(i64, i64, bool), StorageError> {
        let mut prepared: Vec<(Uuid, FleetWrite)> = Vec::new();
        let mut dimensions = 0i32;
        let mut failed = 0i64;
        let mut index_cache: std::collections::HashMap<Uuid, EntityNameIndex> =
            std::collections::HashMap::new();

        for row in rows {
            let id: String = row
                .try_get("id")
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let emb_text: String = row
                .try_get("embedding")
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let meta: Value = row.try_get("metadata").unwrap_or(json!({}));
            let Some(name) = entity_name_from_legacy_id(&id).map(str::to_string) else {
                failed += 1;
                continue;
            };
            let Some(ws) = workspace_from_row(row, &meta) else {
                failed += 1;
                continue;
            };
            let Some(embedding) = parse_vector_text(&emb_text) else {
                failed += 1;
                continue;
            };
            dimensions = embedding.len() as i32;
            if let std::collections::hash_map::Entry::Vacant(e) = index_cache.entry(ws) {
                e.insert(load_entity_name_index(tx, ws).await?);
            }
            let index = index_cache.get(&ws).expect("just inserted");
            let eid = match index.resolve(&name) {
                Some(eid) => eid,
                None => match ensure_entity_spine(tx, ws, &meta, &name).await? {
                    Some(eid) => {
                        index_cache.insert(ws, load_entity_name_index(tx, ws).await?);
                        eid
                    }
                    None => {
                        failed += 1;
                        continue;
                    }
                },
            };
            prepared.push((
                eid,
                FleetWrite {
                    workspace_id: ws,
                    legacy_id: id,
                    vector: vector_literal(&embedding),
                    dim: embedding.len() as i32,
                },
            ));
        }
        let batch = collapse_writes(prepared);
        if batch.keys.is_empty() {
            return Ok((0, failed, false));
        }
        let model_id = self.upsert_model(tx, dimensions).await?;
        let upsert = self.upsert_entity_batch(tx, model_id, &batch).await?;
        Ok((
            upsert.written,
            failed + upsert.extra_failed,
            upsert.hold_cursor,
        ))
    }

    async fn write_relationship_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        rows: &[sqlx::postgres::PgRow],
    ) -> Result<(i64, i64, bool), StorageError> {
        let mut prepared: Vec<(Uuid, FleetWrite)> = Vec::new();
        let mut dimensions = 0i32;
        let mut failed = 0i64;
        let mut index_cache: std::collections::HashMap<Uuid, EntityNameIndex> =
            std::collections::HashMap::new();

        for row in rows {
            let id: String = row
                .try_get("id")
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let emb_text: String = row
                .try_get("embedding")
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let meta: Value = row.try_get("metadata").unwrap_or(json!({}));
            let Some(ws) = workspace_from_row(row, &meta) else {
                failed += 1;
                continue;
            };
            let Some(embedding) = parse_vector_text(&emb_text) else {
                failed += 1;
                continue;
            };
            dimensions = embedding.len() as i32;
            if let std::collections::hash_map::Entry::Vacant(e) = index_cache.entry(ws) {
                e.insert(load_entity_name_index(tx, ws).await?);
            }
            let index = index_cache.get(&ws).expect("just inserted");
            // SPEC-133: index-guided parse when endpoint names contain `->`.
            let Some((src, tgt, rel_type_raw)) = index.parse_relationship_legacy_key(&id) else {
                failed += 1;
                continue;
            };
            let rel_type = normalize_relation_type_str(&rel_type_raw);
            let rid = match resolve_relationship_id(tx, ws, &src, &tgt, &rel_type, index).await? {
                Some(rid) => rid,
                None => {
                    match ensure_relationship_spine(tx, ws, &meta, &src, &tgt, &rel_type, index)
                        .await?
                    {
                        Some(rid) => {
                            index_cache.insert(ws, load_entity_name_index(tx, ws).await?);
                            rid
                        }
                        None => {
                            failed += 1;
                            continue;
                        }
                    }
                }
            };
            prepared.push((
                rid,
                FleetWrite {
                    workspace_id: ws,
                    legacy_id: id,
                    vector: vector_literal(&embedding),
                    dim: embedding.len() as i32,
                },
            ));
        }
        let batch = collapse_writes(prepared);
        if batch.keys.is_empty() {
            return Ok((0, failed, false));
        }
        let model_id = self.upsert_model(tx, dimensions).await?;
        let upsert = self.upsert_relationship_batch(tx, model_id, &batch).await?;
        Ok((
            upsert.written,
            failed + upsert.extra_failed,
            upsert.hold_cursor,
        ))
    }

    async fn write_report_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        rows: &[sqlx::postgres::PgRow],
    ) -> Result<(i64, i64, bool), StorageError> {
        let mut prepared: Vec<(String, FleetWrite)> = Vec::new();
        let mut dimensions = 0i32;
        let mut failed = 0i64;

        for row in rows {
            let id: String = row
                .try_get("id")
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let emb_text: String = row
                .try_get("embedding")
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let meta: Value = row.try_get("metadata").unwrap_or(json!({}));
            if classify_legacy_vector_id(&id) != Some(EmbeddingFamily::Report) {
                failed += 1;
                continue;
            }
            let Some(ws) = workspace_from_row(row, &meta) else {
                failed += 1;
                continue;
            };
            let Some(embedding) = parse_vector_text(&emb_text) else {
                failed += 1;
                continue;
            };
            dimensions = embedding.len() as i32;
            prepared.push((
                id.clone(),
                FleetWrite {
                    workspace_id: ws,
                    legacy_id: id,
                    vector: vector_literal(&embedding),
                    dim: embedding.len() as i32,
                },
            ));
        }
        let batch = collapse_writes(prepared);
        if batch.keys.is_empty() {
            return Ok((0, failed, false));
        }
        let model_id = self.upsert_model(tx, dimensions).await?;
        let upsert = self.upsert_report_batch(tx, model_id, &batch).await?;
        Ok((
            upsert.written,
            failed + upsert.extra_failed,
            upsert.hold_cursor,
        ))
    }

    async fn upsert_entity_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model_id: Uuid,
        batch: &UnnestedBatch<Uuid>,
    ) -> Result<UpsertBatchResult, StorageError> {
        savepoint(tx, "iw2_upsert").await?;
        let written_res = sqlx::query(ENTITY_FLEET_UPSERT_SQL)
            .bind(model_id)
            .bind(&batch.keys)
            .bind(&batch.workspace_ids)
            .bind(&batch.vectors)
            .bind(&batch.dims)
            .bind(&batch.legacy_ids)
            .execute(&mut **tx)
            .await;
        match written_res {
            Ok(r) => {
                release_savepoint(tx, "iw2_upsert").await.ok();
                Ok(UpsertBatchResult {
                    written: r.rows_affected() as i64,
                    extra_failed: 0,
                    hold_cursor: false,
                })
            }
            Err(e) => {
                let state = conflict_sqlstate(&e);
                rollback_to_savepoint(tx, "iw2_upsert").await?;
                match state {
                    Some("21000") | Some("23505") => {
                        tracing::warn!(
                            sqlstate = state.unwrap_or("?"),
                            keys = ?batch.legacy_ids,
                            "iw2 entity UNNEST conflict — per-row fallback; 21000 will not advance cursor"
                        );
                        self.upsert_entity_rows(tx, model_id, batch, state == Some("21000"))
                            .await
                    }
                    _ => Err(StorageError::Database(format!(
                        "iw2 entity insert failed: {e}"
                    ))),
                }
            }
        }
    }

    async fn upsert_relationship_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model_id: Uuid,
        batch: &UnnestedBatch<Uuid>,
    ) -> Result<UpsertBatchResult, StorageError> {
        savepoint(tx, "iw2_upsert").await?;
        let written_res = sqlx::query(RELATIONSHIP_FLEET_UPSERT_SQL)
            .bind(model_id)
            .bind(&batch.keys)
            .bind(&batch.workspace_ids)
            .bind(&batch.vectors)
            .bind(&batch.dims)
            .bind(&batch.legacy_ids)
            .execute(&mut **tx)
            .await;
        match written_res {
            Ok(r) => {
                release_savepoint(tx, "iw2_upsert").await.ok();
                Ok(UpsertBatchResult {
                    written: r.rows_affected() as i64,
                    extra_failed: 0,
                    hold_cursor: false,
                })
            }
            Err(e) => {
                let state = conflict_sqlstate(&e);
                rollback_to_savepoint(tx, "iw2_upsert").await?;
                match state {
                    Some("21000") | Some("23505") => {
                        tracing::warn!(
                            sqlstate = state.unwrap_or("?"),
                            keys = ?batch.legacy_ids,
                            "iw2 relationship UNNEST conflict — per-row fallback; 21000 will not advance cursor"
                        );
                        self.upsert_relationship_rows(tx, model_id, batch, state == Some("21000"))
                            .await
                    }
                    _ => Err(StorageError::Database(format!(
                        "iw2 relationship insert failed: {e}"
                    ))),
                }
            }
        }
    }

    async fn upsert_report_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model_id: Uuid,
        batch: &UnnestedBatch<String>,
    ) -> Result<UpsertBatchResult, StorageError> {
        savepoint(tx, "iw2_upsert").await?;
        let written_res = sqlx::query(REPORT_FLEET_UPSERT_SQL)
            .bind(model_id)
            .bind(&batch.keys)
            .bind(&batch.workspace_ids)
            .bind(&batch.vectors)
            .bind(&batch.dims)
            .bind(&batch.legacy_ids)
            .execute(&mut **tx)
            .await;
        match written_res {
            Ok(r) => {
                release_savepoint(tx, "iw2_upsert").await.ok();
                Ok(UpsertBatchResult {
                    written: r.rows_affected() as i64,
                    extra_failed: 0,
                    hold_cursor: false,
                })
            }
            Err(e) => {
                let state = conflict_sqlstate(&e);
                rollback_to_savepoint(tx, "iw2_upsert").await?;
                match state {
                    Some("21000") | Some("23505") => {
                        tracing::warn!(
                            sqlstate = state.unwrap_or("?"),
                            keys = ?batch.legacy_ids,
                            "iw2 report UNNEST conflict — per-row fallback; 21000 will not advance cursor"
                        );
                        self.upsert_report_rows(tx, model_id, batch, state == Some("21000"))
                            .await
                    }
                    _ => Err(StorageError::Database(format!(
                        "iw2 report insert failed: {e}"
                    ))),
                }
            }
        }
    }

    async fn upsert_entity_rows(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model_id: Uuid,
        batch: &UnnestedBatch<Uuid>,
        from_21000: bool,
    ) -> Result<UpsertBatchResult, StorageError> {
        upsert_uuid_rows(
            tx,
            model_id,
            batch,
            ENTITY_ROW_UPSERT_SQL,
            "entity",
            from_21000,
        )
        .await
    }

    async fn upsert_relationship_rows(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model_id: Uuid,
        batch: &UnnestedBatch<Uuid>,
        from_21000: bool,
    ) -> Result<UpsertBatchResult, StorageError> {
        upsert_uuid_rows(
            tx,
            model_id,
            batch,
            RELATIONSHIP_ROW_UPSERT_SQL,
            "relationship",
            from_21000,
        )
        .await
    }

    async fn upsert_report_rows(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        model_id: Uuid,
        batch: &UnnestedBatch<String>,
        from_21000: bool,
    ) -> Result<UpsertBatchResult, StorageError> {
        let mut written = 0i64;
        let mut extra_failed = 0i64;
        for i in 0..batch.keys.len() {
            savepoint(tx, "iw2_row").await?;
            let res = sqlx::query(REPORT_ROW_UPSERT_SQL)
                .bind(model_id)
                .bind(&batch.keys[i])
                .bind(batch.workspace_ids[i])
                .bind(&batch.vectors[i])
                .bind(batch.dims[i])
                .bind(&batch.legacy_ids[i])
                .execute(&mut **tx)
                .await;
            match res {
                Ok(r) => {
                    release_savepoint(tx, "iw2_row").await.ok();
                    written += r.rows_affected() as i64;
                }
                Err(e) => {
                    let state = conflict_sqlstate(&e);
                    rollback_to_savepoint(tx, "iw2_row").await?;
                    match state {
                        Some("23505") => extra_failed += 1,
                        Some("21000") => {
                            return Ok(UpsertBatchResult {
                                written,
                                extra_failed: extra_failed + (batch.keys.len() - i) as i64,
                                hold_cursor: true,
                            });
                        }
                        _ => {
                            return Err(StorageError::Database(format!(
                                "iw2 report per-row insert failed: {e}"
                            )));
                        }
                    }
                }
            }
        }
        Ok(UpsertBatchResult {
            written,
            extra_failed,
            hold_cursor: from_21000 && written == 0,
        })
    }
}

async fn upsert_uuid_rows(
    tx: &mut Transaction<'_, Postgres>,
    model_id: Uuid,
    batch: &UnnestedBatch<Uuid>,
    sql: &str,
    family: &str,
    from_21000: bool,
) -> Result<UpsertBatchResult, StorageError> {
    let mut written = 0i64;
    let mut extra_failed = 0i64;
    for i in 0..batch.keys.len() {
        savepoint(tx, "iw2_row").await?;
        let res = sqlx::query(sql)
            .bind(model_id)
            .bind(batch.keys[i])
            .bind(batch.workspace_ids[i])
            .bind(&batch.vectors[i])
            .bind(batch.dims[i])
            .bind(&batch.legacy_ids[i])
            .execute(&mut **tx)
            .await;
        match res {
            Ok(r) => {
                release_savepoint(tx, "iw2_row").await.ok();
                written += r.rows_affected() as i64;
            }
            Err(e) => {
                let state = conflict_sqlstate(&e);
                rollback_to_savepoint(tx, "iw2_row").await?;
                match state {
                    Some("23505") => extra_failed += 1,
                    Some("21000") => {
                        return Ok(UpsertBatchResult {
                            written,
                            extra_failed: extra_failed + (batch.keys.len() - i) as i64,
                            hold_cursor: true,
                        });
                    }
                    _ => {
                        return Err(StorageError::Database(format!(
                            "iw2 {family} per-row insert failed: {e}"
                        )));
                    }
                }
            }
        }
    }
    Ok(UpsertBatchResult {
        written,
        extra_failed,
        hold_cursor: from_21000 && written == 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_iw2_step_identity_stable() {
        let job = FleetEmbeddingBackfillJob::new("text-embedding-3-small".into());
        assert_eq!(job.step_id(), "iw2-fleet-embedding-backfill");
        assert_eq!(job.schema_generation(), 1);
        assert_eq!(job.step_sha384().len(), 96);
    }

    #[test]
    fn contract_spec396_sql_distinct_on_arbiters() {
        assert!(
            ENTITY_FLEET_UPSERT_SQL.contains("DISTINCT ON (e)"),
            "entity UNNEST must DISTINCT ON entity_id arbiter"
        );
        assert!(
            RELATIONSHIP_FLEET_UPSERT_SQL.contains("DISTINCT ON (r)"),
            "relationship UNNEST must DISTINCT ON relationship_id arbiter"
        );
        assert!(
            REPORT_FLEET_UPSERT_SQL.contains("DISTINCT ON (r)"),
            "report UNNEST must DISTINCT ON report_id arbiter"
        );
        for sql in [
            ENTITY_FLEET_UPSERT_SQL,
            RELATIONSHIP_FLEET_UPSERT_SQL,
            REPORT_FLEET_UPSERT_SQL,
        ] {
            assert!(sql.contains("WITH ORDINALITY"), "last-write-wins ordinal");
            assert!(
                sql.contains("ON CONFLICT") && sql.contains("DO UPDATE"),
                "must keep COALESCE provenance DO UPDATE"
            );
            assert!(!sql.contains("DO NOTHING"), "DO NOTHING loses provenance");
        }
    }

    #[test]
    fn contract_spec396_resume_last_id_holds_on_21000() {
        assert_eq!(resume_last_id("entity:Z", "entity:A", true), "entity:A");
        assert_eq!(resume_last_id("entity:Z", "entity:A", false), "entity:Z");
        assert_eq!(resume_last_id("entity:Z", "", false), "entity:Z");
    }
}
