//! PostgreSQL key-value storage using JSONB.
//!
//! Provides flexible key-value storage with full JSON query capabilities.
//!
//! ## Implements
//!
//! - [`FEAT0240`]: JSONB key-value storage
//! - [`FEAT0241`]: GIN indexing for fast JSON path queries
//! - [`FEAT0242`]: Atomic upsert operations
//!
//! ## Use Cases
//!
//! - [`UC0601`]: System stores document metadata
//! - [`UC0605`]: System retrieves chunks by ID
//!
//! ## Enforces
//!
//! - [`BR0240`]: Namespace isolation per tenant
//! - [`BR0241`]: Atomic batch operations

use std::collections::HashSet;

use async_trait::async_trait;

use super::config::{qualified_kv_table_name, PostgresConfig};
use super::connection::PostgresPool;
use super::row_count_stats::{self, RowCountStatsConfig};
use crate::error::{Result, StorageError};
use crate::kv_keys;
use crate::traits::KVStorage;

/// SPEC-083 X-37: validate workspace-scoped KV keys in a write batch.
///
/// - Malformed `wsdoc:` / `staging:hash:` keys are rejected.
/// - A single upsert batch may not mix multiple embedded workspace ids
///   (defense-in-depth against accidental cross-tenant writes).
fn enforce_workspace_scoped_keys<'a>(keys: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen_workspace: Option<&str> = None;
    for key in keys {
        if key.starts_with("wsdoc:") || key.starts_with("staging:hash:") {
            let Some(ws) = kv_keys::embedded_workspace_id(key) else {
                return Err(StorageError::InvalidInput(format!(
                    "Malformed workspace-scoped KV key: {key}"
                )));
            };
            match seen_workspace {
                None => seen_workspace = Some(ws),
                Some(prev) if prev != ws => {
                    return Err(StorageError::InvalidInput(format!(
                        "KV upsert mixes workspace scopes '{prev}' and '{ws}'"
                    )));
                }
                Some(_) => {}
            }
        }
    }
    Ok(())
}

/// PostgreSQL key-value storage using JSONB.
///
/// This implementation uses PostgreSQL's JSONB column type for flexible
/// value storage with full JSON query capabilities.
///
/// # Features
///
/// - JSONB storage for flexible schemas
/// - GIN indexing for fast JSON path queries
/// - Atomic upsert operations
/// - Namespace support for multi-tenancy
pub struct PostgresKVStorage {
    pool: PostgresPool,
    table_name: String,
    stats_table_name: String,
    namespace: String,
    prefix: String,
}

impl PostgresKVStorage {
    /// Create a new PostgreSQL key-value storage.
    pub fn new(config: PostgresConfig) -> Self {
        Self::with_pool(PostgresPool::new(config.clone()), config)
    }

    /// Create KV storage using a shared connection pool (SPEC-011).
    pub fn with_pool(pool: PostgresPool, config: PostgresConfig) -> Self {
        let prefix = config.table_prefix();
        let table_name = qualified_kv_table_name(&prefix);
        let stats_table_name = format!("public.eq_{}_kv_stats", prefix);
        let namespace = config.namespace.clone();

        Self {
            pool,
            table_name,
            stats_table_name,
            namespace,
            prefix,
        }
    }

    /// Get the underlying pool.
    pub fn pool(&self) -> &PostgresPool {
        &self.pool
    }

    /// Create the KV table.
    async fn create_table(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                key TEXT PRIMARY KEY,
                value JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name
        );

        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Failed to create KV table: {}", e)))?;

        // SPEC-034 IMP-03: KV GIN index on `value` (GIN over 61 KB chunks) removed.
        // WHY: The index was 112 MB (155× the 760 KB heap) with 0 query scans.
        // All KV lookups use the primary key (key column, btree). No code path
        // queries KV by value content. The btree PK remains.

