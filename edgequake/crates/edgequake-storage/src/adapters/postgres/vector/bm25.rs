//! SPEC-180: Okapi BM25 over the per-workspace postings (`chunk_terms`).
//!
//! Terms come from the same `to_tsvector(workspaces.fts_regconfig, ...)` the migration-225
//! triggers use at index time, so a query term can only match what was indexed. Candidate
//! chunks are the ones that contain any query term (OR semantics, never AND), ordered by a
//! cheap `Σ idf·tf` and cut to `candidate_cap`. Only those get the full length-normalised
//! score. `idf` is the Lucene form `ln(1 + (N - df + 0.5) / (df + 0.5))`; `k1 = 1.5`,
//! `b = 0.75`. `ts_rank_cd` is not used here.

use sqlx::Row;
use uuid::Uuid;

use super::typed_read::resolve_workspace_uuid;
use super::PgVectorStorage;
use crate::adapters::postgres::statement_timeout::{
    vector_query_statement_timeout_ms, LocalTimeoutTx,
};
use crate::error::{Result, StorageError};
use crate::traits::{Bm25Request, ChunkKinds, MetadataFilter, VectorSearchResult};

/// BM25 `k1`. The SQL carries the literal; this pins it in a test.
#[cfg(test)]
const BM25_K1: f64 = 1.5;
/// BM25 `b`. The SQL carries the literal; this pins it in a test.
#[cfg(test)]
const BM25_B: f64 = 0.75;
/// Upper bound on chunks scored per query (SPEC-180 09-retrieval).
pub const BM25_CANDIDATE_CEILING: usize = 500;

/// One statement: tokenize, weigh, pick candidates, score. Binds:
/// `$1` workspace, `$2` query text, `$3` top_k, `$4` candidate cap, `$5` document ids,
/// `$6` tenant, `$7` chunk modality, `$8` source modality, `$9` doc kinds, `$10` MIME types.
pub(crate) const BM25_SQL: &str = r#"
WITH cfg AS (
    SELECT w.fts_regconfig::regconfig AS rc,
           greatest(w.bm25_doc_count, 1)::float8 AS n,
           greatest(w.bm25_total_len, 1)::float8 / greatest(w.bm25_doc_count, 1) AS avgdl
    FROM public.workspaces w
    WHERE w.workspace_id = $1
),
qterms AS (
    SELECT DISTINCT t.lexeme AS term
    FROM cfg, unnest(to_tsvector(cfg.rc, $2)) AS t(lexeme, positions, weights)
),
idf AS (
    SELECT q.term,
           ln(1 + (cfg.n - s.df + 0.5) / (s.df + 0.5)) AS w
    FROM qterms q
    CROSS JOIN cfg
    JOIN public.workspace_term_stats s ON s.workspace_id = $1 AND s.term = q.term
    WHERE s.df > 0
),
hits AS (
    SELECT ct.chunk_id, sum(i.w * ct.tf) AS pre
    FROM public.chunk_terms ct
    JOIN idf i ON i.term = ct.term
    JOIN public.chunks c ON c.id = ct.chunk_id
    JOIN public.documents d ON d.id = c.document_id
    WHERE ct.workspace_id = $1
      AND ($5::uuid[] IS NULL OR c.document_id = ANY($5))
      AND ($6::uuid IS NULL OR c.tenant_id = $6)
      AND ($7::text[] IS NULL OR c.modality = ANY($7))
      AND ($8::text[] IS NULL OR c.source_modality = ANY($8))
      AND ($9::text[] IS NULL OR d.doc_kind = ANY($9))
      AND ($10::text[] IS NULL OR lower(d.content_type) = ANY($10))
    GROUP BY ct.chunk_id
    ORDER BY pre DESC, ct.chunk_id
    LIMIT $4
),
lens AS (
    SELECT ct.chunk_id, sum(ct.tf)::float8 AS len
    FROM public.chunk_terms ct
    JOIN hits h ON h.chunk_id = ct.chunk_id
    GROUP BY ct.chunk_id
),
scored AS (
    SELECT ct.chunk_id,
           sum(i.w * (ct.tf * (1.5 + 1.0))
               / (ct.tf + 1.5 * (1 - 0.75 + 0.75 * l.len / cfg.avgdl))) AS score
    FROM public.chunk_terms ct
    JOIN hits h ON h.chunk_id = ct.chunk_id
    JOIN idf i ON i.term = ct.term
    JOIN lens l ON l.chunk_id = ct.chunk_id
    CROSS JOIN cfg
    GROUP BY ct.chunk_id
)
SELECT coalesce(c.metadata->>'legacy_chunk_key', c.id::text) AS id,
       c.metadata,
       s.score::float4 AS score
FROM scored s
JOIN public.chunks c ON c.id = s.chunk_id
ORDER BY s.score DESC, s.chunk_id
LIMIT $3
"#;

/// Clamp the candidate window to the contract: at least `top_k`, at most 500.
pub fn clamp_candidate_cap(top_k: usize, requested: usize) -> usize {
    requested.max(top_k).clamp(1, BM25_CANDIDATE_CEILING)
}

fn parse_uuid_list(ids: &Option<Vec<String>>) -> Option<Option<Vec<Uuid>>> {
    match ids {
        Some(ids) if !ids.is_empty() => {
            let parsed: Vec<Uuid> = ids.iter().filter_map(|i| Uuid::parse_str(i).ok()).collect();
            if parsed.is_empty() {
                None
            } else {
                Some(Some(parsed))
            }
        }
        _ => Some(None),
    }
}

