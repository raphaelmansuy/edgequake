//! PostgreSQL native FTS for chunk sparse retrieval (SPEC-023 I10).
//!
//! Uses GIN-indexed `content_tsv` + `ts_rank_cd` (cover-density ranking — **not**
//! Okapi BM25; see SPEC-083 X-05) instead of re-scoring vector candidates in
//! application memory.
//!
//! SPEC-105 / LAW-L3: KV LEFT JOIN is **era-aware** — only when
//! `chunk_kv_table_exists` (≤0.22 mid-upgrade). Post-125 census, content comes
//! from `content_tsv` / metadata only (typed chunks SSOT via serving path).
//!
//! #405 / SPEC-091: under typed vector backend, FTS reads `public.chunks.content_tsv`
//! (migration 136) — never retired `eq_*_vectors`. Legacy path keeps
//! `EDGEQUAKE_FTS_LANGUAGE` for rollback only.

use sqlx::Row;
use uuid::Uuid;

use super::typed_read::resolve_workspace_uuid;
use super::PgVectorStorage;
use crate::adapters::postgres::schema;
use crate::adapters::postgres::statement_timeout::{
    vector_query_statement_timeout_ms, LocalTimeoutTx,
};
use crate::error::{Result, StorageError};
use crate::traits::{MetadataFilter, VectorSearchResult};

/// Env key for Postgres text-search configuration name (X-05).
pub const FTS_LANGUAGE_ENV: &str = "EDGEQUAKE_FTS_LANGUAGE";

/// Default Postgres text-search config when env is unset.
pub const DEFAULT_FTS_LANGUAGE: &str = "english";

/// Sanitize a Postgres `regconfig` name: lowercase ASCII letters only.
///
/// Rejects anything that could break out of a SQL string literal.
pub fn sanitize_fts_language(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase();
    if lower.is_empty()
        || !lower.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        || lower.len() > 32
    {
        return DEFAULT_FTS_LANGUAGE.to_string();
    }
    lower
}

/// Resolve FTS language from `EDGEQUAKE_FTS_LANGUAGE` (default `english`).
pub fn fts_language_from_env() -> String {
    match std::env::var(FTS_LANGUAGE_ENV) {
        Ok(v) => sanitize_fts_language(&v),
        Err(_) => DEFAULT_FTS_LANGUAGE.to_string(),
    }
}

fn fts_content_expr(join_kv: bool, lang: &str) -> String {
    if join_kv {
        format!(
            "coalesce(NULLIF(v.content_tsv, ''::tsvector), to_tsvector('{lang}', coalesce(v.metadata->>'content', k.value->>'content', '')))"
        )
    } else {
        format!(
            "coalesce(NULLIF(v.content_tsv, ''::tsvector), to_tsvector('{lang}', coalesce(v.metadata->>'content', '')))"
        )
    }
}

/// SQL for typed chunk FTS (migration 136 `chunks.content_tsv`). Exposed for
/// contract tests — must not reference legacy `eq_*_vectors`.
pub(crate) const TYPED_CHUNKS_FTS_SQL: &str = r#"
            SELECT coalesce(c.metadata->>'legacy_chunk_key', c.id::text) AS id,
                   c.metadata,
                   ts_rank_cd(
                       c.content_tsv,
                       websearch_to_tsquery('english', $1)
                   )::float4 AS score
            FROM public.chunks c
            JOIN public.documents d ON d.id = c.document_id
            WHERE c.content_tsv @@ websearch_to_tsquery('english', $1)
              AND ($2::uuid IS NULL
                   OR d.workspace_id = $2
                   OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))
              AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))
              AND ($5::uuid IS NULL OR c.tenant_id = $5)
              AND ($6::text[] IS NULL OR c.modality = ANY($6))
              AND (
                    $7::text[] IS NULL
                    OR c.id::text = ANY($7)
                    OR c.metadata->>'legacy_chunk_key' = ANY($7)
                  )
            ORDER BY score DESC
            LIMIT $8
            "#;

