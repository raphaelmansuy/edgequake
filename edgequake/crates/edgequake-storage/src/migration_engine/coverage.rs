//! SPEC-111 — shared coverage / name-resolve helpers for iw2 + advisor.
//!
//! LAW-111-2: drop readiness is uncovered==0, not legacy emptiness.
//! LAW-111-6: one `normalize_entity_name` SSOT for joins (write path).
//! LAW-C3: fleet **drop** coverage ≡ migration 131 = provenance only
//! (`legacy_vector_id = v.id`). Normalize is for write/stamp, not drop-GREEN.

use std::collections::HashMap;

use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::embedding_family::{
    entity_name_from_legacy_id, parse_relationship_legacy_key_with_resolver, EmbeddingFamily,
};
use crate::entity_id::normalize_entity_name;
use crate::error::StorageError;

/// Legacy chunk vector id shape used by migration 126 / `count_uncovered_chunk_rows`
/// / W3 verify `expected`+`actual` (LAW-139-3). Do not use a looser `LIKE '%-chunk-%'`.
pub const LEGACY_CHUNK_VECTOR_ID_RE: &str =
    r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}-chunk-[0-9]+$";

/// Workspace-scoped entity name → id index (raw + normalized keys).
#[derive(Debug, Default, Clone)]
pub struct EntityNameIndex {
    by_key: HashMap<String, Uuid>,
}

impl EntityNameIndex {
    /// Build from `(id, name)` rows for one workspace.
    ///
    /// SPEC-120: callers should feed rows `ORDER BY created_at ASC, id ASC`.
    /// All keys use `or_insert` so the **oldest** row wins for exact, normalized,
    /// and workspace-suffix aliases (deterministic under concurrent spines).
    pub fn from_rows(rows: impl IntoIterator<Item = (Uuid, String)>) -> Self {
        let mut by_key = HashMap::new();
        for (id, name) in rows {
            if !name.is_empty() {
                by_key.entry(name.clone()).or_insert(id);
            }
            let norm = normalize_entity_name(&name);
            if !norm.is_empty() {
                by_key.entry(norm).or_insert(id);
            }
            // Workspace-prefixed spine names (`{ws}::{name}`) — legacy dual-write
            // used to match these via `name = $ws || '::' || $legacy`.
            if let Some((_, suffix)) = name.rsplit_once("::") {
                if !suffix.is_empty() {
                    by_key.entry(suffix.to_string()).or_insert(id);
                    let sn = normalize_entity_name(suffix);
                    if !sn.is_empty() {
                        by_key.entry(sn).or_insert(id);
                    }
                }
            }
        }
        Self { by_key }
    }

    /// Resolve a legacy or display name to an entity id.
    pub fn resolve(&self, legacy_or_display: &str) -> Option<Uuid> {
        if legacy_or_display.is_empty() {
            return None;
        }
        if let Some(id) = self.by_key.get(legacy_or_display) {
            return Some(*id);
        }
        let norm = normalize_entity_name(legacy_or_display);
        if norm.is_empty() {
            return None;
        }
        self.by_key.get(&norm).copied()
    }

    /// SPEC-133 DRY: parse `SRC->TGT:TYPE` using this index as the existence check.
    ///
    /// Prefer this over calling [`parse_relationship_legacy_key_with_resolver`]
    /// with a hand-rolled closure at every call site.
    pub fn parse_relationship_legacy_key(
        &self,
        legacy_id: &str,
    ) -> Option<(String, String, String)> {
        parse_relationship_legacy_key_with_resolver(legacy_id, |n| self.resolve(n).is_some())
    }
}

/// Load all entities for a workspace into an [`EntityNameIndex`].
pub async fn load_entity_name_index(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
) -> Result<EntityNameIndex, StorageError> {
    // SPEC-120: stable order so normalized/suffix or_insert prefers oldest.
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, name FROM public.entities WHERE workspace_id = $1 \
         ORDER BY created_at ASC, id ASC",
    )
    .bind(workspace_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| StorageError::Database(format!("coverage load entities failed: {e}")))?;
    Ok(EntityNameIndex::from_rows(rows))
}

