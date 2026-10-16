//! SPEC-091 RM2 — typed `chunks.content_tsv` lexical search (migration 136).

/// Unscoped lexical search. There is no workspace to apply, so the statement
/// does not join `documents` and does not hide a predicate behind `IS NULL`.
#[cfg(feature = "postgres")]
const UNSCOPED_SQL: &str = r#"
        SELECT c.id, c.document_id,
               ts_rank(c.content_tsv, plainto_tsquery('english', $1))::real AS rank
        FROM public.chunks c
        WHERE c.content_tsv @@ plainto_tsquery('english', $1)
        ORDER BY rank DESC
        LIMIT $2
        "#;

/// Workspace-scoped lexical search.
///
/// `$2 IS NULL OR d.workspace_id = $2` is not an index condition. A generic
/// plan prices `content_tsv @@ $1` as a few hundred hits, then looks up every
/// matching document before `LIMIT`. On 39,960 chunks that share the term
/// that was 93.346 ms and 161,286 buffers. This statement requires the
/// workspace. A custom plan scores the chunks and then reads 50 documents
/// (13.693 ms). Null `chunks.workspace_id` rows still match through the
/// document predicate, including a metadata-only workspace.
#[cfg(feature = "postgres")]
const SCOPED_SQL: &str = r#"
        SELECT c.id, c.document_id,
               ts_rank(c.content_tsv, plainto_tsquery('english', $1))::real AS rank
        FROM public.chunks c
        JOIN public.documents d ON d.id = c.document_id
        WHERE c.content_tsv @@ plainto_tsquery('english', $1)
          AND (c.workspace_id = $2 OR c.workspace_id IS NULL)
          AND (d.workspace_id = $2
               OR (d.workspace_id IS NULL AND d.metadata->>'workspace_id' = $3))
        ORDER BY rank DESC
        LIMIT $4
        "#;

/// Search typed chunks by tsquery under an optional workspace filter.
/// Returns (chunk_id, document_id, rank) ordered by rank desc.
#[cfg(feature = "postgres")]
pub async fn search_chunks_fts(
    pool: &sqlx::PgPool,
    query: &str,
    workspace_id: Option<uuid::Uuid>,
    limit: i64,
) -> Result<Vec<(uuid::Uuid, uuid::Uuid, f32)>, crate::error::StorageError> {
    let limit = limit.clamp(1, 200);
    let Some(workspace_id) = workspace_id else {
        let rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, f32)>(UNSCOPED_SQL)
            .bind(query)
            .bind(limit)
            .fetch_all(pool)
            .await
            .map_err(|e| crate::error::StorageError::Database(format!("chunk FTS: {e}")))?;
        return Ok(rows);
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| crate::error::StorageError::Database(format!("chunk FTS begin: {e}")))?;
    // The required predicate is still priced as a few hundred hits on a
    // generic plan, so that plan looks up every document. Replan with the
    // bound term so the document join stops at the page.
    sqlx::query("SET LOCAL plan_cache_mode = force_custom_plan")
        .execute(&mut *tx)
        .await
        .map_err(|e| crate::error::StorageError::Database(format!("chunk FTS plan mode: {e}")))?;
    let rows = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, f32)>(SCOPED_SQL)
        .bind(query)
        .bind(workspace_id)
        .bind(workspace_id.to_string())
        .bind(limit)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| crate::error::StorageError::Database(format!("chunk FTS: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| crate::error::StorageError::Database(format!("chunk FTS commit: {e}")))?;
    Ok(rows)
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::{SCOPED_SQL, UNSCOPED_SQL};

    #[test]
    fn scoped_chunk_fts_requires_the_workspace() {
        assert!(!SCOPED_SQL.contains("$2::uuid IS NULL"));
        assert!(SCOPED_SQL.contains("c.workspace_id = $2 OR c.workspace_id IS NULL"));
        assert!(SCOPED_SQL.contains("d.workspace_id = $2"));
        assert!(SCOPED_SQL.contains("d.metadata->>'workspace_id' = $3"));
        assert!(!UNSCOPED_SQL.contains("IS NULL"));
        assert!(!UNSCOPED_SQL.contains("documents"));
    }
}