/// Workspace-scoped typed FTS. `$2` is required, so a generic plan can
/// bitmap-and `idx_chunks_workspace` with the content GIN. Chunks with a
/// null `workspace_id` still use the document predicate. An unscoped search
/// keeps [`TYPED_CHUNKS_FTS_SQL`].
///
/// A common term matches every chunk in the workspace. A generic plan still
/// estimates a few hundred hits, so it looks up every document before `LIMIT`.
/// The search replans with `force_custom_plan`, which scores the chunks and
/// then reads only the page of documents. `OFFSET 0` applies the workspace
/// and tenant predicates before `@@`. `@@` is not leakproof, so without that
/// fence the tenant role repeats the tenant bitmap once per text hit.
pub(crate) const TYPED_CHUNKS_FTS_SQL_IN_WORKSPACE: &str = r#"
            SELECT coalesce(c.metadata->>'legacy_chunk_key', c.id::text) AS id,
                   c.metadata,
                   ts_rank_cd(
                       c.content_tsv,
                       websearch_to_tsquery('english', $1)
                   )::float4 AS score
            FROM (
                SELECT *
                FROM public.chunks c
                WHERE (c.workspace_id = $2 OR c.workspace_id IS NULL)
                  AND ($5::uuid IS NULL OR c.tenant_id = $5)
                OFFSET 0
            ) c
            JOIN public.documents d ON d.id = c.document_id
            WHERE c.content_tsv @@ websearch_to_tsquery('english', $1)
              AND (c.workspace_id = $2 OR c.workspace_id IS NULL)
              AND (d.workspace_id = $2
                   OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))
              AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))
              AND ($5::uuid IS NULL OR c.tenant_id = $5)
              AND ($6::text[] IS NULL OR c.modality = ANY($6))
              AND (
                    $7::text[] IS NULL
                    OR c.id::text = ANY($7)
                    OR c.metadata->>'legacy_chunk_key' = ANY($7)
                  )
            ORDER BY score DESC
            LIMIT $8
            "#;

/// Document-scoped typed FTS. `$4` is a required non-empty array, so a
/// generic plan can start from `idx_chunks_document` instead of every chunk
/// that matches the term. Workspace scope stays optional.
pub(crate) const TYPED_CHUNKS_FTS_SQL_IN_DOCUMENTS: &str = r#"
            SELECT coalesce(c.metadata->>'legacy_chunk_key', c.id::text) AS id,
                   c.metadata,
                   ts_rank_cd(
                       c.content_tsv,
                       websearch_to_tsquery('english', $1)
                   )::float4 AS score
            FROM public.chunks c
            JOIN public.documents d ON d.id = c.document_id
            WHERE c.content_tsv @@ websearch_to_tsquery('english', $1)
              AND ($2::uuid IS NULL
                   OR d.workspace_id = $2
                   OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))
              AND c.document_id = ANY($4)
              AND ($5::uuid IS NULL OR c.tenant_id = $5)
              AND ($6::text[] IS NULL OR c.modality = ANY($6))
              AND (
                    $7::text[] IS NULL
                    OR c.id::text = ANY($7)
                    OR c.metadata->>'legacy_chunk_key' = ANY($7)
                  )
            ORDER BY score DESC
            LIMIT $8
            "#;

/// `$6 IS NULL OR modality = ANY($6)` hides the modality predicate from a
/// generic plan. Chart search then scores every chunk that matches the words
/// and throws away the other modalities. When the caller passes modalities,
/// the statement requires `metadata->>'modality' = ANY($6)` so
/// `idx_chunks_modality` can start at those rows.
const MODALITY_PREDICATE_OPTIONAL: &str = "AND ($6::text[] IS NULL OR c.modality = ANY($6))";
const MODALITY_PREDICATE_REQUIRED: &str = "AND c.modality = ANY($6)";

/// `$5 IS NULL OR tenant_id = $5` hides the tenant from a generic plan.
/// Search then scores every chunk that matches the words and throws away the
/// other tenants. When the caller passes a tenant, the statement requires
/// `tenant_id = $5` so `idx_chunks_tenant_workspace` can intersect the
/// content bitmap before those rows are fetched.
const TENANT_PREDICATE_OPTIONAL: &str = "AND ($5::uuid IS NULL OR c.tenant_id = $5)";
const TENANT_PREDICATE_REQUIRED: &str = "AND c.tenant_id = $5";