/// Load entity index from a pool (advisor / verify / stamp paths).
pub async fn load_entity_name_index_pool(
    pool: &PgPool,
    workspace_id: Uuid,
) -> Result<EntityNameIndex, StorageError> {
    // SPEC-120: stable order so normalized/suffix or_insert prefers oldest.
    let rows: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, name FROM public.entities WHERE workspace_id = $1 \
         ORDER BY created_at ASC, id ASC",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(|e| StorageError::Database(format!("coverage load entities failed: {e}")))?;
    Ok(EntityNameIndex::from_rows(rows))
}

/// Resolve relationship id via normalized endpoint names + relation type.
pub async fn resolve_relationship_id(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    src: &str,
    tgt: &str,
    rel_type: &str,
    index: &EntityNameIndex,
) -> Result<Option<Uuid>, StorageError> {
    let Some(sid) = index.resolve(src) else {
        return Ok(None);
    };
    let Some(tid) = index.resolve(tgt) else {
        return Ok(None);
    };
    let rid: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM public.relationships \
         WHERE source_id = $1 AND target_id = $2 AND relation_type = $3 \
           AND workspace_id = $4 LIMIT 1",
    )
    .bind(sid)
    .bind(tid)
    .bind(rel_type)
    .bind(workspace_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| StorageError::Database(format!("coverage resolve relationship failed: {e}")))?;
    Ok(rid)
}

/// Resolve relationship id on a pool (mirror / stamp).
pub async fn resolve_relationship_id_pool(
    pool: &PgPool,
    workspace_id: Uuid,
    src: &str,
    tgt: &str,
    rel_type: &str,
    index: &EntityNameIndex,
) -> Result<Option<Uuid>, StorageError> {
    let Some(sid) = index.resolve(src) else {
        return Ok(None);
    };
    let Some(tid) = index.resolve(tgt) else {
        return Ok(None);
    };
    let rid: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM public.relationships \
         WHERE source_id = $1 AND target_id = $2 AND relation_type = $3 \
           AND workspace_id = $4 LIMIT 1",
    )
    .bind(sid)
    .bind(tid)
    .bind(rel_type)
    .bind(workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| StorageError::Database(format!("coverage resolve relationship failed: {e}")))?;
    Ok(rid)
}

/// SSOT list of `public.eq_%_vectors` tables (alphanumeric/`_` names only).
pub async fn list_vector_tables(pool: &PgPool) -> Result<Vec<String>, StorageError> {
    let mut tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name LIKE 'eq\\_%\\_vectors' \
         ORDER BY table_name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| StorageError::Database(format!("coverage list vectors failed: {e}")))?;
    tables.retain(|t| t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    Ok(tables)
}

/// Executor-generic list (iw2 / w3 batch txs).
pub async fn list_vector_tables_ex<'e, E>(ex: E) -> Result<Vec<String>, StorageError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let mut tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name LIKE 'eq\\_%\\_vectors' \
         ORDER BY table_name",
    )
    .fetch_all(ex)
    .await
    .map_err(|e| StorageError::Database(format!("coverage list vectors failed: {e}")))?;
    tables.retain(|t| t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    Ok(tables)
}

/// Split of migration-126 uncovered chunk rows (SPEC-396 honesty).
///
/// `total()` ≡ `count_uncovered_chunk_rows` ≡ DROP 126 (`legacy ∧ ¬typed`).
/// The split is advisor-only — it does **not** weaken the drop predicate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UncoveredChunkSplit {
    /// UUID-shaped `{doc}-chunk-{n}` with no `public.chunks` row.
    pub missing_spine: i64,
    /// Spine exists, but no `chunk_embeddings` coverage.
    pub missing_embedding: i64,
}