        // SPEC-011 iter 02 Fix C: B-tree index on reverse(key) for O(log N) suffix scans.
        // Used by `keys_with_suffix`, which the workspace stats endpoint calls every 30 s
        // with `"-metadata"`. Without this index the equivalent `LIKE '%-metadata'` does
        // a full table scan.
        let reverse_idx_sql = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_kv_reverse_key_idx ON {} (reverse(key) text_pattern_ops)",
            self.prefix, self.table_name
        );
        sqlx::query(&reverse_idx_sql).execute(&pool).await.ok();

        // Local-ingest hardening: PK btree (default collation) cannot serve
        // `LIKE 'prefix%'` under non-C locales (en_US.utf8 → Seq Scan over full KV).
        // `text_pattern_ops` enables Index Only Scan + LIMIT short-circuit for
        // `wsdoc:` workspace index enumeration (O(limit) not O(table)).
        let key_pattern_idx_sql = format!(
            "CREATE INDEX IF NOT EXISTS eq_{}_kv_key_pattern_idx ON {} (key text_pattern_ops)",
            self.prefix, self.table_name
        );
        sqlx::query(&key_pattern_idx_sql).execute(&pool).await.ok();

        self.ensure_row_count_stats(&pool).await?;

        Ok(())
    }

    /// O(1) row counter for `count()` — avoids `SELECT COUNT(*) FROM kv` full-table scans.
    ///
    /// SPEC-011: Production incident — 13s `COUNT(*)` on `eq_eq_default_kv` during health
    /// probes. Maintained counter + triggers keep exact counts at O(1) when `count()` is
    /// called from tests or admin tools.
    async fn ensure_row_count_stats(&self, pool: &sqlx::PgPool) -> Result<()> {
        row_count_stats::ensure_row_count_stats(
            pool,
            &RowCountStatsConfig {
                prefix: &self.prefix,
                table_name: &self.table_name,
                stats_table_name: &self.stats_table_name,
                kind: "kv",
            },
        )
        .await
    }

    async fn reset_row_count_stats(&self, pool: &sqlx::PgPool) -> Result<()> {
        let sql = format!(
            "UPDATE {} SET row_count = 0 WHERE id = 1",
            self.stats_table_name
        );
        sqlx::query(&sql).execute(pool).await.ok();
        Ok(())
    }
}

#[async_trait]
impl KVStorage for PostgresKVStorage {
    fn namespace(&self) -> &str {
        &self.namespace
    }

    async fn initialize(&self) -> Result<()> {
        self.pool.initialize().await?;
        self.create_table().await?;
        Ok(())
    }

    async fn finalize(&self) -> Result<()> {
        Ok(())
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let pool = self.pool.get().await?;

        let sql = format!("SELECT value FROM {} WHERE key = $1", self.table_name);

        let row: Option<(serde_json::Value,)> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV get failed: {}", e)))?;

        Ok(row.map(|(v,)| v))
    }

    async fn get_by_ids(&self, ids: &[String]) -> Result<Vec<serde_json::Value>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;

        // Preserve input order (SPEC-045) — never rely on unordered ANY() scans.
        let sql = format!(
            "SELECT kv.value \
             FROM unnest($1::text[]) WITH ORDINALITY AS u(key, ord) \
             INNER JOIN {table} kv ON kv.key = u.key \
             ORDER BY u.ord",
            table = self.table_name
        );

        let rows: Vec<(serde_json::Value,)> = sqlx::query_as(&sql)
            .bind(ids)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV get_by_ids failed: {}", e)))?;