/// `$2 IS NULL OR workspace` on the id lookup hides the workspace after the
/// primary key. When the caller passes a workspace, both arms require it.
/// Null `chunks.workspace_id` rows still match through the document predicate.
const ID_WORKSPACE_CHUNK_OPTIONAL: &str =
    "AND ($2::uuid IS NULL OR c.workspace_id = $2 OR c.workspace_id IS NULL)";
const ID_WORKSPACE_CHUNK_REQUIRED: &str = "AND (c.workspace_id = $2 OR c.workspace_id IS NULL)";
const ID_WORKSPACE_DOC_OPTIONAL: &str = "AND ($2::uuid IS NULL OR d.workspace_id = $2 OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))";
const ID_WORKSPACE_DOC_REQUIRED: &str =
    "AND (d.workspace_id = $2 OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))";

/// `$4 IS NULL OR document_id = ANY($4)` on the id lookup is the same shape.
/// When the caller passes documents, the predicate is required.
const ID_DOCUMENT_OPTIONAL: &str = "AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))";
const ID_DOCUMENT_REQUIRED: &str = "AND c.document_id = ANY($4)";

fn apply_scope_predicates(sql: &str, modality_required: bool, tenant_required: bool) -> String {
    let sql = if modality_required {
        sql.replace(MODALITY_PREDICATE_OPTIONAL, MODALITY_PREDICATE_REQUIRED)
    } else {
        sql.to_string()
    };
    if tenant_required {
        sql.replace(TENANT_PREDICATE_OPTIONAL, TENANT_PREDICATE_REQUIRED)
    } else {
        sql
    }
}

pub(crate) fn typed_chunks_fts_sql(
    in_workspace: bool,
    in_documents: bool,
    modality_required: bool,
    tenant_required: bool,
) -> String {
    let base = match (in_workspace, in_documents) {
        (true, true) => TYPED_CHUNKS_FTS_SQL_IN_WORKSPACE_AND_DOCUMENTS,
        (true, false) => TYPED_CHUNKS_FTS_SQL_IN_WORKSPACE,
        (false, true) => TYPED_CHUNKS_FTS_SQL_IN_DOCUMENTS,
        (false, false) => TYPED_CHUNKS_FTS_SQL,
    };
    apply_scope_predicates(base, modality_required, tenant_required)
}

/// Id-filtered statement. Workspace and document stay optional only when the
/// caller did not pass them. The lookup still starts at the chunk id.
fn apply_id_scope(
    workspace_required: bool,
    documents_required: bool,
    modality_required: bool,
    tenant_required: bool,
) -> String {
    let mut sql = TYPED_CHUNKS_FTS_SQL_IN_IDS.to_string();
    if workspace_required {
        sql = sql.replace(ID_WORKSPACE_CHUNK_OPTIONAL, ID_WORKSPACE_CHUNK_REQUIRED);
        sql = sql.replace(ID_WORKSPACE_DOC_OPTIONAL, ID_WORKSPACE_DOC_REQUIRED);
    }
    if documents_required {
        sql = sql.replace(ID_DOCUMENT_OPTIONAL, ID_DOCUMENT_REQUIRED);
    }
    apply_scope_predicates(&sql, modality_required, tenant_required)
}