impl UncoveredChunkSplit {
    pub fn total(self) -> i64 {
        self.missing_spine + self.missing_embedding
    }

    pub fn add(&mut self, other: Self) {
        self.missing_spine += other.missing_spine;
        self.missing_embedding += other.missing_embedding;
    }
}

/// Uncovered legacy chunk rows — mirrors migration 126 guard predicate.
pub async fn count_uncovered_chunk_rows(pool: &PgPool) -> Result<i64, StorageError> {
    Ok(count_uncovered_chunk_split(pool).await?.total())
}

/// Uncovered chunk rows split into missing spine vs missing embedding.
pub async fn count_uncovered_chunk_split(
    pool: &PgPool,
) -> Result<UncoveredChunkSplit, StorageError> {
    let tables = list_vector_tables(pool).await?;
    let mut total = UncoveredChunkSplit::default();
    for t in tables {
        total.add(count_uncovered_chunk_split_table(pool, &t).await?);
    }
    Ok(total)
}

async fn count_uncovered_chunk_split_table(
    pool: &PgPool,
    table: &str,
) -> Result<UncoveredChunkSplit, StorageError> {
    let re = LEGACY_CHUNK_VECTOR_ID_RE;
    let sql = format!(
        "SELECT \
           count(*) FILTER ( \
             WHERE v.id ~ '{re}' \
               AND NOT EXISTS ( \
                    SELECT 1 FROM public.chunks c \
                    WHERE c.document_id = left(v.id, 36)::uuid \
                      AND c.chunk_index = substring(v.id from 44)::int)) \
             AS missing_spine, \
           count(*) FILTER ( \
             WHERE v.id ~ '{re}' \
               AND EXISTS ( \
                    SELECT 1 FROM public.chunks c \
                    WHERE c.document_id = left(v.id, 36)::uuid \
                      AND c.chunk_index = substring(v.id from 44)::int) \
               AND NOT EXISTS ( \
                    SELECT 1 FROM public.chunks c \
                    JOIN public.chunk_embeddings ce ON ce.chunk_id = c.id \
                    WHERE c.document_id = left(v.id, 36)::uuid \
                      AND c.chunk_index = substring(v.id from 44)::int)) \
             AS missing_embedding \
         FROM public.{table} v"
    );
    match sqlx::query_as::<_, (i64, i64)>(&sql).fetch_one(pool).await {
        Ok((missing_spine, missing_embedding)) => Ok(UncoveredChunkSplit {
            missing_spine,
            missing_embedding,
        }),
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => {
            Ok(UncoveredChunkSplit::default())
        }
        Err(e) => Err(StorageError::Database(format!(
            "coverage uncovered chunk split failed: {e}"
        ))),
    }
}

/// Drop-covered: provenance `legacy_vector_id` (entity/rel) or report_id/provenance.
/// Matches migration 131 after SPEC-111 residual harden (no exact-name fallback).
pub async fn fleet_row_drop_covered(
    pool: &PgPool,
    legacy_id: &str,
    family: EmbeddingFamily,
) -> Result<bool, StorageError> {
    let sql = match family {
        EmbeddingFamily::Entity => {
            "SELECT EXISTS (SELECT 1 FROM public.entity_embeddings WHERE legacy_vector_id = $1)"
        }
        EmbeddingFamily::Relationship => {
            "SELECT EXISTS (SELECT 1 FROM public.relationship_embeddings WHERE legacy_vector_id = $1)"
        }
        EmbeddingFamily::Report => {
            "SELECT EXISTS (SELECT 1 FROM public.report_embeddings \
             WHERE legacy_vector_id = $1 OR report_id = $1)"
        }
    };
    match sqlx::query_scalar::<_, bool>(sql)
        .bind(legacy_id)
        .fetch_one(pool)
        .await
    {
        Ok(v) => Ok(v),
        Err(sqlx::Error::Database(db))
            if db.code().as_deref() == Some("42703")
                || db.message().contains("legacy_vector_id") =>
        {
            // Pre-143 schema: cannot be drop-covered without provenance column.
            Ok(false)
        }
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => Ok(false),
        Err(e) => Err(StorageError::Database(format!(
            "coverage drop-covered check failed: {e}"
        ))),
    }
}

