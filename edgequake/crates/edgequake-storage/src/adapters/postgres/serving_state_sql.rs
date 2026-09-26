//! Single writer for `public.chunk_serving_state` upserts (SPEC-091).
//!
//! WHY: Four near-identical INSERT…ON CONFLICT statements drifted — only some
//! skipped already-ready rows. Every production write of chunk serving state
//! must go through this module.

use uuid::Uuid;

use crate::error::StorageError;

/// Shared ON CONFLICT clause: skip no-op updates when state is unchanged.
///
/// Expanded as a string literal so callers can embed it with `concat!`.
macro_rules! serving_state_conflict_tail {
    () => {
        "ON CONFLICT (chunk_id) DO UPDATE \
         SET state = EXCLUDED.state, updated_at = now() \
         WHERE public.chunk_serving_state.state IS DISTINCT FROM EXCLUDED.state"
    };
}

const UPSERT_FOR_DOCUMENT_SQL: &str = concat!(
    "INSERT INTO public.chunk_serving_state (chunk_id, state) ",
    "SELECT id, $2 FROM public.chunks WHERE document_id = $1 ",
    serving_state_conflict_tail!()
);

const UPSERT_FOR_IDS_SQL: &str = concat!(
    "INSERT INTO public.chunk_serving_state (chunk_id, state) ",
    "SELECT id, $2 FROM unnest($1::uuid[]) AS id ",
    serving_state_conflict_tail!()
);

/// Upsert serving state for every chunk of one document.
///
/// Returns `rows_affected` (0 when every chunk was already in `state`).
pub(crate) async fn upsert_for_document<'e, E>(
    executor: E,
    document_id: Uuid,
    state: &str,
) -> Result<u64, StorageError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let result = sqlx::query(UPSERT_FOR_DOCUMENT_SQL)
        .bind(document_id)
        .bind(state)
        .execute(executor)
        .await
        .map_err(|e| StorageError::Database(format!("chunk_serving_state upsert failed: {e}")))?;
    Ok(result.rows_affected())
}

/// Upsert serving state for an explicit set of chunk ids (backfill path).
pub(crate) async fn upsert_for_ids<'e, E>(
    executor: E,
    chunk_ids: &[Uuid],
    state: &str,
) -> Result<u64, StorageError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    if chunk_ids.is_empty() {
        return Ok(0);
    }
    let result = sqlx::query(UPSERT_FOR_IDS_SQL)
        .bind(chunk_ids)
        .bind(state)
        .execute(executor)
        .await
        .map_err(|e| StorageError::Database(format!("chunk_serving_state upsert failed: {e}")))?;
    Ok(result.rows_affected())
}

/// Bounded settle+open SQL used by reconcile. The conflict tail is shared so
/// the guard cannot drift from the per-document / per-id writers.
pub(crate) const OPEN_SETTLED_FENCES_BOUNDED_SQL: &str = concat!(
    r#"
WITH candidates AS (
    SELECT DISTINCT e.object_id AS document_id
    FROM public.projection_events e
    WHERE e.object_kind = 'document_batch'
),
settled_docs AS (
    SELECT c.document_id
    FROM candidates c
    WHERE NOT EXISTS (
        SELECT 1
        FROM public.projection_events e
        JOIN public.projection_deliveries d ON d.event_id = e.event_id
        WHERE e.object_id = c.document_id
          AND e.object_kind = 'document_batch'
          AND d.state IN ('pending', 'leased', 'retry', 'quarantined')
    )
),
needs_open AS (
    SELECT sd.document_id
    FROM settled_docs sd
    WHERE EXISTS (
        SELECT 1
        FROM public.chunks c
        LEFT JOIN public.chunk_serving_state css ON css.chunk_id = c.id
        WHERE c.document_id = sd.document_id
          AND COALESCE(css.state, 'declared') IS DISTINCT FROM $1
    )
    ORDER BY sd.document_id
    LIMIT $2
)
INSERT INTO public.chunk_serving_state (chunk_id, state)
SELECT c.id, $1
FROM public.chunks c
JOIN needs_open n ON n.document_id = c.document_id
"#,
    serving_state_conflict_tail!()
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn conflict_tail_has_distinct_guard() {
        assert!(UPSERT_FOR_DOCUMENT_SQL.contains("IS DISTINCT FROM"));
        assert!(UPSERT_FOR_IDS_SQL.contains("IS DISTINCT FROM"));
        assert!(OPEN_SETTLED_FENCES_BOUNDED_SQL.contains("IS DISTINCT FROM"));
        assert!(OPEN_SETTLED_FENCES_BOUNDED_SQL.contains("LIMIT $2"));
        assert!(serving_state_conflict_tail!().contains("IS DISTINCT FROM"));
    }

    #[test]
    fn only_this_module_writes_chunk_serving_state_in_src() {
        let crates_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates/")
            .to_path_buf();
        let allowed = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/adapters/postgres/serving_state_sql.rs");
        let mut offenders = Vec::new();
        for crate_dir in std::fs::read_dir(&crates_root).expect("read crates") {
            let crate_dir = crate_dir.expect("crate entry").path();
            let src = crate_dir.join("src");
            if !src.is_dir() {
                continue;
            }
            walk_rs(&src, &mut |path, contents| {
                if path == allowed {
                    return;
                }
                if writes_chunk_serving_state(contents) {
                    offenders.push(path.display().to_string());
                }
            });
        }
        assert!(
            offenders.is_empty(),
            "chunk_serving_state INSERT/UPDATE must live only in serving_state_sql.rs; found {offenders:?}"
        );
    }

    fn writes_chunk_serving_state(src: &str) -> bool {
        let lower = src.to_ascii_lowercase();
        // Match INSERT/UPDATE of chunk_serving_state with or without public. prefix.
        for line in lower.lines() {
            let trimmed = line.trim_start();
            let insert = trimmed.contains("insert into")
                && (trimmed.contains("public.chunk_serving_state")
                    || trimmed.contains(" chunk_serving_state")
                    || trimmed.contains("into chunk_serving_state"));
            let update = trimmed.starts_with("update ")
                && (trimmed.contains("public.chunk_serving_state")
                    || trimmed.contains(" chunk_serving_state"));
            if insert || update {
                return true;
            }
        }
        false
    }

    fn walk_rs(dir: &Path, visit: &mut dyn FnMut(&Path, &str)) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_rs(&path, visit);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    visit(&path, &contents);
                }
            }
        }
    }
}