/// Id-filtered typed FTS. `$7 IS NULL OR id = ANY OR legacy key = ANY` makes a
/// generic plan score every chunk that matches the words. This statement looks
/// up each requested id first. `OFFSET 0` keeps that lookup from being pulled
/// into the term index. Legacy keys compare the stored `legacy_chunk_key`
/// column (migration 203) so forced row security can use
/// `idx_chunks_tenant_legacy_key`. `metadata->>'legacy_chunk_key'` is not
/// leakproof and cannot be that index condition. Values that parse as UUIDs
/// also use `chunks_pkey`. The other filters then run on those rows.
pub(crate) const TYPED_CHUNKS_FTS_SQL_IN_IDS: &str = r#"
            SELECT id, metadata, score
            FROM (
                SELECT coalesce(c.metadata->>'legacy_chunk_key', c.id::text) AS id,
                       c.metadata,
                       ts_rank_cd(
                           c.content_tsv,
                           websearch_to_tsquery('english', $1)
                       )::float4 AS score
                FROM unnest($7::text[]) AS wanted(key)
                JOIN LATERAL (
                    SELECT c.id, c.document_id, c.workspace_id, c.tenant_id,
                           c.metadata, c.content_tsv
                    FROM public.chunks c
                    WHERE c.legacy_chunk_key = wanted.key
                    OFFSET 0
                ) c ON true
                JOIN public.documents d ON d.id = c.document_id
                WHERE c.content_tsv @@ websearch_to_tsquery('english', $1)
                  AND ($2::uuid IS NULL OR c.workspace_id = $2 OR c.workspace_id IS NULL)
                  AND ($2::uuid IS NULL OR d.workspace_id = $2 OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))
                  AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))
                  AND ($5::uuid IS NULL OR c.tenant_id = $5)
                  AND ($6::text[] IS NULL OR c.modality = ANY($6))
                UNION
                SELECT coalesce(c.metadata->>'legacy_chunk_key', c.id::text) AS id,
                       c.metadata,
                       ts_rank_cd(
                           c.content_tsv,
                           websearch_to_tsquery('english', $1)
                       )::float4 AS score
                FROM unnest($8::uuid[]) AS wanted(id)
                JOIN LATERAL (
                    SELECT c.id, c.document_id, c.workspace_id, c.tenant_id,
                           c.metadata, c.content_tsv
                    FROM public.chunks c
                    WHERE c.id = wanted.id
                    OFFSET 0
                ) c ON true
                JOIN public.documents d ON d.id = c.document_id
                WHERE c.content_tsv @@ websearch_to_tsquery('english', $1)
                  AND ($2::uuid IS NULL OR c.workspace_id = $2 OR c.workspace_id IS NULL)
                  AND ($2::uuid IS NULL OR d.workspace_id = $2 OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))
                  AND ($4::uuid[] IS NULL OR c.document_id = ANY($4))
                  AND ($5::uuid IS NULL OR c.tenant_id = $5)
                  AND ($6::text[] IS NULL OR c.modality = ANY($6))
            ) hits
            ORDER BY score DESC
            LIMIT $9
            "#;

/// Workspace and document are both required. Same document index start as
/// [`TYPED_CHUNKS_FTS_SQL_IN_DOCUMENTS`], plus the workspace bitmap predicate.
pub(crate) const TYPED_CHUNKS_FTS_SQL_IN_WORKSPACE_AND_DOCUMENTS: &str = r#"
            SELECT coalesce(c.metadata->>'legacy_chunk_key', c.id::text) AS id,
                   c.metadata,
                   ts_rank_cd(
                       c.content_tsv,
                       websearch_to_tsquery('english', $1)
                   )::float4 AS score
            FROM public.chunks c
            JOIN public.documents d ON d.id = c.document_id
            WHERE c.content_tsv @@ websearch_to_tsquery('english', $1)
              AND (c.workspace_id = $2 OR c.workspace_id IS NULL)
              AND (d.workspace_id = $2
                   OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))
              AND c.document_id = ANY($4)
              AND ($5::uuid IS NULL OR c.tenant_id = $5)
              AND ($6::text[] IS NULL OR c.modality = ANY($6))
              AND (
                    $7::text[] IS NULL
                    OR c.id::text = ANY($7)
                    OR c.metadata->>'legacy_chunk_key' = ANY($7)
                  )
            ORDER BY score DESC
            LIMIT $8
            "#;

impl PgVectorStorage {
    pub(crate) async fn chunk_kv_table_exists_cached(&self) -> Result<bool> {
        if let Some(exists) = self.chunk_kv_table_exists.get() {
            return Ok(*exists);
        }

        let pool = self.pool.get().await?;
        let exists = schema::relation_exists(&pool, &self.chunk_kv_table_name).await?;
        let _ = self.chunk_kv_table_exists.set(exists);
        Ok(exists)
    }