/// Count uncovered legacy fleet rows — set-based SQL ≡ migration 131 guards.
pub async fn count_uncovered_fleet_rows(pool: &PgPool) -> Result<i64, StorageError> {
    let tables = list_vector_tables(pool).await?;
    let mut uncovered = 0i64;
    for table in tables {
        uncovered += count_uncovered_fleet_table(pool, &table).await?;
    }
    Ok(uncovered)
}

async fn count_uncovered_fleet_table(pool: &PgPool, table: &str) -> Result<i64, StorageError> {
    // Entity: provenance only
    let entity_sql = format!(
        "SELECT count(*) FROM public.{table} v \
         WHERE v.id LIKE 'entity:%' \
           AND NOT EXISTS ( \
                SELECT 1 FROM public.entity_embeddings ee \
                WHERE ee.legacy_vector_id = v.id)"
    );
    // Relationship: provenance only
    let rel_sql = format!(
        "SELECT count(*) FROM public.{table} v \
         WHERE v.id ~ '^.+->.+:.+$' \
           AND v.id NOT LIKE 'entity:%' \
           AND v.id NOT LIKE 'community_report:%' \
           AND NOT EXISTS ( \
                SELECT 1 FROM public.relationship_embeddings re \
                WHERE re.legacy_vector_id = v.id)"
    );
    // Report: provenance or report_id
    let report_sql = format!(
        "SELECT count(*) FROM public.{table} v \
         WHERE v.id LIKE 'community_report:%' \
           AND NOT EXISTS ( \
                SELECT 1 FROM public.report_embeddings re \
                WHERE re.legacy_vector_id = v.id OR re.report_id = v.id)"
    );

    let mut n = 0i64;
    for (label, sql) in [
        ("entity", entity_sql),
        ("rel", rel_sql),
        ("report", report_sql),
    ] {
        match sqlx::query_scalar::<_, i64>(&sql).fetch_one(pool).await {
            Ok(c) => n += c,
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => {}
            Err(sqlx::Error::Database(db))
                if db.code().as_deref() == Some("42703")
                    || db.message().contains("legacy_vector_id") =>
            {
                // Pre-143: every fleet row of this family is uncovered.
                let fallback = match label {
                    "entity" => {
                        format!("SELECT count(*) FROM public.{table} WHERE id LIKE 'entity:%'")
                    }
                    "rel" => format!(
                        "SELECT count(*) FROM public.{table} \
                         WHERE id ~ '^.+->.+:.+$' AND id NOT LIKE 'entity:%' \
                           AND id NOT LIKE 'community_report:%'"
                    ),
                    _ => format!(
                        "SELECT count(*) FROM public.{table} WHERE id LIKE 'community_report:%'"
                    ),
                };
                n += sqlx::query_scalar::<_, i64>(&fallback)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "coverage uncovered fleet {label} failed: {e}"
                )))
            }
        }
    }
    Ok(n)
}

/// Count how many of the given legacy ids are **drop-covered** (provenance).
pub async fn count_covered_legacy_ids(
    pool: &PgPool,
    ids: &[(String, Option<Uuid>, EmbeddingFamily)],
) -> Result<i64, StorageError> {
    let mut covered = 0i64;
    for (id, _ws, family) in ids {
        if fleet_row_drop_covered(pool, id, *family).await? {
            covered += 1;
        }
    }
    Ok(covered)
}

