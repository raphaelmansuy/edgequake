//! Node mutate ops — GraphStorageMutateOps + native upsert (SPEC-054 ISP).
//!
//! Production: native SQL + UNIQUE (`EDGEQUAKE_NATIVE_GRAPH_WRITES`, default ON).
//! Cypher MERGE = debug opt-out. Prefer bound `$1` Cypher for any new write path.

use std::collections::HashMap;
use std::sync::atomic::Ordering;

use super::super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};

impl PostgresAGEGraphStorage {
    /// Upsert a node into the graph.
    ///
    /// # WHY: MERGE-Based Upsert
    ///
    /// Uses Cypher MERGE instead of separate CREATE/UPDATE:
    /// - Atomic: No race conditions between check and insert
    /// - Idempotent: Safe to retry on network failures
    /// - Efficient: Single round-trip vs two queries
    ///
    /// Also triggers lazy index creation on first node.
    pub(in crate::adapters::postgres::graph) async fn pg_upsert_node(
        &self,
        node_id: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // SPEC-059: single-node path must use native ON CONFLICT + eq_merge when
        // enabled — Cypher MERGE races UNIQUE under concurrent upserts.
        if super::super::native_graph_writes_enabled() {
            return self
                .pg_upsert_nodes_batch(&[(node_id.to_string(), properties)])
                .await;
        }

        let escaped_id = Self::escape_cypher_string(node_id);

        // Build properties with node_id included
        let mut props_with_id = properties.clone();
        props_with_id.insert(
            "node_id".to_string(),
            serde_json::Value::String(node_id.to_string()),
        );

        // WHY: AGE 1.6.0 does NOT support `ON CREATE SET` (added only in the
        // unreleased dev branch, apache/age#2347) — it raises "syntax error at
        // or near ON". `SET n = <variable map>` fails with "properties()
        // argument must resolve to a scalar value" (apache/age#1634). The
        // stable, version-safe pattern is per-key `SET n.key = <literal>`
        // expanded inline — verified against AGE 1.6.0 for both fresh and
        // existing vertices. node_id is the MERGE key and is left untouched.
        let mut set_clauses: Vec<String> = Vec::with_capacity(props_with_id.len());
        for (k, v) in &props_with_id {
            if k == "node_id" {
                continue;
            }
            set_clauses.push(format!("n.{} = {}", k, Self::value_to_cypher(v)));
        }
        let set_clause = if set_clauses.is_empty() {
            // node_id-only node: still set node_id so a fresh MERGE persists it.
            format!("n.node_id = '{}'", escaped_id)
        } else {
            set_clauses.join(", ")
        };
        let cypher = format!(
            "MERGE (n:Node {{node_id: '{}'}}) SET {}",
            escaped_id, set_clause
        );

        self.cypher_execute(&cypher).await?;

        // Ensure indexes exist after first node insertion.
        // AGE creates the Node table lazily; SPEC-069: ensure_indexes owns
        // indexes_verified (no force-set) so concurrent workers single-flight DDL.
        if !self.indexes_verified.load(Ordering::Acquire) {
            self.ensure_indexes().await?;
        }

        Ok(())
    }