    /// Typed-authority FTS over `public.chunks.content_tsv` (#405 / SPEC-091).
    ///
    /// Regconfig is fixed to `english` (STORED generated column in migration 136).
    /// Returns legacy-shaped ids via `metadata.legacy_chunk_key`.
    pub(crate) async fn typed_chunks_text_search_filtered(
        &self,
        query_text: &str,
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        if query_text.trim().is_empty() || top_k == 0 {
            return Ok(Vec::new());
        }

        let mf = metadata_filter.cloned().unwrap_or_default();
        if let Some(vtype) = mf.vector_type.as_deref() {
            if !vtype.eq_ignore_ascii_case("chunk") {
                // Non-chunk vector_type short-circuit: no entity/rel tsvector.
                return Ok(Vec::new());
            }
        }

        let pool = self.pool.get().await?;

        let workspace_uuid = if let Some(wid) = mf.workspace_id.as_deref() {
            match resolve_workspace_uuid(&pool, wid).await? {
                Some(u) => Some(u),
                None => return Ok(Vec::new()),
            }
        } else {
            None
        };
        let workspace_text = workspace_uuid.map(|u| u.to_string());

        let document_ids = match &mf.document_ids {
            Some(ids) if !ids.is_empty() => {
                let parsed: Vec<Uuid> = ids
                    .iter()
                    .filter_map(|id| Uuid::parse_str(id).ok())
                    .collect();
                if parsed.is_empty() {
                    return Ok(Vec::new());
                }
                Some(parsed)
            }
            _ => None,
        };

        let tenant_id = match mf.tenant_id.as_deref() {
            Some(tid) => match Uuid::parse_str(tid) {
                Ok(u) => Some(u),
                Err(_) => return Ok(Vec::new()),
            },
            None => None,
        };

        let modalities = mf.modalities.clone().filter(|m| !m.is_empty());
        let id_filter = filter_ids
            .filter(|ids| !ids.is_empty())
            .map(|ids| ids.to_vec());
        let uuid_ids: Vec<Uuid> = id_filter
            .iter()
            .flatten()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();

        // A cached generic plan prices `content_tsv @@ $1` as a few hundred
        // hits. A common word then looks up every matching document before
        // LIMIT. Replan with the bound term so the document join stops at
        // the page.
        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| StorageError::Database(format!("Typed chunk FTS acquire failed: {e}")))?;
        let mut timed = LocalTimeoutTx::begin(&mut conn, vector_query_statement_timeout_ms())
            .await
            .map_err(|e| StorageError::Database(format!("Typed chunk FTS begin failed: {e}")))?;
        sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
            .execute(timed.as_mut())
            .await
            .map_err(|e| {
                StorageError::Database(format!("Typed chunk FTS plan mode failed: {e}"))
            })?;

        let rows = if id_filter.is_some() {
            let sql = apply_id_scope(
                workspace_uuid.is_some(),
                document_ids.is_some(),
                modalities.is_some(),
                tenant_id.is_some(),
            );
            sqlx::query(&sql)
                .bind(query_text)
                .bind(workspace_uuid)
                .bind(workspace_text)
                .bind(document_ids.as_deref())
                .bind(tenant_id)
                .bind(modalities.as_deref())
                .bind(id_filter.as_deref())
                .bind(uuid_ids.as_slice())
                .bind(top_k as i32)
                .fetch_all(timed.as_mut())
                .await
        } else {
            let sql = typed_chunks_fts_sql(
                workspace_uuid.is_some(),
                document_ids.is_some(),
                modalities.is_some(),
                tenant_id.is_some(),
            );
            sqlx::query(&sql)
                .bind(query_text)
                .bind(workspace_uuid)
                .bind(workspace_text)
                .bind(document_ids.as_deref())
                .bind(tenant_id)
                .bind(modalities.as_deref())
                .bind(id_filter.as_deref())
                .bind(top_k as i32)
                .fetch_all(timed.as_mut())
                .await
        }
        .map_err(|e| StorageError::Database(format!("Typed chunk FTS query failed: {e}")))?;
        timed
            .commit()
            .await
            .map_err(|e| StorageError::Database(format!("Typed chunk FTS commit failed: {e}")))?;