/// Whether a typed embedding exists for a normalize-resolved spine (write path).
/// Not used for drop readiness.
pub async fn fleet_row_join_resolvable(
    pool: &PgPool,
    legacy_id: &str,
    family: EmbeddingFamily,
    workspace_id: Option<Uuid>,
) -> Result<bool, StorageError> {
    match family {
        EmbeddingFamily::Entity => {
            let Some(name) = entity_name_from_legacy_id(legacy_id) else {
                return Ok(false);
            };
            let Some(ws) = workspace_id else {
                return Ok(false);
            };
            let index = load_entity_name_index_pool(pool, ws).await?;
            let Some(eid) = index.resolve(name) else {
                return Ok(false);
            };
            let n: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM public.entity_embeddings WHERE entity_id = $1)",
            )
            .bind(eid)
            .fetch_one(pool)
            .await
            .map_err(|e| StorageError::Database(format!("coverage join resolvable: {e}")))?;
            Ok(n)
        }
        EmbeddingFamily::Relationship => {
            let Some(ws) = workspace_id else {
                return Ok(false);
            };
            let index = load_entity_name_index_pool(pool, ws).await?;
            // SPEC-133: index-guided parse when endpoint names contain `->`.
            let Some((src, tgt, rel_type)) = index.parse_relationship_legacy_key(legacy_id) else {
                return Ok(false);
            };
            let n = resolve_relationship_id_pool(pool, ws, &src, &tgt, &rel_type, &index)
                .await?
                .is_some();
            if !n {
                return Ok(false);
            }
            // Also need an embedding row on that relationship
            let rid = resolve_relationship_id_pool(pool, ws, &src, &tgt, &rel_type, &index)
                .await?
                .expect("just checked");
            let has: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM public.relationship_embeddings WHERE relationship_id = $1)",
            )
            .bind(rid)
            .fetch_one(pool)
            .await
            .map_err(|e| StorageError::Database(format!("coverage rel embed: {e}")))?;
            Ok(has)
        }
        EmbeddingFamily::Report => {
            let n: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM public.report_embeddings WHERE report_id = $1)",
            )
            .bind(legacy_id)
            .fetch_one(pool)
            .await
            .map_err(|e| StorageError::Database(format!("coverage report resolvable: {e}")))?;
            Ok(n)
        }
    }
}

/// Count dual-legacy→one-typed collision stalls.
///
/// A stall is an uncovered legacy entity/rel key that normalize-resolves to a
/// typed embedding whose `legacy_vector_id` is already set to a **different**
/// id (unique `legacy_vector_id` prevents stamping the alias).
pub async fn count_provenance_stall_rows(pool: &PgPool) -> Result<i64, StorageError> {
    let sample = sample_provenance_stall_ids(pool, 10_000).await?;
    Ok(sample.len() as i64)
}