    /// SC1: batched node upsert using a single `UNWIND ... MERGE` per chunk.
    ///
    /// WHY: the default trait impl issues one `cypher()` round trip per node.
    /// For a document yielding hundreds of entities that is hundreds of network
    /// + planner round trips. UNWIND expands an inline list literal into rows
    ///   inside ONE AGE query, so a 500-node batch becomes a single round trip.
    ///
    /// WHY inline literal (not a bound `$param`): AGE's `cypher()` SQL wrapper
    /// does not forward sqlx bind parameters into the Cypher scope, so the list
    /// must be materialized as a Cypher literal. Every value flows through
    /// `value_to_cypher`, which single-quote-escapes strings, so injection via
    /// node ids/properties is neutralized exactly as in the single-node path.
    ///
    /**
     * @dataop      DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046
     * @engine      apache_age (primary write: native SQL ON CONFLICT when NATIVE_GRAPH_WRITES)
     * @intent      Batch upsert graph vertices; native path uses UNIQUE node_id arbiter.
     * @tables      {graph}."Node"
     * @indexes     UNIQUE expression on properties->>'node_id'
     * @complexity  time: O(K log N) native; Cypher MERGE higher latency
     * @limits      - Adaptive Cypher UNWIND body ≤512KB; chunk dedupe by node_id
     *              - Cypher path debug-only when native disabled
     * @scaling     Linear in K; prefer native
     * @tests       tests/data_layer/data_layer_limits.rs
     * @pgversions  16: ok | 17: ok | 18: ok
     * @docs        specs/088-data-layer/age.md#data-age-graph-upsert-nodes-batch-046
     */
    /// # SPEC-032 W-05: Adaptive UNWIND chunk size
    ///
    /// WHY: a fixed CHUNK=500 with entities that have long descriptions (e.g.,
    /// 500 chars × 12 properties × 500 rows = ~3 MB Cypher literal) can exceed
    /// PostgreSQL's statement size limit and the AGE planner's token budget.
    /// We estimate the average row byte size from the first row and reduce the
    /// chunk to keep the total body under 512 KB.
    pub(in crate::adapters::postgres::graph) async fn pg_upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        let _timer = crate::TimedStorageOp::start_dataop(
            crate::dataop::DATA_AGE_GRAPH_UPSERT_NODES_BATCH_046,
        );
        if nodes.is_empty() {
            return Ok(());
        }

        // SPEC-034 IMP-01: Use native SQL path when feature flag is enabled.
        // WHY: Native SQL INSERT ON CONFLICT DO UPDATE with btree index is
        // O(log G) vs Cypher MERGE GIN scan which is O(G) — ~69× faster.
        // First Principles: arbiter key is node_id — collapse duplicates first.
        let nodes = crate::graph_batch_dedupe::dedupe_nodes_by_id(nodes);
        let nodes = nodes.as_slice();
        if super::super::native_graph_writes_enabled() {
            return self.pg_upsert_nodes_batch_native(nodes).await;
        }
        // IMP-046-01: Cypher MERGE is debug-only — O(G) GIN path, higher locks.
        tracing::warn!(
            target: "edgequake_storage::graph",
            count = nodes.len(),
            "native graph writes disabled — using Cypher MERGE (set EDGEQUAKE_NATIVE_GRAPH_WRITES=1 for O(K log N) ON CONFLICT)"
        );

        // SPEC-032 W-05: Adaptive chunk size — keep UNWIND body ≤ 512 KB.
        let chunk_size = Self::adaptive_unwind_chunk_size(nodes);

        for chunk in nodes.chunks(chunk_size) {
            let rows: Vec<String> = chunk
                .iter()
                .map(|(node_id, properties)| {
                    let mut map = properties.clone();
                    // node_id is the MERGE key; force it into the property map.
                    map.insert(
                        "node_id".to_string(),
                        serde_json::Value::String(node_id.clone()),
                    );
                    Self::properties_to_cypher(&map)
                })
                .collect();

            // WHY: AGE 1.6.0 does NOT support `ON CREATE SET` (apache/age#2347
            // is unreleased) — it raises "syntax error at or near ON". `SET n =
            // props` (variable map) fails with "properties() argument must
            // resolve to a scalar value" (apache/age#1634). The version-safe
            // pattern is per-key `SET n.key = props.key` referencing the UNWIND
            // row variable — verified against AGE 1.6.0 to persist on both
            // freshly-MERGEd and existing vertices. Keys are surfaced inline
            // (AGE's cypher() SQL wrapper does not forward sqlx bind params).
            let mut set_keys: Vec<&str> = Vec::with_capacity(64);
            // Collect the property key set from the first row (uniform schema
            // across entities in a batch). node_id is the MERGE key — excluded.
            if let Some((_, props)) = chunk.first() {
                for k in props.keys() {
                    if k != "node_id" {
                        set_keys.push(k.as_str());
                    }
                }
            }
            let set_clause = if set_keys.is_empty() {
                String::new()
            } else {
                let sets: Vec<String> = set_keys
                    .iter()
                    .map(|k| format!("n.{} = props.{}", k, k))
                    .collect();
                format!(" SET {}", sets.join(", "))
            };
            let cypher = format!(
                "UNWIND [{}] AS props \
                 MERGE (n:Node {{node_id: props.node_id}}){}",
                rows.join(", "),
                set_clause
            );
            self.cypher_execute(&cypher).await?;
        }