        Ok(rows
            .iter()
            .map(|row| VectorSearchResult {
                id: row.get("id"),
                score: row.get::<f32, _>("score"),
                metadata: row.get("metadata"),
            })
            .collect())
    }

    /// Full-text search with `ts_rank_cd` over legacy vector chunk content.
    ///
    /// Rollback path when `EDGEQUAKE_VECTOR_BACKEND=legacy_tables`.
    pub(crate) async fn postgres_text_search_filtered(
        &self,
        query_text: &str,
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        if query_text.trim().is_empty() || top_k == 0 {
            return Ok(Vec::new());
        }

        let lang = fts_language_from_env();
        let pool = self.pool.get().await?;
        let join_kv = self.chunk_kv_table_exists_cached().await?;
        let content_expr = fts_content_expr(join_kv, &lang);
        let mf = metadata_filter.cloned().unwrap_or_default();
        let has_id_filter = filter_ids.map(|ids| !ids.is_empty()).unwrap_or(false);
        let filter_sql = mf.build_sql_with_alias(has_id_filter, 2, Some("v"));

        let mut conditions = vec![format!(
            "{content_expr} @@ websearch_to_tsquery('{lang}', $1)"
        )];
        conditions.extend(filter_sql.conditions);

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let kv_join = if join_kv {
            format!(
                "LEFT JOIN {} k ON k.key = coalesce(v.metadata->>'content_ref', v.id)",
                self.chunk_kv_table_name
            )
        } else {
            String::new()
        };

        let sql = format!(
            r#"
            SELECT v.id, v.metadata,
                   ts_rank_cd(
                       {content_expr},
                       websearch_to_tsquery('{lang}', $1)
                   )::float4 AS score
            FROM {vectors} v
            {kv_join}
            {where_clause}
            ORDER BY score DESC
            LIMIT ${limit_param}
            "#,
            content_expr = content_expr,
            lang = lang,
            vectors = self.table_name,
            kv_join = kv_join,
            where_clause = where_clause,
            limit_param = filter_sql.next_param
        );

        use sqlx::postgres::PgArguments;
        use sqlx::Arguments;

        let mut args = PgArguments::default();
        args.add(query_text)
            .map_err(|e| StorageError::Database(format!("Failed to bind FTS query text: {}", e)))?;

        if let Some(ids) = filter_ids {
            if !ids.is_empty() {
                let id_vec: Vec<String> = ids.to_vec();
                args.add(&id_vec).map_err(|e| {
                    StorageError::Database(format!("Failed to bind filter_ids: {}", e))
                })?;
            }
        }

        if let Some(doc_ids) = &mf.document_ids {
            args.add(&doc_ids.clone()).map_err(|e| {
                StorageError::Database(format!("Failed to bind document_ids: {}", e))
            })?;
        }
        if let Some(tid) = &mf.tenant_id {
            args.add(tid)
                .map_err(|e| StorageError::Database(format!("Failed to bind tenant_id: {}", e)))?;
        }
        if let Some(wid) = &mf.workspace_id {
            args.add(wid).map_err(|e| {
                StorageError::Database(format!("Failed to bind workspace_id: {}", e))
            })?;
        }
        if let Some(vtype) = &mf.vector_type {
            args.add(vtype).map_err(|e| {
                StorageError::Database(format!("Failed to bind vector_type: {}", e))
            })?;
        }
        if let Some(modalities) = &mf.modalities {
            let mods: Vec<String> = modalities.clone();
            args.add(&mods)
                .map_err(|e| StorageError::Database(format!("Failed to bind modalities: {}", e)))?;
        }

        args.add(top_k as i32)
            .map_err(|e| StorageError::Database(format!("Failed to bind top_k: {}", e)))?;

        let rows = sqlx::query_with(&sql, args)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Postgres FTS query failed: {}", e)))?;

        Ok(rows
            .iter()
            .map(|row| VectorSearchResult {
                id: row.get("id"),
                score: row.get::<f32, _>("score"),
                metadata: row.get("metadata"),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_typed_chunks_fts_sql_shape() {
        let sql = TYPED_CHUNKS_FTS_SQL;
        assert!(sql.contains("c.content_tsv"));
        assert!(sql.contains("legacy_chunk_key"));
        assert!(sql.contains("websearch_to_tsquery('english'"));
        assert!(sql.contains("ts_rank_cd"));
        assert!(
            !sql.contains("eq_") && !sql.contains("self.table_name"),
            "typed FTS must not reference legacy vectors tables"
        );
        assert!(
            !sql.contains("metadata->>'type'"),
            "typed FTS must not filter metadata type=chunk (relational rows omit it)"
        );
        let scoped = TYPED_CHUNKS_FTS_SQL_IN_WORKSPACE;
        assert!(scoped.contains("c.workspace_id = $2 OR c.workspace_id IS NULL"));
        assert!(scoped.contains("d.workspace_id = $2"));
        assert!(!scoped.contains("$2::uuid IS NULL"));
        for sql in [
            TYPED_CHUNKS_FTS_SQL_IN_DOCUMENTS,
            TYPED_CHUNKS_FTS_SQL_IN_WORKSPACE_AND_DOCUMENTS,
        ] {
            assert!(sql.contains("c.document_id = ANY($4)"));
            assert!(!sql.contains("$4::uuid[] IS NULL"));
        }
        let chart = typed_chunks_fts_sql(true, false, true, false);
        assert!(chart.contains(MODALITY_PREDICATE_REQUIRED));
        assert!(!chart.contains("$6::text[] IS NULL"));
        assert!(chart.contains("c.workspace_id = $2 OR c.workspace_id IS NULL"));
        assert!(chart.contains(TENANT_PREDICATE_OPTIONAL));
        let open = typed_chunks_fts_sql(false, false, false, false);
        assert!(open.contains(MODALITY_PREDICATE_OPTIONAL));
        assert!(open.contains(TENANT_PREDICATE_OPTIONAL));
        let tenant = typed_chunks_fts_sql(true, false, false, true);
        assert!(tenant.contains(TENANT_PREDICATE_REQUIRED));
        assert!(!tenant.contains("$5::uuid IS NULL"));
        let by_id_tenant = apply_scope_predicates(TYPED_CHUNKS_FTS_SQL_IN_IDS, false, true);
        assert!(!by_id_tenant.contains("$5::uuid IS NULL"));
        assert_eq!(by_id_tenant.matches("c.tenant_id = $5").count(), 2);
        let by_id = TYPED_CHUNKS_FTS_SQL_IN_IDS;
        assert!(by_id.contains("OFFSET 0"));
        assert!(by_id.contains("c.legacy_chunk_key = wanted.key"));
        assert!(by_id.contains("c.id = wanted.id"));
        assert!(!by_id.contains("$7::text[] IS NULL"));
        let scoped_ids = apply_id_scope(true, true, false, false);
        assert!(!scoped_ids.contains("$2::uuid IS NULL"));
        assert!(!scoped_ids.contains("$4::uuid[] IS NULL"));
        assert!(scoped_ids.contains("c.workspace_id = $2 OR c.workspace_id IS NULL"));
        assert!(scoped_ids.contains("c.document_id = ANY($4)"));
        assert_eq!(scoped_ids.matches("c.document_id = ANY($4)").count(), 2);
        let open_ids = apply_id_scope(false, false, false, false);
        assert!(open_ids.contains("$2::uuid IS NULL"));
        assert!(open_ids.contains("$4::uuid[] IS NULL"));
    }

    #[test]
    fn e2e_fts_language_config() {
        assert_eq!(sanitize_fts_language("french"), "french");
        assert_eq!(sanitize_fts_language("SIMPLE"), "simple");
        assert_eq!(
            sanitize_fts_language("english'; drop table x"),
            DEFAULT_FTS_LANGUAGE
        );
        assert_eq!(sanitize_fts_language(""), DEFAULT_FTS_LANGUAGE);
        assert_eq!(sanitize_fts_language("fr-FR"), DEFAULT_FTS_LANGUAGE);

        std::env::remove_var(FTS_LANGUAGE_ENV);
        assert_eq!(fts_language_from_env(), DEFAULT_FTS_LANGUAGE);

        std::env::set_var(FTS_LANGUAGE_ENV, "french");
        assert_eq!(fts_language_from_env(), "french");
        let expr = fts_content_expr(false, &fts_language_from_env());
        assert!(
            expr.contains("to_tsvector('french'"),
            "FTS content expr must use configured language: {expr}"
        );
        assert!(
            !expr.contains("BM25"),
            "X-05: postgres FTS path must not claim BM25"
        );
        std::env::remove_var(FTS_LANGUAGE_ENV);
    }
}
