//! SPEC-091 W3/W4 descriptor: legacy `eq_*_vectors` chunk rows → typed
//! `chunk_embeddings` (migration 108), **fleet-wide**.
//!
//! The job enumerates every remaining `public.eq_%_vectors` relation (shared +
//! per-workspace) and keyset-scans each in deterministic (sorted) order. The
//! cursor carries `(table, last_id)` so the leased runner resumes across tables
//! exactly as it does within one. Per table: Rust-side chunk-key filter
//! (`{doc}-chunk-{n}`), join `(document_id, chunk_index) -> chunks.id`, upsert
//! model + insert typed rows — all inside the runner's transaction. Idempotent
//! via `ON CONFLICT (model_id, chunk_id) DO NOTHING`. 42P01-safe (EC-35): a
//! dropped legacy table ⇒ that table is skipped / estimate 0 / clean pass.

use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha384};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::coverage::{list_vector_tables, list_vector_tables_ex};
use super::runner::{BackfillJob, BatchOutcome, VerifyReport};
use crate::error::StorageError;
use crate::kv_key_schema::kv_keys;

const DESCRIPTOR_DEF: &str = concat!(
    "w3-chunk-embedding-backfill/v2:",
    "source=legacy_vectors_fleet:keyset_per_table;filter=parse_doc_chunk;",
    "join=chunks(document_id,chunk_index);insert=unnest+on_conflict(model_id,chunk_id);",
    "verify=coverage+sampled_vector_equality_fleet"
);

/// Count chunk rows in one legacy table (42P01-safe → 0).
async fn count_table_chunks(pool: &PgPool, table: &str) -> Result<i64, StorageError> {
    match sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM public.{table} WHERE id ~ '{re}'",
        re = super::coverage::LEGACY_CHUNK_VECTOR_ID_RE
    ))
    .fetch_one(pool)
    .await
    {
        Ok(n) => Ok(n),
        Err(sqlx::Error::Database(db)) if db.code().as_deref() == Some("42P01") => Ok(0),
        Err(e) => Err(StorageError::Database(format!(
            "w3 fleet count({table}) failed: {e}"
        ))),
    }
}

pub struct ChunkEmbeddingBackfillJob {
    model_name: String,
}

impl ChunkEmbeddingBackfillJob {
    /// Fleet-wide constructor: the job discovers legacy vector tables at run
    /// time, so no per-table binding is needed (kept parameter for the boot
    /// call-site signature; intentionally ignored).
    pub fn new(_vectors_table: String, model_name: String) -> Self {
        Self { model_name }
    }
}

/// Parse `embedding::text` (`[0.1,0.2,...]`) back into `Vec<f32>`.
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

