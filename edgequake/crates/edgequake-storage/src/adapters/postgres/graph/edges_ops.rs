use std::collections::HashMap;

use sqlx::Row;

use super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::GraphEdge;

impl PostgresAGEGraphStorage {
    /// IMP-031-03: native edge endpoint lookup — O(log E) via source/target indexes.
    pub(super) async fn pg_has_edge(&self, source: &str, target: &str) -> Result<bool> {
        Ok(self.pg_get_edge(source, target).await?.is_some())
    }

    /**
     * @dataop      DATA-AGE-GRAPH-GET-EDGE-034
     * @engine      apache_age (native SQL primary; IMP-031-03)
     * @intent      Fetch single directed edge by (source_id, target_id) — O(log E).
     * @indexes     EDGE eq_source/target or expression btree on properties
     * @complexity  time: O(log E); space: O(1)
     */
    pub(super) async fn pg_get_edge(
        &self,
        source: &str,
        target: &str,
    ) -> Result<Option<GraphEdge>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let src = if eq_present {
            super::helpers::coalesce_endpoint("e", "source")
        } else {
            super::helpers::prop_only_endpoint("e", "source")
        };
        let tgt = if eq_present {
            super::helpers::coalesce_endpoint("e", "target")
        } else {
            super::helpers::prop_only_endpoint("e", "target")
        };
        let sql = format!(
            "/* DATA-AGE-GRAPH-GET-EDGE-034 */ \
             SELECT ag_catalog.agtype_to_json(e.properties) AS props \
             FROM {graph}.\"EDGE\" e \
             WHERE {src} = $1 AND {tgt} = $2 \
             LIMIT 1",
            graph = self.graph_name,
            src = src,
            tgt = tgt,
        );
        let rows = sqlx::query(&sql)
            .bind(source)
            .bind(target)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("get_edge native failed: {e}")))?;
        Ok(Self::edges_from_props_rows(&rows).into_iter().next())
    }

    pub(super) async fn pg_upsert_edge(
        &self,
        source: &str,
        target: &str,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // IMP-046 / IMP-031: single-edge write uses native batch-of-1 when enabled.
        if super::native_graph_writes_enabled() {
            return self
                .pg_upsert_edges_batch(&[(source.to_string(), target.to_string(), properties)])
                .await;
        }
        tracing::warn!(
            target: "edgequake_storage::graph",
            "native graph writes disabled — single edge Cypher MERGE (debug path)"
        );
        let escaped_source = Self::escape_cypher_string(source);
        let escaped_target = Self::escape_cypher_string(target);

        // Build properties with source_id and target_id
        let mut props_with_ids = properties.clone();
        props_with_ids.insert(
            "source_id".to_string(),
            serde_json::Value::String(source.to_string()),
        );
        props_with_ids.insert(
            "target_id".to_string(),
            serde_json::Value::String(target.to_string()),
        );
        // WHY: AGE 1.6.0 does NOT support `ON CREATE SET` (apache/age#2347 is
        // unreleased) — "syntax error at or near ON". `SET r = <variable map>`
        // fails (apache/age#1634). The version-safe pattern is per-key
        // `SET r.key = <literal>` expanded inline — verified against AGE 1.6.0
        // to persist on both freshly-MERGEd and existing edges. source_id /
        // target_id are the MERGE key and are persisted by the MERGE pattern.
        let mut set_clauses: Vec<String> = Vec::with_capacity(props_with_ids.len());
        for (k, v) in &props_with_ids {
            if k == "source_id" || k == "target_id" {
                continue;
            }
            set_clauses.push(format!("r.{} = {}", k, Self::value_to_cypher(v)));
        }
        let set_clause = if set_clauses.is_empty() {
            String::new()
        } else {
            format!(" SET {}", set_clauses.join(", "))
        };
        let cypher = format!(
            "MERGE (a:Node {{node_id: '{src}'}}) \
             MERGE (b:Node {{node_id: '{tgt}'}}) \
             MERGE (a)-[r:EDGE {{source_id: '{src}', target_id: '{tgt}'}}]->(b){set_clause}",
            src = escaped_source,
            tgt = escaped_target,
            set_clause = set_clause
        );
        self.cypher_execute(&cypher).await
    }

    /// SC1: batched edge upsert using a single `UNWIND ... MERGE` per chunk.
    ///
    /// WHY: same round-trip collapse as `upsert_nodes_batch`. Each row carries
    /// `source_id`/`target_id` plus the edge properties; MERGE on the endpoint
    /// nodes then MERGE on the relationship keyed by (source_id, target_id)
    /// guarantees at-most-one edge per pair (no DELETE/CREATE race), and
    /// `SET r.key = e.key` (per-key) applies last-write-wins property updates.
    ///
    /// # SPEC-032 W-05: Adaptive UNWIND chunk size (same logic as node batch)
    pub(super) async fn pg_upsert_edges_batch(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        if edges.is_empty() {
            return Ok(());
        }

        // First Principles: ON CONFLICT / Cypher MERGE keys are (source_id, target_id).
        // Collapse duplicates here so every write path (native + Cypher) is safe.
        let edges = crate::graph_batch_dedupe::dedupe_edges_by_endpoints(edges);
        let edges = edges.as_slice();

        // SPEC-034 IMP-01: Use native SQL path when feature flag is enabled.
        if super::native_graph_writes_enabled() {
            return self.pg_upsert_edges_batch_native(edges).await;
        }

        // SPEC-032 W-05: adaptive chunk based on estimated row bytes.
        let chunk_size = Self::adaptive_edge_chunk_size(edges);

        for chunk in edges.chunks(chunk_size) {
            let rows: Vec<String> = chunk
                .iter()
                .map(|(source, target, properties)| {
                    let mut map = properties.clone();
                    map.insert(
                        "source_id".to_string(),
                        serde_json::Value::String(source.clone()),
                    );
                    map.insert(
                        "target_id".to_string(),
                        serde_json::Value::String(target.clone()),
                    );
                    Self::properties_to_cypher(&map)
                })
                .collect();

            // WHY: AGE 1.6.0 does NOT support `ON CREATE SET` (apache/age#2347
            // is unreleased) and `SET r = <variable map>` fails
            // (apache/age#1634). The version-safe pattern is per-key
            // `SET r.key = e.key` referencing the UNWIND row — verified against
            // AGE 1.6.0 to persist on both fresh and existing edges.
            // source_id/target_id are the MERGE key (persisted by MERGE).
            let mut set_keys: Vec<&str> = Vec::with_capacity(32);
            if let Some((_, _, props)) = chunk.first() {
                for k in props.keys() {
                    if k != "source_id" && k != "target_id" {
                        set_keys.push(k.as_str());
                    }
                }
            }
            let set_clause = if set_keys.is_empty() {
                String::new()
            } else {
                let sets: Vec<String> = set_keys
                    .iter()
                    .map(|k| format!("r.{} = e.{}", k, k))
                    .collect();
                format!(" SET {}", sets.join(", "))
            };
            let cypher = format!(
                "UNWIND [{}] AS e \
                 MERGE (a:Node {{node_id: e.source_id}}) \
                 MERGE (b:Node {{node_id: e.target_id}}) \
                 MERGE (a)-[r:EDGE {{source_id: e.source_id, target_id: e.target_id}}]->(b){}",
                rows.join(", "),
                set_clause
            );
            self.cypher_execute(&cypher).await?;
        }

        Ok(())
    }

    /// SPEC-032 W-05: Adaptive UNWIND chunk size for edge batches.
    fn adaptive_edge_chunk_size(
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> usize {
        const MAX_BODY_BYTES: usize = 512 * 1024;
        const MIN_CHUNK: usize = 50;
        const MAX_CHUNK: usize = 500;

        if let Some((src, tgt, props)) = edges.first() {
            let estimated_row: usize = props
                .iter()
                .map(|(k, v)| k.len() + v.to_string().len() + 8)
                .sum::<usize>()
                + src.len()
                + tgt.len()
                + 24; // source_id + target_id + struct overhead
            let adaptive = MAX_BODY_BYTES
                .checked_div(estimated_row)
                .map(|n| n.clamp(MIN_CHUNK, MAX_CHUNK))
                .unwrap_or(MAX_CHUNK);
            // SPEC-047 P7f: env-tunable cap over adaptive estimate.
            return crate::graph_batch_dedupe::resolve_graph_upsert_chunk(adaptive);
        }
        crate::graph_batch_dedupe::resolve_graph_upsert_chunk(MAX_CHUNK)
    }

    /**
     * @dataop      DATA-AGE-GRAPH-DELETE-EDGE-052
     * @engine      apache_age (native SQL primary; IMP-031-05)
     * @intent      Delete directed edge by (source, target) — O(log E).
     */
    pub(super) async fn pg_delete_edge(&self, source: &str, target: &str) -> Result<()> {
        if super::native_graph_writes_enabled() {
            return self
                .pg_delete_edges_batch(&[(source.to_string(), target.to_string())])
                .await;
        }
        tracing::warn!(
            target: "edgequake_storage::graph",
            "native graph writes disabled — Cypher edge DELETE (debug path)"
        );
        let cypher =
            "MATCH (a:Node {node_id: $source_id})-[r:EDGE]->(b:Node {node_id: $target_id}) DELETE r";
        let params = serde_json::json!({ "source_id": source, "target_id": target });
        self.cypher_execute_bound(cypher, &params).await
    }

    /// Batch-delete edges by `(source, target)` pairs.
    ///
    /// Native path: one SQL DELETE with unnested pairs on EDGE endpoint indexes
    /// (SPEC-060 / IMP-031-05). Cypher fallback loops when native writes disabled.
    pub(super) async fn pg_delete_edges_batch(&self, edges: &[(String, String)]) -> Result<()> {
        if edges.is_empty() {
            return Ok(());
        }

        let mut unique: Vec<(String, String)> = edges.to_vec();
        unique.sort();
        unique.dedup();

        if !super::native_graph_writes_enabled() {
            for (source, target) in &unique {
                // Avoid re-entering batch (would recurse when flag off).
                let cypher =
                    "MATCH (a:Node {node_id: $source_id})-[r:EDGE]->(b:Node {node_id: $target_id}) DELETE r";
                let params = serde_json::json!({ "source_id": source, "target_id": target });
                self.cypher_execute_bound(cypher, &params).await?;
            }
            return Ok(());
        }

        let sources: Vec<String> = unique.iter().map(|(s, _)| s.clone()).collect();
        let targets: Vec<String> = unique.iter().map(|(_, t)| t.clone()).collect();

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;
        let graph = &self.graph_name;
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let src = if eq_present {
            super::helpers::coalesce_endpoint("e", "source")
        } else {
            super::helpers::prop_only_endpoint("e", "source")
        };
        let tgt = if eq_present {
            super::helpers::coalesce_endpoint("e", "target")
        } else {
            super::helpers::prop_only_endpoint("e", "target")
        };
        // Pairwise match via UNNEST — avoids cartesian ANY×ANY false positives.
        let del_edges = format!(
            r#"/* DATA-AGE-GRAPH-DELETE-EDGES-BATCH */
               DELETE FROM {graph}."EDGE" e
               USING (
                 SELECT * FROM unnest($1::text[], $2::text[]) AS t(source_id, target_id)
               ) pairs
               WHERE {src} = pairs.source_id
                 AND {tgt} = pairs.target_id"#
        );
        sqlx::query(&del_edges)
            .bind(&sources)
            .bind(&targets)
            .execute(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("native batch edge delete failed: {e}")))?;
        Ok(())
    }

    /// Tenant-scoped edge delete — strict match (IMP-031-05 native).
    pub(super) async fn pg_delete_edge_scoped(
        &self,
        source: &str,
        target: &str,
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<bool> {
        if !super::native_graph_writes_enabled() {
            let src = Self::escape_cypher_string(source);
            let tgt = Self::escape_cypher_string(target);
            let tid = Self::escape_cypher_string(tenant_id);
            let wid = Self::escape_cypher_string(workspace_id);
            let cypher = format!(
                "MATCH (a:Node {{node_id: '{src}'}})-[r:EDGE]->(b:Node {{node_id: '{tgt}'}}) \
                 WHERE r.tenant_id = '{tid}' AND r.workspace_id = '{wid}' \
                 DELETE r \
                 RETURN r"
            );
            let rows = self.cypher_query(&cypher, &["r"]).await?;
            return Ok(!rows.is_empty());
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;
        let graph = &self.graph_name;
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let src_e = if eq_present {
            super::helpers::coalesce_endpoint("e", "source")
        } else {
            super::helpers::prop_only_endpoint("e", "source")
        };
        let tgt_e = if eq_present {
            super::helpers::coalesce_endpoint("e", "target")
        } else {
            super::helpers::prop_only_endpoint("e", "target")
        };
        let del = format!(
            r#"/* DATA-AGE-GRAPH-DELETE-EDGE-SCOPED */
               DELETE FROM {graph}."EDGE" e
               WHERE {src_e} = $1 AND {tgt_e} = $2
                 AND COALESCE(ag_catalog.agtype_to_json(e.properties)->>'tenant_id', '') = $3
                 AND COALESCE(ag_catalog.agtype_to_json(e.properties)->>'workspace_id', '') = $4
               RETURNING 1"#
        );
        let rows = sqlx::query(&del)
            .bind(source)
            .bind(target)
            .bind(tenant_id)
            .bind(workspace_id)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| {
                StorageError::Database(format!("native scoped edge delete failed: {e}"))
            })?;
        Ok(!rows.is_empty())
    }

    pub(super) async fn pg_get_node_edges(&self, node_id: &str) -> Result<Vec<GraphEdge>> {
        self.pg_get_incident_edges_batch(&[node_id.to_string()], None, None)
            .await
    }

    /// Batch incident-edge lookup via indexed property scan on the "EDGE" child table
    /// (SPEC-025 6.2 / SPEC-053 hardening / SPEC-053.1 json-equality fix).
    ///
    /// # WHY: O(k log E) instead of O(V + E) (SPEC-053 root cause fix)
    ///
    /// The previous implementation joined the AGE parent edge table to the AGE parent
    /// vertex table to resolve edge endpoints. M070 dropped all parent-table indexes
    /// (they were confirmed "0 scans"), so that approach forced a full sequential scan
    /// on every BFS frontier step, causing statement-timeout errors on large graphs.
    ///
    /// Every edge already stores `source_id` and `target_id` as explicit properties
    /// (set by `pg_upsert_edge` / `pg_upsert_edges_batch`). The `"EDGE"` child table
    /// has two btree expression indexes on exactly those columns:
    ///   - `idx_edge_source_id`  ON "EDGE" ((agtype_to_json(properties)->>'source_id'))
    ///   - `idx_edge_target_id`  ON "EDGE" ((agtype_to_json(properties)->>'target_id'))
    ///
    /// # WHY OR (not UNION)
    ///
    /// `UNION` (set union) requires PostgreSQL to compare rows for deduplication.
    /// `agtype_to_json()` returns the PostgreSQL `json` type, which deliberately has
    /// **no equality operator** (only `jsonb` has one — PG docs §9.16.1).
    /// Using `UNION` on a `json` column produces:
    ///   "could not identify an equality operator for type json"
    ///
    /// `OR` is the correct alternative: it is a single-scan predicate that never
    /// needs to compare the `json` column values. PostgreSQL resolves `OR` predicates
    /// across two different indexed columns via **Bitmap OR** — the planner issues
    /// two BitmapIndexScans (one per property index) and merges the result bitmaps.
    /// Expected plan:
    ///   BitmapHeapScan("EDGE")
    ///     → BitmapOr
    ///         → BitmapIndexScan(idx_edge_source_id, source_id IN list)
    ///         → BitmapIndexScan(idx_edge_target_id, target_id IN list)
    ///
    /// Each edge row is returned at most once (OR semantics — no duplicates).
    pub(super) async fn pg_get_incident_edges_batch(
        &self,
        node_ids: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<GraphEdge>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let mut unique: Vec<String> = node_ids.to_vec();
        unique.sort();
        unique.dedup();

        // SPEC-058: Strict RAG isolation (missing tenant/workspace on edge → excluded).
        let scope_filter = crate::traits::EdgeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            relationship_type: None,
        };
        let scope_clause = Self::edge_and_clause(
            "e",
            &scope_filter,
            super::helpers::EdgeTenantFilterMode::Strict,
        );

        // WHY 200 (was 100): each chunk now costs O(log E) not O(V), so we can
        // double the batch size to halve the number of round-trips.
        const CHUNK: usize = 200;
        let mut all_edges = Vec::new();

        for chunk in unique.chunks(CHUNK) {
            let in_list: String = chunk
                .iter()
                .map(|id| format!("'{}'", Self::escape_sql_string(id)))
                .collect::<Vec<_>>()
                .join(", ");

            // WHY "EDGE" child table (not "_ag_label_edge" parent):
            //   idx_edge_source_id / idx_edge_target_id live on the child table.
            //   Querying the parent forces a seq-scan because M070 dropped all
            //   parent-table indexes.
            // WHY OR (not UNION): agtype_to_json returns `json` (not `jsonb`).
            //   `json` has no equality operator in PostgreSQL, so UNION (which
            //   deduplicates via equality) raises "could not identify an equality
            //   operator for type json". OR evaluates as a BitmapOr of two index
            //   scans without ever comparing the json column values.
            // SPEC-083 / X-03: COALESCE(eq_*, props) when columns exist; prop-only otherwise.
            let eq_present = self.eq_columns_present(&mut conn).await?;
            let src = if eq_present {
                super::helpers::coalesce_endpoint("e", "source")
            } else {
                super::helpers::prop_only_endpoint("e", "source")
            };
            let tgt = if eq_present {
                super::helpers::coalesce_endpoint("e", "target")
            } else {
                super::helpers::prop_only_endpoint("e", "target")
            };
            if !eq_present || super::helpers::eq_id_fallback_env_enabled() {
                tracing::debug!(
                    target: "edgequake_storage",
                    eq_present,
                    "eq_id_fallback_used: pg_get_incident_edges_batch"
                );
            }
            let sql = format!(
                "SELECT ag_catalog.agtype_to_json(e.properties) AS props \
                 FROM {graph}.\"EDGE\" e \
                 WHERE ({src} IN ({in_list}) OR {tgt} IN ({in_list}))\
                 {scope}",
                graph = self.graph_name,
                in_list = in_list,
                scope = scope_clause,
                src = src,
                tgt = tgt,
            );

            let rows = sqlx::query(&sql).fetch_all(&mut *conn).await.map_err(|e| {
                StorageError::Database(format!("Batch incident edges query failed: {}", e))
            })?;

            all_edges.extend(Self::edges_from_props_rows(&rows));
        }

        Ok(all_edges)
    }

    /// Extract `GraphEdge` values from rows that expose a single `props` column
    /// (the result of `ag_catalog.agtype_to_json(e.properties)`).
    ///
    /// WHY: `source_id` and `target_id` are stored as named properties on every
    /// edge (invariant established by `pg_upsert_edge`). We read them from `props`
    /// directly — no JOIN to the vertex table needed.
    fn edges_from_props_rows(rows: &[sqlx::postgres::PgRow]) -> Vec<GraphEdge> {
        rows.iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let source = props.get("source_id")?.as_str()?.to_string();
                let target = props.get("target_id")?.as_str()?.to_string();
                let properties = props.as_object()?.clone().into_iter().collect();
                Some(GraphEdge {
                    source,
                    target,
                    properties,
                })
            })
            .collect()
    }

    /// ADMIN / dump path (FORBIDDEN on hot HTTP) — native scan (IMP-031-07).
    /// Complexity: O(E) unavoidable; skips AGE Cypher traversal overhead.
    pub(super) async fn pg_get_all_edges(&self) -> Result<Vec<GraphEdge>> {
        let pool = self.pool.get().await?;
        let sql = format!(
            r#"/* DATA-AGE-GRAPH-GET-ALL-EDGES */
               SELECT ag_catalog.agtype_to_json(e.properties) AS props
               FROM {}."EDGE" e"#,
            self.graph_name
        );
        let rows = sqlx::query(&sql)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("get_all_edges native failed: {e}")))?;
        Ok(Self::edges_from_props_rows(&rows))
    }

    /// SPEC-034 IMP-01: Native SQL batch edge upsert — O(log G) per edge.
    ///
    /// # WHY: Replace Cypher MERGE with native INSERT ON CONFLICT DO UPDATE
    ///
    /// Cypher MERGE for edges does GIN containment scans on BOTH endpoint nodes
    /// plus the edge table. Native SQL uses the btree expression indexes on
    /// `(source_id::text)` and `(target_id::text)` added in Migration 072.
    ///
    /// # Prerequisite
    ///
    /// This method requires that the endpoint nodes already exist in the "Node"
    /// table (written by `pg_upsert_nodes_batch_native` first). The edge
    /// references start_id and end_id by graphid, looked up via the btree index.
    ///
    /// # Monitoring
    ///
    /// Logs a WARNING when the batch exceeds 800ms to detect regressions.
    pub(super) async fn pg_upsert_edges_batch_native(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        if edges.is_empty() {
            return Ok(());
        }
        // Defense in depth: callers may skip the Cypher-path dedupe entrypoint.
        let edges = crate::graph_batch_dedupe::dedupe_edges_by_endpoints(edges);
        let edges = edges.as_slice();
        let start = std::time::Instant::now();
        // SPEC-062 / SPEC-069: eq_* must exist before native INSERT (fail closed).
        if !self
            .indexes_verified
            .load(std::sync::atomic::Ordering::Acquire)
        {
            self.ensure_indexes().await?;
        }
        if !self
            .indexes_verified
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(crate::error::StorageError::Database(
                "graph schema not bootstrapped (eq_id)".into(),
            ));
        }
        let chunk_size = Self::adaptive_edge_chunk_size(edges);
        let mut inserted_or_updated = 0u64;
        let expected = edges.len() as u64;

        for chunk in edges.chunks(chunk_size) {
            inserted_or_updated += self.pg_upsert_edges_batch_native_chunk(chunk).await?;
        }

        // INNER JOIN drops edges whose endpoints are missing — surface that loudly.
        if inserted_or_updated < expected {
            tracing::warn!(
                expected,
                applied = inserted_or_updated,
                dropped = expected.saturating_sub(inserted_or_updated),
                "SPEC-034 IMP-01: Native edge upsert applied fewer rows than input — missing endpoint nodes?"
            );
        }

        let elapsed = start.elapsed();
        if elapsed.as_millis() > 800 {
            tracing::warn!(
                batch_size = edges.len(),
                chunk_size,
                elapsed_ms = elapsed.as_millis(),
                "SPEC-034 IMP-01: Native edge batch upsert exceeded 800ms threshold"
            );
        }
        tracing::debug!(
            batch_size = edges.len(),
            chunk_size,
            applied = inserted_or_updated,
            elapsed_ms = elapsed.as_millis(),
            "SPEC-034 IMP-01: Native edge batch upsert completed"
        );

        Ok(())
    }

    async fn pg_upsert_edges_batch_native_chunk(
        &self,
        edges: &[(String, String, HashMap<String, serde_json::Value>)],
    ) -> Result<u64> {
        let pool = self.pool.get().await?;
        let graph = &self.graph_name;

        let mut source_ids: Vec<String> = Vec::with_capacity(edges.len());
        let mut target_ids: Vec<String> = Vec::with_capacity(edges.len());
        let mut rel_types: Vec<String> = Vec::with_capacity(edges.len());
        let mut props_json: Vec<String> = Vec::with_capacity(edges.len());

        for (src, tgt, props) in edges {
            source_ids.push(src.clone());
            target_ids.push(tgt.clone());
            let rel = crate::graph_batch_dedupe::normalize_rel_type(props);
            rel_types.push(rel.clone());
            let mut full = props.clone();
            full.insert(
                "source_id".to_string(),
                serde_json::Value::String(src.clone()),
            );
            full.insert(
                "target_id".to_string(),
                serde_json::Value::String(tgt.clone()),
            );
            // Keep properties.relation_type aligned with arbiter column.
            full.insert("relation_type".to_string(), serde_json::Value::String(rel));
            props_json.push(serde_json::to_string(&full).unwrap_or_else(|_| "{}".to_string()));
        }

        // D-30 / SPEC-083: arbiter is (eq_source_id, eq_target_id, eq_rel_type)
        // so Alice-KNOWS-Bob and Alice-WORKS_WITH-Bob both persist.
        // DISTINCT ON is a SQL safety net if a caller bypasses Rust dedupe
        // (Postgres forbids ON CONFLICT DO UPDATE affecting a row twice).
        // ORDER BY … ord DESC → last-write-wins, matching Rust policy.
        let sql = format!(
            r#"
            INSERT INTO {graph}."EDGE" (id, start_id, end_id, properties, eq_source_id, eq_target_id, eq_rel_type)
            SELECT
                eq_next_edge_id('{graph}'),
                j.start_id,
                j.end_id,
                j.props_text::ag_catalog.agtype,
                j.source_id_val,
                j.target_id_val,
                j.rel_type_val
            FROM (
                SELECT DISTINCT ON (d.source_id_val, d.target_id_val, d.rel_type_val)
                    sn.id AS start_id,
                    tn.id AS end_id,
                    d.props_text,
                    d.source_id_val,
                    d.target_id_val,
                    d.rel_type_val
                FROM (
                    SELECT DISTINCT ON (source_id_val, target_id_val, rel_type_val)
                        source_id_val,
                        target_id_val,
                        rel_type_val,
                        props_text
                    FROM unnest($1::text[], $2::text[], $3::text[], $4::text[])
                           WITH ORDINALITY AS p(source_id_val, target_id_val, rel_type_val, props_text, ord)
                    ORDER BY source_id_val, target_id_val, rel_type_val, ord DESC
                ) AS d
                JOIN {graph}."Node" sn
                  ON sn.eq_node_id = d.source_id_val
                JOIN {graph}."Node" tn
                  ON tn.eq_node_id = d.target_id_val
                ORDER BY d.source_id_val, d.target_id_val, d.rel_type_val
            ) AS j
            ON CONFLICT (eq_source_id, eq_target_id, eq_rel_type)
                WHERE eq_source_id IS NOT NULL AND eq_target_id IS NOT NULL AND eq_rel_type IS NOT NULL
            DO UPDATE SET
                properties = EXCLUDED.properties,
                eq_source_id = EXCLUDED.eq_source_id,
                eq_target_id = EXCLUDED.eq_target_id,
                eq_rel_type = EXCLUDED.eq_rel_type,
                start_id = EXCLUDED.start_id,
                end_id = EXCLUDED.end_id
            "#,
            graph = graph
        );

        let result = sqlx::query(&sql)
            .bind(&source_ids)
            .bind(&target_ids)
            .bind(&rel_types)
            .bind(&props_json)
            .execute(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("Native SQL edge batch upsert failed: {e}"))
            })?;

        Ok(result.rows_affected())
    }
}
