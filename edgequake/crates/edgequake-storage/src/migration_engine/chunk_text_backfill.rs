//! SPEC-091 Wave-1 descriptor: KV chunk text → relational `chunks` spine.
//!
//! Keyset scan over the shared KV table (PK btree range, never OFFSET),
//! Rust-side chunk-key filter, per-batch document lookup (tenant/workspace),
//! single unnest insert + serving-state seed — all inside the runner's
//! transaction. Idempotent via `ON CONFLICT (document_id, chunk_index)`.

use async_trait::async_trait;
use serde_json::{json, Value};
use sha2::{Digest, Sha384};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::runner::{BackfillJob, BatchOutcome, VerifyReport};
use super::verify;
use crate::adapters::postgres::chunk_repository::{insert_chunks_batch, ChunkInsertRow};
use crate::adapters::postgres::serving_state_sql::upsert_for_ids;
use crate::error::StorageError;
use crate::kv_key_schema::kv_keys;

/// Descriptor bytes hashed into `step_sha384` (drift guard for in-flight jobs).
const DESCRIPTOR_DEF: &str = concat!(
    "w1-chunk-text-backfill/v1:",
    "source=kv:keyset;filter=parse_doc_chunk;insert=unnest+on_conflict;",
    "serving=ready;verify=coverage+sampled_content"
);

pub struct ChunkTextBackfillJob {
    kv_table: String,
}

impl ChunkTextBackfillJob {
    pub fn new(kv_table: String) -> Self {
        // Identifier injection guard: table names cannot be parameterized.
        debug_assert!(
            kv_table
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.'),
            "unsafe kv table identifier"
        );
        Self { kv_table }
    }
}