/// Sample stall legacy ids (capped) for ops / e2e asserts.
pub async fn sample_provenance_stall_ids(
    pool: &PgPool,
    limit: usize,
) -> Result<Vec<String>, StorageError> {
    let tables = list_vector_tables(pool).await?;
    let mut out = Vec::new();
    let mut index_cache: std::collections::HashMap<Uuid, EntityNameIndex> =
        std::collections::HashMap::new();

    for table in tables {
        if out.len() >= limit {
            break;
        }
        // Entity stalls
        let entity_sql = format!(
            "SELECT id, metadata FROM public.{table} v \
             WHERE v.id LIKE 'entity:%' \
               AND NOT EXISTS ( \
                    SELECT 1 FROM public.entity_embeddings ee \
                    WHERE ee.legacy_vector_id = v.id) \
             ORDER BY v.id LIMIT $1"
        );
        let rows = match sqlx::query_as::<_, (String, Option<serde_json::Value>)>(&entity_sql)
            .bind((limit - out.len()) as i64)
            .fetch_all(pool)
            .await
        {
            Ok(r) => r,
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => continue,
            Err(sqlx::Error::Database(db))
                if db.code().as_deref() == Some("42703")
                    || db.message().contains("legacy_vector_id") =>
            {
                continue;
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "stall sample entity failed: {e}"
                )))
            }
        };
        for (id, meta) in rows {
            if out.len() >= limit {
                break;
            }
            let Some(ws) = meta.as_ref().and_then(parse_workspace_from_meta) else {
                continue;
            };
            if let std::collections::hash_map::Entry::Vacant(e) = index_cache.entry(ws) {
                e.insert(load_entity_name_index_pool(pool, ws).await?);
            }
            let index = index_cache.get(&ws).expect("just inserted");
            let Some(name) = entity_name_from_legacy_id(&id) else {
                continue;
            };
            let Some(eid) = index.resolve(name) else {
                continue;
            };
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT legacy_vector_id FROM public.entity_embeddings \
                 WHERE entity_id = $1 AND legacy_vector_id IS NOT NULL LIMIT 1",
            )
            .bind(eid)
            .fetch_optional(pool)
            .await
            .map_err(|e| StorageError::Database(format!("stall entity lookup: {e}")))?;
            if let Some(ref other) = existing {
                if other.as_str() != id.as_str() {
                    out.push(id);
                }
            }
        }

        // Relationship stalls (same idea)
        if out.len() >= limit {
            break;
        }
        let rel_sql = format!(
            "SELECT id, metadata FROM public.{table} v \
             WHERE v.id ~ '^.+->.+:.+$' \
               AND v.id NOT LIKE 'entity:%' \
               AND v.id NOT LIKE 'community_report:%' \
               AND NOT EXISTS ( \
                    SELECT 1 FROM public.relationship_embeddings re \
                    WHERE re.legacy_vector_id = v.id) \
             ORDER BY v.id LIMIT $1"
        );
        let rel_rows = match sqlx::query_as::<_, (String, Option<serde_json::Value>)>(&rel_sql)
            .bind((limit - out.len()) as i64)
            .fetch_all(pool)
            .await
        {
            Ok(r) => r,
            Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => continue,
            Err(sqlx::Error::Database(db))
                if db.code().as_deref() == Some("42703")
                    || db.message().contains("legacy_vector_id") =>
            {
                continue;
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "stall sample rel failed: {e}"
                )))
            }
        };
        for (id, meta) in rel_rows {
            if out.len() >= limit {
                break;
            }
            let Some(ws) = meta.as_ref().and_then(parse_workspace_from_meta) else {
                continue;
            };
            if let std::collections::hash_map::Entry::Vacant(e) = index_cache.entry(ws) {
                e.insert(load_entity_name_index_pool(pool, ws).await?);
            }
            let index = index_cache.get(&ws).expect("just inserted");
            // SPEC-133: index-guided parse when endpoint names contain `->`.
            let Some((src, tgt, rel_type)) = index.parse_relationship_legacy_key(&id) else {
                continue;
            };
            let Some(rid) =
                resolve_relationship_id_pool(pool, ws, &src, &tgt, &rel_type, index).await?
            else {
                continue;
            };
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT legacy_vector_id FROM public.relationship_embeddings \
                 WHERE relationship_id = $1 AND legacy_vector_id IS NOT NULL LIMIT 1",
            )
            .bind(rid)
            .fetch_optional(pool)
            .await
            .map_err(|e| StorageError::Database(format!("stall rel lookup: {e}")))?;
            if let Some(ref other) = existing {
                if other.as_str() != id.as_str() {
                    out.push(id);
                }
            }
        }
    }
    Ok(out)
}