        // Lazily create indexes after the first successful batch (SPEC-069).
        if !self.indexes_verified.load(Ordering::Acquire) {
            self.ensure_indexes().await?;
        }

        Ok(())
    }

    /// SPEC-032 W-05: Compute adaptive UNWIND chunk size for node batches.
    ///
    /// Samples the first row to estimate bytes-per-row; caps total body at
    /// 512 KB to avoid PostgreSQL statement-size and AGE planner limits.
    ///
    /// Bounds: [50, 500] to guarantee at least some batching and stay within
    /// safe limits even for entities with very long descriptions.
    pub(super) fn adaptive_unwind_chunk_size(
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> usize {
        const MAX_BODY_BYTES: usize = 512 * 1024; // 512 KB
        const MIN_CHUNK: usize = 50;
        const MAX_CHUNK: usize = 500;

        if let Some((_, props)) = nodes.first() {
            // Estimate row size: sum of serialised property values + key names + punctuation
            let estimated_row: usize = props
                .iter()
                .map(|(k, v)| k.len() + v.to_string().len() + 8) // 8 bytes overhead per key:val
                .sum::<usize>()
                + 16; // node_id + struct punctuation

            let adaptive = MAX_BODY_BYTES
                .checked_div(estimated_row)
                .map(|n| n.clamp(MIN_CHUNK, MAX_CHUNK))
                .unwrap_or(MAX_CHUNK);
            // SPEC-047 P7f: env-tunable cap over adaptive estimate.
            let cap = crate::graph_batch_dedupe::resolve_graph_upsert_chunk(adaptive);
            tracing::trace!(
                estimated_row_bytes = estimated_row,
                adaptive_chunk = adaptive,
                resolved_chunk = cap,
                "UNWIND node chunk size (SPEC-032 W-05 / SPEC-047 P7f)"
            );
            return cap;
        }
        crate::graph_batch_dedupe::resolve_graph_upsert_chunk(MAX_CHUNK)
    }

    pub(in crate::adapters::postgres::graph) async fn pg_delete_node(
        &self,
        node_id: &str,
    ) -> Result<()> {
        if super::super::native_graph_writes_enabled() {
            return self.pg_delete_nodes_batch(&[node_id.to_string()]).await;
        }
        let cypher = "MATCH (n:Node {node_id: $node_id}) DETACH DELETE n";
        let params = serde_json::json!({ "node_id": node_id });
        self.cypher_execute_bound(cypher, &params).await
    }

    /// SPEC-060: native DETACH-equivalent — delete incident EDGE rows then Node rows
    /// via UNIQUE `node_id` expression index (`ANY($1)`), not per-id Cypher.
    pub(in crate::adapters::postgres::graph) async fn pg_delete_nodes_batch(
        &self,
        node_ids: &[String],
    ) -> Result<()> {
        if node_ids.is_empty() {
            return Ok(());
        }
        if !super::super::native_graph_writes_enabled() {
            for id in node_ids {
                let cypher = "MATCH (n:Node {node_id: $node_id}) DETACH DELETE n";
                let params = serde_json::json!({ "node_id": id });
                self.cypher_execute_bound(cypher, &params).await?;
            }
            return Ok(());
        }

        let pool = self.pool.get().await?;
        let graph = &self.graph_name;
        let mut unique = node_ids.to_vec();
        unique.sort();
        unique.dedup();

        // Detach: edges where either end is in the delete set (COALESCE eq_* when present).
        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| StorageError::Connection(format!("Failed to acquire connection: {e}")))?;
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let src = if eq_present {
            super::super::helpers::coalesce_endpoint("e", "source")
        } else {
            super::super::helpers::prop_only_endpoint("e", "source")
        };
        let tgt = if eq_present {
            super::super::helpers::coalesce_endpoint("e", "target")
        } else {
            super::super::helpers::prop_only_endpoint("e", "target")
        };
        let node_key = if eq_present {
            super::super::helpers::coalesce_endpoint("n", "node")
        } else {
            super::super::helpers::prop_only_endpoint("n", "node")
        };
        let del_edges = format!(
            r#"/* DATA-AGE-GRAPH-DELETE-NODES-BATCH detach */
               DELETE FROM {graph}."EDGE" e
               WHERE {src} = ANY($1::text[]) OR {tgt} = ANY($1::text[])"#
        );
        sqlx::query(&del_edges)
            .bind(&unique)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("native batch edge detach failed: {e}")))?;

        let del_nodes = format!(
            r#"/* DATA-AGE-GRAPH-DELETE-NODES-BATCH */
               DELETE FROM {graph}."Node" n
               WHERE {node_key} = ANY($1::text[])"#
        );
        sqlx::query(&del_nodes)
            .bind(&unique)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("native batch node delete failed: {e}")))?;
        Ok(())
    }

    /// Tenant-scoped delete — no cross-tenant IDOR (IMP-031-05 native when enabled).
    pub(in crate::adapters::postgres::graph) async fn pg_delete_node_scoped(
        &self,
        node_id: &str,
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<bool> {
        if !super::super::native_graph_writes_enabled() {
            let escaped_id = Self::escape_cypher_string(node_id);
            let escaped_tid = Self::escape_cypher_string(tenant_id);
            let escaped_wid = Self::escape_cypher_string(workspace_id);
            let cypher = format!(
                "MATCH (n:Node {{node_id: '{escaped_id}'}}) \
                 WHERE n.tenant_id = '{escaped_tid}' AND n.workspace_id = '{escaped_wid}' \
                 DETACH DELETE n \
                 RETURN n"
            );
            let rows = self.cypher_query(&cypher, &["n"]).await?;
            return Ok(!rows.is_empty());
        }

        // Verify node is in tenant/ws before detach-delete.
        let Some(node) = self.pg_get_node(node_id).await? else {
            return Ok(false);
        };
        let nt = node
            .properties
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let nw = node
            .properties
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if nt != tenant_id || nw != workspace_id {
            return Ok(false);
        }
        self.pg_delete_nodes_batch(&[node_id.to_string()]).await?;
        Ok(true)
    }

    /// SPEC-034 IMP-01: Native SQL batch node upsert — O(log G) per node.
    ///
    /// # WHY: Replace Cypher MERGE GIN scan with native SQL btree lookup
    ///
    /// AGE's `cypher()` UDF compiles `MERGE (n:Node {node_id: 'X'})` into a
    /// GIN containment scan (`properties @> '{"node_id":"X"}'`). At 50K nodes
    /// this takes ~5.6ms per node. Native SQL uses the btree index on
    /// `(agtype_to_json(properties)->>'node_id')` which costs ~0.081ms — 69×
    /// faster.
    ///
    /// # Enabled by
    /// `EDGEQUAKE_NATIVE_GRAPH_WRITES=1` environment variable.
    ///
    /// # AGE Compatibility (verified on AGE 1.6.0)
    ///
    /// AGE nodes are stored in `"<graph>"."Node"` (id: graphid, properties: agtype).
    /// `eq_next_node_id(graph_name)` (Migration 076) generates valid graphids via:
    ///   `(label_id << 48) | nextval(seq)` cast through `::text::ag_catalog.graphid`.
    ///
    /// The agtype cast uses `::ag_catalog.agtype` (NOT `::jsonb::agtype`).
    /// AGE registers no jsonb→agtype cast; the input function `agtype_in`
    /// accepts text in agtype format, making the text→agtype path correct.
    ///
    /// # Conflict Resolution (LightRAG merge semantics)
    ///
    /// ON CONFLICT uses the UNIQUE index `idx_node_prop_node_id_unique`
    /// (Migration 074). DO UPDATE SET applies last-writer-wins property
    /// update, identical to Cypher `MERGE ... SET n.key = new_value`.
    ///
    /// # Monitoring
    ///
    /// Logs a WARNING when the batch exceeds 500ms to detect regressions early.
    pub(in crate::adapters::postgres::graph) async fn pg_upsert_nodes_batch_native(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }
        // First Principles: arbiter key is node_id — collapse duplicates first.
        let nodes = crate::graph_batch_dedupe::dedupe_nodes_by_id(nodes);
        let nodes = nodes.as_slice();
        let start = std::time::Instant::now();
        // SPEC-062 / SPEC-069: eq_* must exist before native INSERT.
        // Boot owns DDL; hot path only ensures once (single-flight). Fail closed
        // if schema still missing — never migrate mid-delete under query timeout.
        if !self.indexes_verified.load(Ordering::Acquire) {
            self.ensure_indexes().await?;
        }
        if !self.indexes_verified.load(Ordering::Acquire) {
            return Err(StorageError::Database(
                "graph schema not bootstrapped (eq_id)".into(),
            ));
        }
        // Mirror Cypher adaptive chunking — large unnest() statements lock longer
        // and risk statement-size / planner blowups on multi-thousand entity docs.
        let chunk_size = Self::adaptive_unwind_chunk_size(nodes);

        for chunk in nodes.chunks(chunk_size) {
            self.pg_upsert_nodes_batch_native_chunk(chunk).await?;
        }

        let elapsed = start.elapsed();
        if elapsed.as_millis() > 500 {
            tracing::warn!(
                batch_size = nodes.len(),
                chunk_size,
                elapsed_ms = elapsed.as_millis(),
                "SPEC-034 IMP-01: Native node batch upsert exceeded 500ms threshold"
            );
        }
        tracing::debug!(
            batch_size = nodes.len(),
            chunk_size,
            elapsed_ms = elapsed.as_millis(),
            "SPEC-034 IMP-01: Native node batch upsert completed"
        );

        Ok(())
    }

    async fn pg_upsert_nodes_batch_native_chunk(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        let pool = self.pool.get().await?;
        let graph = &self.graph_name;

        let mut node_ids: Vec<String> = Vec::with_capacity(nodes.len());
        let mut props_json: Vec<String> = Vec::with_capacity(nodes.len());

        for (id, props) in nodes {
            node_ids.push(id.clone());
            let mut full = props.clone();
            full.insert("node_id".to_string(), serde_json::Value::String(id.clone()));
            props_json.push(serde_json::to_string(&full).unwrap_or_else(|_| "{}".to_string()));
        }

        // Conflict target matches idx_node_prop_node_id_unique (Migration 074/083).
        // DISTINCT ON is a SQL safety net (same policy as edge native upsert).
        // SPEC-062: prefer eq_node_id arbiter when column exists (ensure_eq_id_columns).
        // Fallback to expression unique if denormalized columns not yet present.
        let sql = format!(
            r#"
            INSERT INTO {graph}."Node" (id, properties, eq_node_id)
            SELECT
                eq_next_node_id('{graph}'),
                d.props_text::ag_catalog.agtype,
                d.node_id_val
            FROM (
                SELECT DISTINCT ON (node_id_val)
                    node_id_val,
                    props_text
                FROM unnest($1::text[], $2::text[])
                       WITH ORDINALITY AS p(node_id_val, props_text, ord)
                ORDER BY node_id_val, ord DESC
            ) AS d
            ON CONFLICT (eq_node_id) WHERE eq_node_id IS NOT NULL
            DO UPDATE SET
                properties = EXCLUDED.properties,
                eq_node_id = EXCLUDED.eq_node_id
            "#,
            graph = graph
        );

        sqlx::query(&sql)
            .bind(&node_ids)
            .bind(&props_json)
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Native SQL node batch upsert failed: {e}"))
            })?;

        Ok(())
    }
}