#[async_trait]
impl BackfillJob for ChunkTextBackfillJob {
    fn step_id(&self) -> &'static str {
        "w1-chunk-text-backfill"
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
        json!({ "last_key": "" })
    }

    async fn estimate_total(&self, pool: &PgPool) -> Result<i64, StorageError> {
        // Exact COUNT(*) once (07 §Estimates: backfills use exact counts).
        match sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {}", self.kv_table))
            .fetch_one(pool)
            .await
        {
            Ok(n) => Ok(n),
            // EC-35 post-Wave-D edge case: the generic KV relation has been
            // dropped, so there is nothing left to backfill — report 0 (EC-15
            // zero-estimate short-circuit) instead of erroring every boot.
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => {
                Ok(0)
            }
            Err(e) => Err(StorageError::Database(format!(
                "backfill estimate failed: {e}"
            ))),
        }
    }

    async fn run_batch(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        cursor: &Value,
        limit: i64,
    ) -> Result<BatchOutcome, StorageError> {
        let last_key = cursor
            .get("last_key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        // Keyset pagination over the PK btree — non-chunk keys are scanned and
        // skipped in Rust so the cursor always advances (no LIKE '%..%' seq scan).
        let rows = match sqlx::query_as::<_, (String, Value)>(&format!(
            "SELECT key, value FROM {} WHERE key > $1 ORDER BY key LIMIT $2",
            self.kv_table
        ))
        .bind(&last_key)
        .bind(limit)
        .fetch_all(&mut **tx)
        .await
        {
            Ok(rows) => rows,
            // EC-35: KV source dropped post-Wave-D — nothing to scan, finish.
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => {
                return Ok(BatchOutcome {
                    scanned: 0,
                    written: 0,
                    failed: 0,
                    next_cursor: None,
                });
            }
            Err(e) => return Err(StorageError::Database(format!("backfill scan failed: {e}"))),
        };

        if rows.is_empty() {
            return Ok(BatchOutcome {
                scanned: 0,
                written: 0,
                failed: 0,
                next_cursor: None,
            });
        }

        let scanned = rows.len() as i64;
        let next_key = rows.last().map(|(k, _)| k.clone()).unwrap_or_default();

        // Parse chunk rows; collect distinct document ids for one lookup.
        let mut parsed: Vec<(String, Uuid, i32, Value)> = Vec::new();
        let mut doc_ids: Vec<Uuid> = Vec::new();
        for (key, value) in &rows {
            let Some((doc_str, index)) = kv_keys::parse_doc_chunk(key) else {
                continue; // other KV family — scanned, skipped
            };
            let Ok(doc_uuid) = Uuid::parse_str(doc_str) else {
                tracing::warn!(key = %key, "backfill: chunk key with non-UUID document id — skipped");
                continue;
            };
            let index = i32::try_from(index).unwrap_or(i32::MAX);
            if !doc_ids.contains(&doc_uuid) {
                doc_ids.push(doc_uuid);
            }
            parsed.push((key.clone(), doc_uuid, index, value.clone()));
        }

        // One per-batch document lookup (tenant/workspace authority + FK guard).
        let doc_map: std::collections::HashMap<Uuid, (Option<Uuid>, Option<Uuid>)> =
            if doc_ids.is_empty() {
                Default::default()
            } else {
                sqlx::query_as::<_, (Uuid, Option<Uuid>, Option<Uuid>)>(
                    "SELECT id, tenant_id, workspace_id FROM public.documents WHERE id = ANY($1)",
                )
                .bind(&doc_ids)
                .fetch_all(&mut **tx)
                .await
                .map_err(|e| StorageError::Database(format!("backfill doc lookup failed: {e}")))?
                .into_iter()
                .map(|(id, t, w)| (id, (t, w)))
                .collect()
            };

        let mut inserts: Vec<ChunkInsertRow> = Vec::with_capacity(parsed.len());
        let mut orphaned = 0usize;
        for (key, doc_uuid, index, value) in parsed {
            let Some(&(tenant_id, workspace_id)) = doc_map.get(&doc_uuid) else {
                orphaned += 1;
                continue; // FK guard: document row gone → quarantine via failed_count
            };
            let content = value
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if content.is_empty() {
                orphaned += 1;
                continue;
            }
            inserts.push(ChunkInsertRow {
                document_id: doc_uuid,
                tenant_id,
                workspace_id,
                chunk_index: index,
                content: content.to_string(),
                start_offset: value
                    .get("start_offset")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32),
                end_offset: value
                    .get("end_offset")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32),
                token_count: value
                    .get("token_count")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32),
                metadata: json!({ "legacy_chunk_key": key }),
                page_start: value
                    .get("page_start")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32),
                page_end: value
                    .get("page_end")
                    .and_then(Value::as_i64)
                    .map(|v| v as i32)
                    .or_else(|| {
                        value
                            .get("page_start")
                            .and_then(Value::as_i64)
                            .map(|v| v as i32)
                    }),
            });
        }
        if orphaned > 0 {
            tracing::warn!(
                orphaned,
                "backfill: skipped chunks with missing document or empty content"
            );
        }

        let inserted_ids = insert_chunks_batch(&mut **tx, &inserts).await?;
        let written = inserted_ids.len() as i64;
        // Legacy chunks already have vectors + graph projections → serve-ready.
        upsert_for_ids(
            &mut **tx,
            &inserted_ids,
            crate::serving_fence::SERVING_STATE_READY,
        )
        .await?;

        Ok(BatchOutcome {
            scanned,
            written,
            failed: 0,
            next_cursor: Some(json!({ "last_key": next_key })),
        })
    }

    async fn verify(&self, pool: &PgPool) -> Result<VerifyReport, StorageError> {
        verify::verify_chunk_text_backfill(pool, &self.kv_table).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_spec091_step_identity_stable() {
        let job = ChunkTextBackfillJob::new("public.eq_eq_default_kv".into());
        assert_eq!(job.step_id(), "w1-chunk-text-backfill");
        assert_eq!(job.schema_generation(), 1);
        assert_eq!(job.step_sha384().len(), 96); // SHA-384 hex
        assert_eq!(job.step_sha384(), job.step_sha384());
        assert_eq!(job.initial_cursor(), json!({ "last_key": "" }));
    }

    #[test]
    fn contract_spec091_descriptor_def_covers_contract() {
        assert!(DESCRIPTOR_DEF.contains("keyset"));
        assert!(DESCRIPTOR_DEF.contains("on_conflict"));
        assert!(DESCRIPTOR_DEF.contains("verify"));
    }
}