fn lowercase_list(list: &Option<Vec<String>>) -> Option<Vec<String>> {
    list.as_ref()
        .map(|l| l.iter().map(|v| v.trim().to_ascii_lowercase()).collect())
}

impl PgVectorStorage {
    /// BM25 over `chunk_terms` for one workspace. `Ok(None)` outside the typed backend.
    pub(crate) async fn typed_chunks_bm25_search(
        &self,
        req: &Bm25Request<'_>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Option<Vec<VectorSearchResult>>> {
        if !crate::vector_backend::vector_backend_reads_typed(crate::vector_backend_from_env()) {
            return Ok(None);
        }
        let empty = Ok(Some(Vec::new()));
        if req.query_text.trim().is_empty() || req.top_k == 0 {
            return empty;
        }
        let mf = metadata_filter.cloned().unwrap_or_default();
        let kinds = mf.kinds.clone().unwrap_or_default();
        if kinds.matches_nothing() {
            return empty;
        }
        let Some(workspace) = mf.workspace_id.as_deref() else {
            return empty;
        };
        let pool = self.pool.get().await?;
        let Some(workspace) = resolve_workspace_uuid(&pool, workspace).await? else {
            return empty;
        };
        let Some(document_ids) = parse_uuid_list(&mf.document_ids) else {
            return empty;
        };
        let tenant = match mf.tenant_id.as_deref() {
            Some(t) => match Uuid::parse_str(t) {
                Ok(u) => Some(u),
                Err(_) => return empty,
            },
            None => None,
        };
        let rows = self
            .run_bm25(&pool, req, workspace, document_ids, tenant, &mf, &kinds)
            .await?;
        Ok(Some(rows))
    }

    #[allow(clippy::too_many_arguments)] // one bind per SQL parameter; a wrapper would only rename them
    async fn run_bm25(
        &self,
        pool: &sqlx::PgPool,
        req: &Bm25Request<'_>,
        workspace: Uuid,
        document_ids: Option<Vec<Uuid>>,
        tenant: Option<Uuid>,
        mf: &MetadataFilter,
        kinds: &ChunkKinds,
    ) -> Result<Vec<VectorSearchResult>> {
        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| StorageError::Database(format!("BM25 acquire failed: {e}")))?;
        let mut timed = LocalTimeoutTx::begin(&mut conn, vector_query_statement_timeout_ms())
            .await
            .map_err(|e| StorageError::Database(format!("BM25 begin failed: {e}")))?;
        sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
            .execute(timed.as_mut())
            .await
            .map_err(|e| StorageError::Database(format!("BM25 plan mode failed: {e}")))?;
        let cap = clamp_candidate_cap(req.top_k, req.candidate_cap);
        let rows = sqlx::query(BM25_SQL)
            .bind(workspace)
            .bind(req.query_text)
            .bind(req.top_k as i64)
            .bind(cap as i64)
            .bind(document_ids.as_deref())
            .bind(tenant)
            .bind(mf.modalities.as_deref().filter(|m| !m.is_empty()))
            .bind(lowercase_list(&kinds.source_modalities))
            .bind(lowercase_list(&kinds.doc_kinds))
            .bind(lowercase_list(&kinds.mime_types))
            .fetch_all(timed.as_mut())
            .await
            .map_err(|e| StorageError::Database(format!("BM25 query failed: {e}")))?;
        timed
            .commit()
            .await
            .map_err(|e| StorageError::Database(format!("BM25 commit failed: {e}")))?;
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
    fn sql_uses_the_documented_constants_and_or_semantics() {
        assert!(BM25_SQL.contains("ln(1 + (cfg.n - s.df + 0.5) / (s.df + 0.5))"));
        assert!(BM25_SQL.contains("ct.tf * (1.5 + 1.0)"));
        assert!(BM25_SQL.contains("1.5 * (1 - 0.75 + 0.75 * l.len / cfg.avgdl)"));
        assert!(!BM25_SQL.contains("ts_rank"));
        assert!(!BM25_SQL.contains("&"), "AND semantics are not allowed");
        assert_eq!(BM25_K1, 1.5);
        assert_eq!(BM25_B, 0.75);
    }

    #[test]
    fn candidate_window_is_clamped() {
        assert_eq!(clamp_candidate_cap(10, 0), 10);
        assert_eq!(clamp_candidate_cap(10, 120), 120);
        assert_eq!(clamp_candidate_cap(10, 9000), BM25_CANDIDATE_CEILING);
        assert_eq!(clamp_candidate_cap(900, 20), BM25_CANDIDATE_CEILING);
    }

    #[test]
    fn document_list_parsing_distinguishes_none_empty_and_garbage() {
        assert_eq!(parse_uuid_list(&None), Some(None));
        assert_eq!(parse_uuid_list(&Some(vec![])), Some(None));
        assert_eq!(parse_uuid_list(&Some(vec!["nope".into()])), None);
        let id = Uuid::new_v4();
        assert_eq!(
            parse_uuid_list(&Some(vec![id.to_string()])),
            Some(Some(vec![id]))
        );
    }

    #[test]
    fn lists_are_lowercased() {
        assert_eq!(
            lowercase_list(&Some(vec!["PDF".into(), " Text/Markdown ".into()])),
            Some(vec!["pdf".into(), "text/markdown".into()])
        );
    }
}