#[async_trait]
impl BackfillJob for ChunkEmbeddingBackfillJob {
    fn step_id(&self) -> &'static str {
        "w3-chunk-embedding-backfill"
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
        json!({ "table": Value::Null, "last_id": "" })
    }

    async fn estimate_total(&self, pool: &PgPool) -> Result<i64, StorageError> {
        let tables = list_vector_tables(pool).await?;
        let mut total = 0;
        for t in &tables {
            total += count_table_chunks(pool, t).await?;
        }
        Ok(total)
    }

    async fn run_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        cursor: &Value,
        limit: i64,
    ) -> Result<BatchOutcome, StorageError> {
        // Fleet table list is re-derived per batch so tables dropped mid-run are
        // skipped and newly-appearing ones are picked up on the next pass.
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

        // Resolve the active table: the cursor's table if still present, else
        // the first table after it in sorted order (or the first table when the
        // cursor has not started / points past the end).
        let active_idx = match cur_table.as_deref() {
            Some(t) => match tables.iter().position(|x| x == t) {
                Some(i) => i,
                // Cursor's table is gone (dropped) → resume at the next table.
                None => match tables.iter().position(|x| x.as_str() > t) {
                    Some(i) => i,
                    None => {
                        return Ok(BatchOutcome {
                            scanned: 0,
                            written: 0,
                            failed: 0,
                            next_cursor: None,
                        })
                    }
                },
            },
            None => 0,
        };
        // When we advance to a fresh table (cursor's table dropped) the keyset
        // must restart from the beginning of that table.
        let start_id = if cur_table.as_deref() == tables.get(active_idx).map(String::as_str) {
            last_id.clone()
        } else {
            String::new()
        };
        let table = &tables[active_idx];

        let rows = match sqlx::query_as::<_, (String, String)>(&format!(
            "SELECT id, embedding::text FROM public.{table} WHERE id > $1 ORDER BY id LIMIT $2"
        ))
        .bind(&start_id)
        .bind(limit)
        .fetch_all(&mut **tx)
        .await
        {
            Ok(rows) => rows,
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => {
                // Table dropped between list and scan → advance to next table.
                return Ok(BatchOutcome {
                    scanned: 0,
                    written: 0,
                    failed: 0,
                    next_cursor: Some(json!({ "table": table, "last_id": "" })),
                });
            }
            Err(e) => {
                return Err(StorageError::Database(format!(
                    "w3 backfill scan({table}) failed: {e}"
                )))
            }
        };

        if rows.is_empty() {
            // Current table exhausted → advance cursor to the next table (or
            // signal completion when this was the last).
            return if active_idx + 1 < tables.len() {
                Ok(BatchOutcome {
                    scanned: 0,
                    written: 0,
                    failed: 0,
                    next_cursor: Some(json!({ "table": tables[active_idx + 1], "last_id": "" })),
                })
            } else {
                Ok(BatchOutcome {
                    scanned: 0,
                    written: 0,
                    failed: 0,
                    next_cursor: None,
                })
            };
        }

        let scanned = rows.len() as i64;
        let next_id = rows.last().map(|(k, _)| k.clone()).unwrap_or_default();

        // Filter to chunk rows and parse keys.
        let mut parsed: Vec<(Uuid, i32, Vec<f32>)> = Vec::new();
        for (id, emb_text) in &rows {
            let Some((doc_str, index)) = kv_keys::parse_doc_chunk(id) else {
                continue; // non-chunk family — scanned, skipped
            };
            let Ok(doc_uuid) = Uuid::parse_str(doc_str) else {
                continue;
            };
            let index = i32::try_from(index).unwrap_or(i32::MAX);
            let Some(embedding) = parse_vector_text(emb_text) else {
                continue;
            };
            parsed.push((doc_uuid, index, embedding));
        }

        if parsed.is_empty() {
            return Ok(BatchOutcome {
                scanned,
                written: 0,
                failed: 0,
                next_cursor: Some(json!({ "table": table, "last_id": next_id })),
            });
        }

        // Resolve chunks.id + workspace_id by (document_id, chunk_index).
        let docs: Vec<Uuid> = parsed.iter().map(|(d, _, _)| *d).collect();
        let idxs: Vec<i32> = parsed.iter().map(|(_, i, _)| *i).collect();
        let spine: Vec<(Uuid, i32, Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT document_id, chunk_index, id, workspace_id FROM chunks \
             WHERE (document_id, chunk_index) IN (SELECT * FROM unnest($1::uuid[], $2::int[]))",
        )
        .bind(&docs)
        .bind(&idxs)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("w3 spine lookup failed: {e}")))?;
        let spine_map: std::collections::HashMap<(Uuid, i32), (Uuid, Option<Uuid>)> = spine
            .into_iter()
            .map(|(d, i, id, w)| ((d, i), (id, w)))
            .collect();

        let dimensions = parsed[0].2.len() as i32;
        if dimensions <= 0 {
            return Ok(BatchOutcome {
                scanned,
                written: 0,
                failed: 0,
                next_cursor: Some(json!({ "table": table, "last_id": next_id })),
            });
        }

        // Upsert model registry row for (name, dimensions).
        let model_id: Uuid = sqlx::query_scalar(
            "INSERT INTO embedding_models (name, dimensions) VALUES ($1, $2) \
             ON CONFLICT (name, dimensions) DO UPDATE SET name = EXCLUDED.name RETURNING id",
        )
        .bind(&self.model_name)
        .bind(dimensions)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("w3 model upsert failed: {e}")))?;

        let mut chunk_ids: Vec<Uuid> = Vec::new();
        let mut workspace_ids: Vec<Uuid> = Vec::new();
        let mut vectors: Vec<String> = Vec::new();
        let mut dims: Vec<i32> = Vec::new();
        // SPEC-396: missing `chunks` spine / NULL workspace is a visible miss,
        // not a silent skip with failed: 0. DROP 126 stays fail-closed on the
        // stricter uncovered predicate (legacy ∧ ¬typed coverage).
        let mut failed = 0i64;
        for (doc_uuid, index, embedding) in parsed {
            let Some((chunk_id, ws)) = spine_map.get(&(doc_uuid, index)) else {
                failed += 1;
                continue;
            };
            let Some(ws_uuid) = ws else {
                failed += 1;
                continue;
            };
            chunk_ids.push(*chunk_id);
            workspace_ids.push(*ws_uuid);
            vectors.push(format!(
                "[{}]",
                embedding
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            dims.push(embedding.len() as i32);
        }

        let written = if chunk_ids.is_empty() {
            0
        } else {
            sqlx::query(
                "INSERT INTO chunk_embeddings (model_id, chunk_id, workspace_id, embedding, dimensions) \
                 SELECT $1, c, w, v::halfvec, d \
                 FROM unnest($2::uuid[], $3::uuid[], $4::text[], $5::int[]) AS t(c, w, v, d) \
                 ON CONFLICT (model_id, chunk_id) DO NOTHING",
            )
            .bind(model_id)
            .bind(&chunk_ids)
            .bind(&workspace_ids)
            .bind(&vectors)
            .bind(&dims)
            .execute(&mut **tx)
            .await
            .map_err(|e| StorageError::Database(format!("w3 typed insert failed: {e}")))?
            .rows_affected() as i64
        };

        Ok(BatchOutcome {
            scanned,
            written,
            failed,
            next_cursor: Some(json!({ "table": table, "last_id": next_id })),
        })
    }

    async fn verify(&self, pool: &PgPool) -> Result<VerifyReport, StorageError> {
        // Fleet-wide verify: aggregate coverage + sampled equality across every
        // remaining legacy chunk-vector relation. A dropped fleet verifies clean
        // (expected=0), matching the 42P01-safe single-table semantics.
        let tables = list_vector_tables(pool).await?;
        let mut agg = VerifyReport {
            metric: "w3-chunk-embedding-fleet".to_string(),
            expected: 0,
            actual: 0,
            sampled: 0,
            mismatches: 0,
        };
        for table in &tables {
            let r = super::verify::verify_chunk_embedding_backfill(pool, table, &self.model_name)
                .await?;
            agg.expected += r.expected;
            agg.actual += r.actual;
            agg.sampled += r.sampled;
            agg.mismatches += r.mismatches;
        }
        Ok(agg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_spec091_w3_step_identity_stable() {
        let job = ChunkEmbeddingBackfillJob::new(
            "public.eq_eq_default_vectors".into(),
            "text-embedding-3-small".into(),
        );
        assert_eq!(job.step_id(), "w3-chunk-embedding-backfill");
        assert_eq!(job.schema_generation(), 1);
        assert_eq!(job.step_sha384().len(), 96);
        assert_eq!(
            job.initial_cursor(),
            json!({ "table": Value::Null, "last_id": "" })
        );
    }

    #[test]
    fn contract_spec091_w3_parse_vector_text() {
        assert_eq!(
            parse_vector_text("[0.1,0.2,0.3]"),
            Some(vec![0.1, 0.2, 0.3])
        );
        assert_eq!(parse_vector_text("[]"), Some(vec![]));
        assert!(parse_vector_text("not-a-vector").is_none());
    }
}