/// Parse workspace_id from legacy vector metadata JSON.
pub fn parse_workspace_from_meta(meta: &serde_json::Value) -> Option<Uuid> {
    meta.get("workspace_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

/// Resolve workspace: metadata JSON first, then first-class column (SPEC-111).
pub fn resolve_workspace_id(
    meta: Option<&serde_json::Value>,
    column_workspace_id: Option<&str>,
) -> Option<Uuid> {
    if let Some(ws) = meta.and_then(parse_workspace_from_meta) {
        return Some(ws);
    }
    column_workspace_id.and_then(|s| Uuid::parse_str(s.trim()).ok())
}

/// Stamp-job verify counts: only rows that are **stampable**.
///
/// Stampable = already provenance-covered **or** join-resolvable to an existing
/// typed embedding. Orphan fleet rows without a typed spine are **not** this
/// job's expected set (iw2 backfill / wipe residue) — counting them made
/// provenance-stamp fail with `expected=N actual=0` forever.
pub async fn count_stamp_verify_coverage(pool: &PgPool) -> Result<(i64, i64), StorageError> {
    let tables = list_vector_tables(pool).await?;
    let mut expected = 0i64;
    let mut actual = 0i64;
    for table in &tables {
        for family in EmbeddingFamily::FLEET_BACKFILL_FAMILIES {
            let (exp, act) = count_stamp_verify_family(pool, table, family).await?;
            expected += exp;
            actual += act;
        }
    }
    Ok((expected, actual))
}

async fn count_stamp_verify_family(
    pool: &PgPool,
    table: &str,
    family: EmbeddingFamily,
) -> Result<(i64, i64), StorageError> {
    let filter = match family {
        EmbeddingFamily::Entity => "id LIKE 'entity:%'",
        EmbeddingFamily::Relationship => {
            "id ~ '^.+->.+:.+$' AND id NOT LIKE 'entity:%' AND id NOT LIKE 'community_report:%'"
        }
        EmbeddingFamily::Report => "id LIKE 'community_report:%'",
    };
    let has_col = table_has_workspace_id_col(pool, table)
        .await
        .unwrap_or_default();
    let sql = if has_col {
        format!(
            "SELECT id, metadata, workspace_id::text AS workspace_id FROM public.{table} WHERE {filter}"
        )
    } else {
        format!("SELECT id, metadata FROM public.{table} WHERE {filter}")
    };

    let rows = match sqlx::query(&sql).fetch_all(pool).await {
        Ok(r) => r,
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => {
            return Ok((0, 0));
        }
        Err(e) => {
            return Err(StorageError::Database(format!(
                "stamp verify scan {family:?}: {e}"
            )))
        }
    };
    let mut expected = 0i64;
    let mut actual = 0i64;
    for row in rows {
        let id: String = row
            .try_get("id")
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let meta: Option<serde_json::Value> = row.try_get("metadata").ok();
        let col_ws: Option<String> = if has_col {
            row.try_get("workspace_id").ok().flatten()
        } else {
            None
        };
        let ws = resolve_workspace_id(meta.as_ref(), col_ws.as_deref());
        if fleet_row_drop_covered(pool, &id, family).await? {
            expected += 1;
            actual += 1;
            continue;
        }
        if fleet_row_join_resolvable(pool, &id, family, ws).await? {
            expected += 1;
        }
    }
    Ok((expected, actual))
}

/// Whether `public.{table}` has a first-class `workspace_id` column.
async fn table_has_workspace_id_col(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    table: &str,
) -> Result<bool, StorageError> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = $1
              AND column_name = 'workspace_id'
         )",
    )
    .bind(table)
    .fetch_one(executor)
    .await
    .map_err(|e| StorageError::Database(format!("workspace_id column probe: {e}")))
}