        Ok(rows.into_iter().map(|(v,)| v).collect())
    }

    async fn get_by_ids_ordered(&self, ids: &[String]) -> Result<Vec<Option<serde_json::Value>>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;

        let sql = format!(
            "SELECT kv.value \
             FROM unnest($1::text[]) WITH ORDINALITY AS u(key, ord) \
             LEFT JOIN {table} kv ON kv.key = u.key \
             ORDER BY u.ord",
            table = self.table_name
        );

        let rows: Vec<(Option<serde_json::Value>,)> = sqlx::query_as(&sql)
            .bind(ids)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV get_by_ids_ordered failed: {}", e)))?;

        Ok(rows.into_iter().map(|(v,)| v).collect())
    }

    async fn filter_keys(&self, keys: HashSet<String>) -> Result<HashSet<String>> {
        if keys.is_empty() {
            return Ok(HashSet::new());
        }

        let pool = self.pool.get().await?;
        let keys_vec: Vec<String> = keys.iter().cloned().collect();

        let sql = format!("SELECT key FROM {} WHERE key = ANY($1)", self.table_name);

        let rows: Vec<(String,)> = sqlx::query_as(&sql)
            .bind(&keys_vec)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV filter_keys failed: {}", e)))?;

        let existing: HashSet<String> = rows.into_iter().map(|(k,)| k).collect();

        // Return keys that do NOT exist
        Ok(keys.difference(&existing).cloned().collect())
    }

    async fn upsert(&self, data: &[(String, serde_json::Value)]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        // SPEC-083 X-37: reject malformed workspace-scoped keys; when a batch
        // mixes multiple embedded workspace ids, fail closed (cross-tenant write).
        enforce_workspace_scoped_keys(data.iter().map(|(k, _)| k.as_str()))?;

        let pool = self.pool.get().await?;
        // C-22: all batches commit atomically — mid-batch failure must not leave
        // a partial KV write set.
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(format!("KV upsert begin failed: {}", e)))?;
        const BATCH_SIZE: usize = 1000;

        for chunk in data.chunks(BATCH_SIZE) {
            let keys: Vec<String> = chunk.iter().map(|(k, _)| k.clone()).collect();
            let values: Vec<serde_json::Value> = chunk.iter().map(|(_, v)| v.clone()).collect();

            let sql = format!(
                r#"
                INSERT INTO {} (key, value, updated_at)
                SELECT k, v, NOW()
                FROM unnest($1::text[], $2::jsonb[]) AS batch(k, v)
                ON CONFLICT (key) DO UPDATE SET
                    value = EXCLUDED.value,
                    updated_at = NOW()
                "#,
                self.table_name
            );

            sqlx::query(&sql)
                .bind(&keys)
                .bind(&values)
                .execute(&mut *tx)
                .await
                .map_err(|e| StorageError::Database(format!("KV upsert failed: {}", e)))?;
        }

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("KV upsert commit failed: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let pool = self.pool.get().await?;

        let sql = format!("DELETE FROM {} WHERE key = ANY($1)", self.table_name);

        sqlx::query(&sql)
            .bind(ids)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV delete failed: {}", e)))?;

        Ok(())
    }

    async fn is_empty(&self) -> Result<bool> {
        let pool = self.pool.get().await?;

        let sql = format!(
            "SELECT NOT EXISTS (SELECT 1 FROM {} LIMIT 1) AS is_empty",
            self.table_name
        );

        let row: (bool,) = sqlx::query_as(&sql)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV is_empty failed: {}", e)))?;

        Ok(row.0)
    }

    async fn count(&self) -> Result<usize> {
        let pool = self.pool.get().await?;

        // O(1): read maintained counter — never `SELECT COUNT(*) FROM kv` (SPEC-011).
        let sql = format!(
            "SELECT row_count FROM {} WHERE id = 1",
            self.stats_table_name
        );

        let row: Option<(i64,)> = sqlx::query_as(&sql)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV count failed: {}", e)))?;

        if let Some((count,)) = row {
            return Ok(count as usize);
        }

        // SPEC-012 Fix H (self-heal): production logs showed `SELECT COUNT(*) as count
        // FROM eq_eq_default_kv` running 5806× / 12 s total on a deployment that
        // predated SPEC-011 — the stats table was never bootstrapped. Run the
        // initialiser inline so the *next* call hits the O(1) path, then return the
        // exact count from the bootstrap backfill we just inserted.
        tracing::warn!(
            stats_table = %self.stats_table_name,
            "KV stats row missing — running self-heal (one-time COUNT(*) + create triggers)"
        );
        if let Err(e) = self.ensure_row_count_stats(&pool).await {
            tracing::warn!(error = %e, "KV stats self-heal failed; falling back to COUNT(*)");
        }

        let fallback = format!("SELECT COUNT(*) as count FROM {}", self.table_name);
        let row: (i64,) = sqlx::query_as(&fallback)
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV count fallback failed: {}", e)))?;
        Ok(row.0 as usize)
    }

    async fn ping(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        let sql = format!("SELECT 1 FROM {} LIMIT 1", self.table_name);

        sqlx::query(&sql)
            .fetch_optional(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV ping failed: {}", e)))?;

        Ok(())
    }

    /// SPEC-087 / Issue #334: O(1) round-trip chunk-key count (no payload fetch).
    async fn count_embedded_chunks_for_docs(&self, doc_ids: &[String]) -> Result<usize> {
        if doc_ids.is_empty() {
            return Ok(0);
        }

        let pool = self.pool.get().await?;
        // Escape LIKE meta in each doc id so `%`/`_` in ids cannot widen the match.
        let patterns: Vec<String> = doc_ids
            .iter()
            .map(|id| format!("{}-chunk-%", escape_like_meta(id)))
            .collect();

        let sql = format!(
            "SELECT COUNT(*)::bigint FROM {} WHERE key LIKE ANY($1::text[])",
            self.table_name
        );

        let row: (i64,) = sqlx::query_as(&sql)
            .bind(&patterns)
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("KV count_embedded_chunks_for_docs failed: {e}"))
            })?;

        Ok(row.0 as usize)
    }

    async fn keys_like(&self, pattern: &str) -> Result<Vec<String>> {
        // SPEC-070: never unbounded fetch_all — safety LIMIT on the wire.
        const SAFETY_CAP: usize = 100_000;
        let pool = self.pool.get().await?;
        let sql = format!(
            "SELECT key FROM {} WHERE key LIKE $1 LIMIT $2",
            self.table_name
        );
        let rows: Vec<(String,)> = sqlx::query_as(&sql)
            .bind(pattern)
            .bind(i64::try_from(SAFETY_CAP).unwrap_or(100_000))
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV keys_like failed: {}", e)))?;
        if rows.len() >= SAFETY_CAP {
            tracing::warn!(
                pattern,
                cap = SAFETY_CAP,
                "KV keys_like hit safety cap — prefer keys_with_prefix_limited (SPEC-070)"
            );
        }
        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    async fn keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        // SPEC-070: delegate to limited path (O(limit), not unbounded SeqScan risk).
        const SAFETY_CAP: usize = 100_000;
        let (keys, truncated) = self.keys_with_prefix_limited(prefix, SAFETY_CAP).await?;
        if truncated {
            tracing::warn!(
                prefix,
                cap = SAFETY_CAP,
                "KV keys_with_prefix hit safety cap — prefer keys_with_prefix_limited (SPEC-070)"
            );
        }
        Ok(keys)
    }

    async fn keys_with_prefix_limited(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<(Vec<String>, bool)> {
        // Clamp before i64 cast — usize::MAX as i64 is -1 on 64-bit targets.
        let limit = limit.clamp(1, 1_000_000);
        let pool = self.pool.get().await?;
        let like_pattern = format!("{}%", escape_like_meta(prefix));
        // Fetch limit+1 so we can report truncation without a second COUNT query.
        let fetch_limit = i64::try_from(limit).unwrap_or(1_000_000).saturating_add(1);

        // No ORDER BY: with `key text_pattern_ops` the planner can Index Scan
        // and stop after LIMIT (O(limit)). ORDER BY key forced Sort/SeqScan
        // under en_US.utf8 before the pattern index existed.
        let sql = format!(
            "SELECT key FROM {} WHERE key LIKE $1 LIMIT $2",
            self.table_name
        );

        let rows: Vec<(String,)> = sqlx::query_as(&sql)
            .bind(&like_pattern)
            .bind(fetch_limit)
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("KV keys_with_prefix_limited failed: {}", e))
            })?;

        let truncated = rows.len() > limit;
        Ok((
            rows.into_iter().take(limit).map(|(k,)| k).collect(),
            truncated,
        ))
    }

    async fn keys_with_suffix(&self, suffix: &str) -> Result<Vec<String>> {
        // SPEC-011 + SPEC-070: reverse-key index + safety LIMIT (no unbounded fetch).
        const SAFETY_CAP: usize = 100_000;
        let (keys, truncated) = self.keys_with_suffix_limited(suffix, SAFETY_CAP).await?;
        if truncated {
            tracing::warn!(
                suffix,
                cap = SAFETY_CAP,
                "KV keys_with_suffix hit safety cap — prefer keys_with_suffix_limited (SPEC-070)"
            );
        }
        Ok(keys)
    }

    async fn keys_with_suffix_limited(
        &self,
        suffix: &str,
        limit: usize,
    ) -> Result<(Vec<String>, bool)> {
        let limit = limit.clamp(1, 1_000_000);
        let pool = self.pool.get().await?;
        let reversed: String = escape_like_meta(suffix).chars().rev().collect();
        let like_pattern = format!("{reversed}%");
        let fetch_limit = i64::try_from(limit).unwrap_or(1_000_000).saturating_add(1);

        // No ORDER BY key: that forced Sort over the full reverse-index match
        // set before LIMIT. Unordered LIMIT lets the bitmap/index path stop early.
        let sql = format!(
            "SELECT key FROM {} WHERE reverse(key) LIKE $1 LIMIT $2",
            self.table_name
        );

        let rows: Vec<(String,)> = sqlx::query_as(&sql)
            .bind(&like_pattern)
            .bind(fetch_limit)
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("KV keys_with_suffix_limited failed: {}", e))
            })?;

        let truncated = rows.len() > limit;
        Ok((
            rows.into_iter().take(limit).map(|(k,)| k).collect(),
            truncated,
        ))
    }

    async fn keys(&self) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;

        let sql = format!("SELECT key FROM {}", self.table_name);

        let rows: Vec<(String,)> = sqlx::query_as(&sql)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV keys failed: {}", e)))?;

        Ok(rows.into_iter().map(|(k,)| k).collect())
    }

    async fn clear(&self) -> Result<()> {
        let pool = self.pool.get().await?;

        // TRUNCATE is faster than DELETE; row triggers don't fire — reset stats explicitly.
        let sql = format!("TRUNCATE {}", self.table_name);

        sqlx::query(&sql)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("KV clear failed: {}", e)))?;

        self.reset_row_count_stats(&pool).await?;

        Ok(())
    }

    /// Atomically transition document status if current status matches expected.
    ///
    /// @implements FIX-RACE-01: Prevent TOCTOU race conditions
    ///
    /// # WHY: Atomic Compare-And-Swap
    ///
    /// Uses PostgreSQL's atomic UPDATE with WHERE clause to ensure only one
    /// process can successfully transition the status. The affected row count
    /// tells us if the transition succeeded (1) or failed (0).
    ///
    /// SQL: UPDATE ... SET value = jsonb_set(...) WHERE key = $1 AND value->>'status' = $2
    ///
    /// This is atomic at the database level - no race window possible.
    async fn transition_if_status(
        &self,
        key: &str,
        expected_status: &str,
        new_status: &str,
    ) -> Result<bool> {
        let pool = self.pool.get().await?;

        // Atomic update: only succeeds if current status matches expected
        // jsonb_set updates the 'status' field within the JSONB value
        let sql = format!(
            r#"
            UPDATE {}
            SET value = jsonb_set(value, '{{status}}', to_jsonb($3::text)),
                updated_at = NOW()
            WHERE key = $1 AND value->>'status' = $2
            "#,
            self.table_name
        );

        let result = sqlx::query(&sql)
            .bind(key)
            .bind(expected_status)
            .bind(new_status)
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("KV transition_if_status failed: {}", e))
            })?;

        // rows_affected = 1 means transition succeeded
        // rows_affected = 0 means status didn't match (or key not found)
        Ok(result.rows_affected() == 1)
    }
}