/// Fetch one batch of legacy fleet ids for stamping (keyset).
///
/// Returns `(id, metadata, column_workspace_id)`.
pub async fn scan_fleet_stamp_batch(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    family: EmbeddingFamily,
    last_id: &str,
    limit: i64,
) -> Result<Vec<(String, Option<serde_json::Value>, Option<String>)>, StorageError> {
    let filter = match family {
        EmbeddingFamily::Entity => "id LIKE 'entity:%'",
        EmbeddingFamily::Relationship => {
            "id LIKE '%->%:%' AND id NOT LIKE 'entity:%' AND id NOT LIKE 'community_report:%'"
        }
        EmbeddingFamily::Report => "id LIKE 'community_report:%'",
    };
    let has_col = table_has_workspace_id_col(&mut **tx, table).await?;
    let sql = if has_col {
        format!(
            "SELECT id, metadata, workspace_id::text AS workspace_id FROM public.{table} \
             WHERE {filter} AND id > $1 ORDER BY id LIMIT $2"
        )
    } else {
        format!(
            "SELECT id, metadata FROM public.{table} \
             WHERE {filter} AND id > $1 ORDER BY id LIMIT $2"
        )
    };

    let rows = match sqlx::query(&sql)
        .bind(last_id)
        .bind(limit)
        .fetch_all(&mut **tx)
        .await
    {
        Ok(r) => r,
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => {
            return Ok(Vec::new());
        }
        Err(e) => {
            return Err(StorageError::Database(format!(
                "stamp scan {family:?} failed: {e}"
            )))
        }
    };
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row
            .try_get("id")
            .map_err(|e| StorageError::Database(e.to_string()))?;
        let meta: Option<serde_json::Value> = row.try_get("metadata").ok();
        let col_ws: Option<String> = if has_col {
            row.try_get("workspace_id").ok().flatten()
        } else {
            None
        };
        out.push((id, meta, col_ws));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_spec396_uncovered_chunk_split_total() {
        let s = UncoveredChunkSplit {
            missing_spine: 3,
            missing_embedding: 5,
        };
        assert_eq!(s.total(), 8);
        let mut acc = UncoveredChunkSplit::default();
        acc.add(s);
        acc.add(UncoveredChunkSplit {
            missing_spine: 1,
            missing_embedding: 0,
        });
        assert_eq!(acc.total(), 9);
        assert_eq!(acc.missing_spine, 4);
    }

    #[test]
    fn resolve_display_name_to_normalized_legacy_key() {
        let acme = Uuid::new_v4();
        let index = EntityNameIndex::from_rows([(acme, "Acme Corp Ltd".into())]);
        assert_eq!(index.resolve("ACME_CORP_LTD"), Some(acme));
        assert_eq!(index.resolve("Acme Corp Ltd"), Some(acme));
        assert_eq!(normalize_entity_name("Acme Corp Ltd"), "ACME_CORP_LTD");
    }

    #[test]
    fn resolve_already_normalized_spine() {
        let id = Uuid::new_v4();
        let index = EntityNameIndex::from_rows([(id, "SARAH_CHEN".into())]);
        assert_eq!(index.resolve("SARAH_CHEN"), Some(id));
        assert_eq!(index.resolve("Sarah Chen"), Some(id));
    }

    #[test]
    fn resolve_empty_or_opaque_unresolved() {
        let index = EntityNameIndex::from_rows([(Uuid::new_v4(), "ok".into())]);
        assert_eq!(index.resolve(""), None);
        assert!(normalize_entity_name("84b69e27-e38b-444a-83dd-5e6a537c6f12").is_empty());
    }

    #[test]
    fn resolve_workspace_prefixed_spine_name() {
        let id = Uuid::new_v4();
        let ws = Uuid::new_v4();
        let index = EntityNameIndex::from_rows([(id, format!("{ws}::Acme Corp Ltd"))]);
        assert_eq!(index.resolve("ACME_CORP_LTD"), Some(id));
        assert_eq!(index.resolve("Acme Corp Ltd"), Some(id));
    }

    /// SPEC-133 DRY: index wrapper disambiguates target-arrow keys.
    #[test]
    fn contract_spec133_index_parse_relationship_legacy_key() {
        let src_id = Uuid::new_v4();
        let tgt_id = Uuid::new_v4();
        let src = "FLOW_DIRECTION";
        let tgt = "ARROW_1_(SHADED_BOX_->CIRCULAR_TARGET)";
        let index = EntityNameIndex::from_rows([(src_id, src.into()), (tgt_id, tgt.into())]);
        let key = crate::format_relationship_legacy_key(src, tgt, "RELATED_TO");
        assert_eq!(
            index.parse_relationship_legacy_key(&key),
            Some((src.into(), tgt.into(), "RELATED_TO".into()))
        );
    }
}