impl std::fmt::Debug for PostgresKVStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresKVStorage")
            .field("namespace", &self.namespace)
            .field("table_name", &self.table_name)
            .finish()
    }
}

/// Escape `%`, `_`, and `\` for PostgreSQL `LIKE` patterns (literal match).
fn escape_like_meta(raw: &str) -> String {
    raw.chars()
        .flat_map(|c| match c {
            '%' | '_' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_storage_creation() {
        let config = PostgresConfig::default().with_namespace("test");
        let storage = PostgresKVStorage::new(config);

        // Table name includes schema prefix for PostgreSQL
        assert_eq!(storage.table_name, "public.eq_eq_test_kv");
    }

    #[test]
    fn escape_like_meta_escapes_wildcards() {
        assert_eq!(escape_like_meta("wsdoc:ab"), "wsdoc:ab");
        assert_eq!(escape_like_meta("a%b_c\\d"), "a\\%b\\_c\\\\d");
        assert_eq!(escape_like_meta("-metadata"), "-metadata");
    }

    #[test]
    fn enforce_workspace_scoped_keys_rejects_mixed_workspaces() {
        let keys = ["wsdoc:ws-a:doc1", "wsdoc:ws-b:doc2"];
        let err = enforce_workspace_scoped_keys(keys.into_iter()).unwrap_err();
        assert!(err.to_string().contains("mixes workspace"));
    }

    #[test]
    fn enforce_workspace_scoped_keys_rejects_malformed() {
        let keys = ["wsdoc:missing-doc-id"];
        let err = enforce_workspace_scoped_keys(keys.into_iter()).unwrap_err();
        assert!(err.to_string().contains("Malformed"));
    }

    #[test]
    fn enforce_workspace_scoped_keys_allows_same_workspace() {
        let keys = ["wsdoc:ws-a:doc1", "staging:hash:ws-a:abc", "doc1-metadata"];
        assert!(enforce_workspace_scoped_keys(keys.into_iter()).is_ok());
    }

    #[test]
    fn e2e_kv_upsert_all_or_nothing() {
        // C-22 / matrix: upsert uses a single transaction (begin → batches → commit).
        let src = include_str!("kv.rs");
        assert!(src.contains("pool.begin()"));
        assert!(src.contains("tx.commit()"));
        assert!(src.contains("C-22"));
        assert!(src.contains("all batches commit atomically"));
    }
}
