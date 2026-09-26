//! Bounded graph scan — SPEC-006 postgres push-down.

use super::helpers::{EdgeTenantFilterMode, VertexTenantFilterMode};
use super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{EdgeListFilter, GraphEdge, GraphNode, NodeListFilter, PagedGraphResult};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// SPEC-089 Phase 4 e2e hook: how many source-prefix discovery SQL calls ran.
///
/// Incremented once per `pg_find_nodes_by_source_prefixes` /
/// `pg_find_edges_by_source_prefixes` entry (F-336-12 amp proof).
pub static SOURCE_PREFIX_DISCOVERY_CALLS: AtomicU64 = AtomicU64::new(0);

impl PostgresAGEGraphStorage {
    fn build_node_where_clause(filter: &NodeListFilter) -> String {
        Self::build_vertex_property_where("v", filter)
    }

    fn build_node_where_clause_for_discovery(filter: &NodeListFilter) -> String {
        Self::build_vertex_property_where_mode(
            "v",
            filter,
            VertexTenantFilterMode::LegacyNullAsWildcard,
        )
    }

    fn build_edge_where_clause(filter: &EdgeListFilter) -> String {
        Self::build_edge_property_where("e", filter, EdgeTenantFilterMode::Strict)
    }

    fn build_edge_where_clause_for_discovery(filter: &EdgeListFilter) -> String {
        Self::build_edge_property_where("e", filter, EdgeTenantFilterMode::LegacyNullAsWildcard)
    }

    pub(super) async fn pg_list_nodes_filtered(
        &self,
        filter: &NodeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphNode>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let where_clause = Self::build_node_where_clause(filter);

        let count_sql = format!(
            "SELECT COUNT(*)::BIGINT AS total
             FROM {graph}.\"_ag_label_vertex\" v
             WHERE {where_clause}",
            graph = self.graph_name,
            where_clause = where_clause
        );

        let total: i64 = sqlx::query_scalar(&count_sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Node count query failed: {}", e)))?;

        let page_sql = format!(
            "SELECT ag_catalog.agtype_to_json(v.properties) AS props
             FROM {graph}.\"_ag_label_vertex\" v
             WHERE {where_clause}
             ORDER BY ag_catalog.agtype_to_json(v.properties)->>'node_id'
             OFFSET {offset} LIMIT {limit}",
            graph = self.graph_name,
            where_clause = where_clause,
            offset = offset,
            limit = limit
        );

        let rows = sqlx::query(&page_sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Node list query failed: {}", e)))?;

        let items: Vec<GraphNode> = rows
            .iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let node_id = props.get("node_id")?.as_str()?.to_string();
                let properties = props.as_object()?.clone().into_iter().collect();
                Some(GraphNode {
                    id: node_id,
                    properties,
                })
            })
            .collect();

        Ok(PagedGraphResult {
            items,
            total: total as usize,
            offset,
            limit,
        })
    }

    pub(super) async fn pg_list_edges_filtered(
        &self,
        filter: &EdgeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphEdge>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let where_clause = Self::build_edge_where_clause(filter);

        let count_sql = format!(
            "SELECT COUNT(*)::BIGINT AS total
             FROM {graph}.\"_ag_label_edge\" e
             WHERE {where_clause}",
            graph = self.graph_name,
            where_clause = where_clause
        );

        let total: i64 = sqlx::query_scalar(&count_sql)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Edge count query failed: {}", e)))?;

        let page_sql = format!(
            "SELECT
                ag_catalog.agtype_to_json(e.properties) AS props,
                ag_catalog.agtype_to_json(sv.properties)->>'node_id' AS source_id,
                ag_catalog.agtype_to_json(tv.properties)->>'node_id' AS target_id
             FROM {graph}.\"_ag_label_edge\" e
             JOIN {graph}.\"_ag_label_vertex\" sv ON e.start_id::text = sv.id::text
             JOIN {graph}.\"_ag_label_vertex\" tv ON e.end_id::text = tv.id::text
             WHERE {where_clause}
             ORDER BY source_id, target_id
             OFFSET {offset} LIMIT {limit}",
            graph = self.graph_name,
            where_clause = where_clause,
            offset = offset,
            limit = limit
        );

        let rows = sqlx::query(&page_sql)
            .fetch_all(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Edge list query failed: {}", e)))?;

        let items: Vec<GraphEdge> = rows
            .iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let source: String = row.get("source_id");
                let target: String = row.get("target_id");
                let properties = props.as_object()?.clone().into_iter().collect();
                Some(GraphEdge {
                    source,
                    target,
                    properties,
                })
            })
            .collect();

        Ok(PagedGraphResult {
            items,
            total: total as usize,
            offset,
            limit,
        })
    }

    fn build_source_prefix_clause_legacy(props_expr: &str, source_prefixes: &[String]) -> String {
        let props = format!("({props_expr})::jsonb");
        let mut conditions = Vec::new();
        for prefix in source_prefixes {
            conditions.push(super::helpers::jsonb_matches_doc_source_prefix_legacy(
                &props, prefix,
            ));
        }
        if conditions.is_empty() {
            "FALSE".to_string()
        } else {
            conditions.join(" OR ")
        }
    }

    /// SPEC-071: legacy LIKE / `source_chunk_ids` path — opt-in only.
    /// Default off: modern GIN on child tables is the request-path SSOT.
    fn source_prefix_legacy_enabled() -> bool {
        match std::env::var("EDGEQUAKE_SOURCE_PREFIX_LEGACY") {
            Ok(v) => {
                let v = v.trim().to_ascii_lowercase();
                matches!(v.as_str(), "1" | "true" | "on" | "yes")
            }
            Err(_) => false,
        }
    }

    /// Deduplicate exact ids + normalized `{doc}-chunk-` prefixes for GIN probes.
    fn source_prefix_probe_sets(source_prefixes: &[String]) -> (Vec<String>, Vec<String>) {
        let mut exact: Vec<String> = source_prefixes
            .iter()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        let mut chunk_prefixes: Vec<String> = exact
            .iter()
            .map(|p| super::helpers::normalize_doc_chunk_prefix(p))
            .collect();
        exact.sort();
        exact.dedup();
        chunk_prefixes.sort();
        chunk_prefixes.dedup();
        (exact, chunk_prefixes)
    }

    pub(super) async fn pg_find_nodes_by_source_prefixes(
        &self,
        filter: &NodeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphNode>> {
        if source_prefixes.is_empty() {
            return Ok(Vec::new());
        }
        SOURCE_PREFIX_DISCOVERY_CALLS.fetch_add(1, Ordering::Relaxed);

        let (exact_ids, chunk_prefixes) = Self::source_prefix_probe_sets(source_prefixes);
        if exact_ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Discovery uses legacy-null workspace match; never require tenant props.
        // Apply tenant filter on the *hits* CTE (alias h), never on the GIN join —
        // otherwise the planner starts from idx_node_tenant_id (~30k rows) and
        // rechecks @> as a Join Filter (4s+ on 200k nodes → statement_timeout).
        let tenant_where_hits = Self::build_vertex_property_where_mode(
            "h",
            filter,
            VertexTenantFilterMode::LegacyNullAsWildcard,
        );
        let tenant_where_v = Self::build_node_where_clause_for_discovery(filter);
        let props_expr = "ag_catalog.agtype_to_json(v.properties)";
        let probe_limit = super::helpers::SOURCE_CHUNK_PROBE_LIMIT as i32;

        // SPEC-071 / IMP-031-08 / SPEC-149: probe-first MATERIALIZED CTEs force
        // Bitmap Index Scan on both source_ids and source_chunk_ids GIN indexes.
        let probes_cte = super::helpers::source_ids_probes_cte_sql();
        let hits_cte = super::helpers::lineage_hits_cte_sql(
            "v.properties",
            &format!(
                r#"FROM probes pr INNER JOIN {graph}."Node" v"#,
                graph = self.graph_name
            ),
            props_expr,
            "",
        );
        let modern_sql = format!(
            r#"
            WITH {probes_cte},
            {hits_cte}
            SELECT ag_catalog.agtype_to_json(h.properties) AS props
            FROM hits h
            WHERE {tenant_where}
            LIMIT 5000
            "#,
            probes_cte = probes_cte.trim(),
            hits_cte = hits_cte.trim(),
            tenant_where = tenant_where_hits,
        );

        // SPEC-089 Wave 3 / F-336-08 / LAW-H2: kill discovery CROSS JOIN probes.
        let timeout_ms = super::helpers::SOURCE_DISCOVERY_STATEMENT_TIMEOUT_MS;
        let legacy_enabled = Self::source_prefix_legacy_enabled();
        let legacy_sql = if legacy_enabled {
            let legacy_where = Self::build_source_prefix_clause_legacy(props_expr, source_prefixes);
            Some(format!(
                "SELECT {props} AS props
                 FROM {graph}.\"Node\" v
                 WHERE {tenant_where} AND ({legacy_where})
                 LIMIT 5000",
                props = props_expr,
                graph = self.graph_name,
                tenant_where = tenant_where_v,
                legacy_where = legacy_where
            ))
        } else {
            None
        };

        let mut timed = super::helpers::LocalTimeoutTx::begin(&mut conn, timeout_ms).await?;
        let mut by_id: HashMap<String, GraphNode> = HashMap::new();
        let modern_rows = match sqlx::query(&modern_sql)
            .bind(&exact_ids)
            .bind(&chunk_prefixes)
            .bind(probe_limit)
            .fetch_all(&mut **timed.as_mut())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = timed.rollback().await;
                return Err(StorageError::Database(format!(
                    "Source-prefix node query failed: {e}"
                )));
            }
        };
        for row in modern_rows {
            let props: serde_json::Value = row.get("props");
            let Some(node_id) = props.get("node_id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(obj) = props.as_object() else {
                continue;
            };
            by_id.entry(node_id.to_string()).or_insert(GraphNode {
                id: node_id.to_string(),
                properties: obj.clone().into_iter().collect(),
            });
        }

        // SPEC-071: legacy SeqScan only when explicitly enabled.
        if let Some(legacy_sql) = legacy_sql {
            let legacy_rows = match sqlx::query(&legacy_sql)
                .fetch_all(&mut **timed.as_mut())
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = timed.rollback().await;
                    return Err(StorageError::Database(format!(
                        "Source-prefix node query failed: {e}"
                    )));
                }
            };
            for row in legacy_rows {
                let props: serde_json::Value = row.get("props");
                let Some(node_id) = props.get("node_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let Some(obj) = props.as_object() else {
                    continue;
                };
                by_id.entry(node_id.to_string()).or_insert(GraphNode {
                    id: node_id.to_string(),
                    properties: obj.clone().into_iter().collect(),
                });
            }
        }
        timed.commit().await?;

        let mut out: Vec<GraphNode> = by_id.into_values().collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    pub(super) async fn pg_find_edges_by_source_prefixes(
        &self,
        filter: &EdgeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphEdge>> {
        if source_prefixes.is_empty() {
            return Ok(Vec::new());
        }
        SOURCE_PREFIX_DISCOVERY_CALLS.fetch_add(1, Ordering::Relaxed);

        let (exact_ids, chunk_prefixes) = Self::source_prefix_probe_sets(source_prefixes);
        if exact_ids.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        // Tenant post-filter on hits CTE (alias h) — same probe-first plan fix as nodes.
        let tenant_where_hits = Self::build_edge_property_where(
            "h",
            filter,
            EdgeTenantFilterMode::LegacyNullAsWildcard,
        );
        let tenant_where_e = Self::build_edge_where_clause_for_discovery(filter);
        let props_expr = "ag_catalog.agtype_to_json(e.properties)";
        let probe_limit = super::helpers::SOURCE_CHUNK_PROBE_LIMIT as i32;

        // SPEC-083 / X-03: never drop non-backfilled edges (eq_* IS NULL).
        // Contract: modern path resolves endpoints via eq_source_id / eq_target_id
        // (coalesce_endpoint) — not AGE parent text-cast JOINs.
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let src_expr = if eq_present {
            super::helpers::coalesce_endpoint("e", "source") // eq_source_id
        } else {
            super::helpers::prop_only_endpoint("e", "source")
        };
        let tgt_expr = if eq_present {
            super::helpers::coalesce_endpoint("e", "target") // eq_target_id
        } else {
            super::helpers::prop_only_endpoint("e", "target")
        };

        // SPEC-071 / IMP-031-08 / SPEC-149: MATERIALIZED probe-first → both GIN indexes.
        let probes_cte = super::helpers::source_ids_probes_cte_sql();
        let edge_extra_where = format!(
            "WHERE {src} IS NOT NULL AND {tgt} IS NOT NULL",
            src = src_expr,
            tgt = tgt_expr
        );
        let hits_cte = super::helpers::lineage_hits_cte_sql(
            &format!(
                "e.properties, {src} AS source_id, {tgt} AS target_id",
                src = src_expr,
                tgt = tgt_expr
            ),
            &format!(
                r#"FROM probes pr INNER JOIN {graph}."EDGE" e"#,
                graph = self.graph_name
            ),
            props_expr,
            &edge_extra_where,
        );
        let modern_sql = format!(
            r#"
            WITH {probes_cte},
            {hits_cte}
            SELECT
                ag_catalog.agtype_to_json(h.properties) AS props,
                h.source_id,
                h.target_id
            FROM hits h
            WHERE {tenant_where}
            LIMIT 5000
            "#,
            probes_cte = probes_cte.trim(),
            hits_cte = hits_cte.trim(),
            tenant_where = tenant_where_hits,
        );

        let timeout_ms = super::helpers::SOURCE_DISCOVERY_STATEMENT_TIMEOUT_MS;
        let legacy_sql = if Self::source_prefix_legacy_enabled() {
            let legacy_where = Self::build_source_prefix_clause_legacy(props_expr, source_prefixes);
            Some(format!(
                "SELECT
                    {props} AS props,
                    {src} AS source_id,
                    {tgt} AS target_id
                 FROM {graph}.\"EDGE\" e
                 WHERE {tenant_where}
                   AND ({legacy_where})
                   AND {src} IS NOT NULL
                   AND {tgt} IS NOT NULL
                 LIMIT 5000",
                props = props_expr,
                graph = self.graph_name,
                tenant_where = tenant_where_e,
                legacy_where = legacy_where,
                src = src_expr,
                tgt = tgt_expr,
            ))
        } else {
            None
        };

        let mut timed = super::helpers::LocalTimeoutTx::begin(&mut conn, timeout_ms).await?;
        // SPEC-098 D-30 / Symptom F: collapse on (src, tgt, rel), not (src, tgt).
        let mut by_key: HashMap<(String, String, String), GraphEdge> = HashMap::new();
        let modern_rows = match sqlx::query(&modern_sql)
            .bind(&exact_ids)
            .bind(&chunk_prefixes)
            .bind(probe_limit)
            .fetch_all(&mut **timed.as_mut())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = timed.rollback().await;
                return Err(StorageError::Database(format!(
                    "Source-prefix edge query failed: {e}"
                )));
            }
        };
        for row in modern_rows {
            Self::insert_discovered_edge(&mut by_key, row);
        }

        if let Some(legacy_sql) = legacy_sql {
            let legacy_rows = match sqlx::query(&legacy_sql)
                .fetch_all(&mut **timed.as_mut())
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let _ = timed.rollback().await;
                    return Err(StorageError::Database(format!(
                        "Source-prefix edge query failed: {e}"
                    )));
                }
            };
            for row in legacy_rows {
                Self::insert_discovered_edge(&mut by_key, row);
            }
        }

        // SPEC-098 Symptom F: poisoned source_ids leave singular source_chunk_id /
        // source_document_id as the only citation. Bounded exact probes (no SeqScan
        // unnest of source_chunk_ids arrays — SPEC-071).
        let singular_sql = format!(
            r#"
            WITH probes AS (
              SELECT unnest($1::text[]) AS probe_id
              UNION ALL
              SELECT pref || gs.i::text
              FROM unnest($2::text[]) AS pref
              CROSS JOIN generate_series(0, $3::int - 1) AS gs(i)
            )
            SELECT
                ag_catalog.agtype_to_json(e.properties) AS props,
                {src} AS source_id,
                {tgt} AS target_id
            FROM {graph}."EDGE" e
            WHERE {tenant_where}
              AND {src} IS NOT NULL
              AND {tgt} IS NOT NULL
              AND (
                -- SPEC-119 / LAW-119-2: btree expression must match (no ::jsonb on ->>).
                -- ::jsonb cast defeats idx_edge_source_chunk_id / idx_edge_source_document_id
                -- (same class as GH-362). Modern GIN path above keeps ::jsonb -> 'source_ids'.
                {props}->>'source_chunk_id' IN (SELECT probe_id FROM probes)
                OR {props}->>'source_document_id' IN (SELECT probe_id FROM probes)
              )
            LIMIT 5000
            "#,
            props = props_expr,
            graph = self.graph_name,
            tenant_where = tenant_where_e,
            src = src_expr,
            tgt = tgt_expr,
        );
        let singular_rows = match sqlx::query(&singular_sql)
            .bind(&exact_ids)
            .bind(&chunk_prefixes)
            .bind(probe_limit)
            .fetch_all(&mut **timed.as_mut())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = timed.rollback().await;
                return Err(StorageError::Database(format!(
                    "Source-prefix singular edge query failed: {e}"
                )));
            }
        };
        for row in singular_rows {
            Self::insert_discovered_edge(&mut by_key, row);
        }

        timed.commit().await?;

        let mut out: Vec<GraphEdge> = by_key.into_values().collect();
        out.sort_by(|a, b| {
            let ar = crate::graph_batch_dedupe::normalize_rel_type(&a.properties);
            let br = crate::graph_batch_dedupe::normalize_rel_type(&b.properties);
            (&a.source, &a.target, ar).cmp(&(&b.source, &b.target, br))
        });
        Ok(out)
    }

    fn insert_discovered_edge(
        by_key: &mut HashMap<(String, String, String), GraphEdge>,
        row: sqlx::postgres::PgRow,
    ) {
        let props: serde_json::Value = row.get("props");
        let source: String = row.get("source_id");
        let target: String = row.get("target_id");
        if source.is_empty() || target.is_empty() {
            return;
        }
        let Some(obj) = props.as_object() else {
            return;
        };
        let properties: HashMap<String, serde_json::Value> = obj.clone().into_iter().collect();
        let rel = crate::graph_batch_dedupe::normalize_rel_type(&properties);
        by_key
            .entry((source.clone(), target.clone(), rel))
            .or_insert(GraphEdge {
                source,
                target,
                properties,
            });
    }

    pub(super) async fn pg_find_edge_by_relationship_id(
        &self,
        filter: &EdgeListFilter,
        relationship_id: &str,
    ) -> Result<Option<GraphEdge>> {
        if relationship_id.is_empty() {
            return Ok(None);
        }

        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let tenant_where = Self::build_edge_where_clause(filter);
        let esc_id = Self::escape_sql_string(relationship_id);
        let props_expr = "ag_catalog.agtype_to_json(e.properties)";

        let sql = format!(
            "SELECT
                {props} AS props,
                ag_catalog.agtype_to_json(sv.properties)->>'node_id' AS source_id,
                ag_catalog.agtype_to_json(tv.properties)->>'node_id' AS target_id
             FROM {graph}.\"_ag_label_edge\" e
             JOIN {graph}.\"_ag_label_vertex\" sv ON e.start_id::text = sv.id::text
             JOIN {graph}.\"_ag_label_vertex\" tv ON e.end_id::text = tv.id::text
             WHERE {tenant_where}
               AND (
                 {props}->>'id' = '{esc_id}'
                 OR CONCAT(
                   ag_catalog.agtype_to_json(sv.properties)->>'node_id',
                   '_',
                   ag_catalog.agtype_to_json(tv.properties)->>'node_id'
                 ) = '{esc_id}'
               )
             LIMIT 1",
            props = props_expr,
            graph = self.graph_name,
            tenant_where = tenant_where,
            esc_id = esc_id
        );

        let row = sqlx::query(&sql)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| StorageError::Database(format!("Relationship id lookup failed: {}", e)))?;

        Ok(row.map(|row| {
            let props: serde_json::Value = row.get("props");
            let source: String = row.get("source_id");
            let target: String = row.get("target_id");
            let properties = props
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect();
            GraphEdge {
                source,
                target,
                properties,
            }
        }))
    }
}

#[cfg(test)]
mod source_prefix_clause_tests {
    use super::PostgresAGEGraphStorage;

    #[test]
    fn source_prefix_legacy_clause_casts_agtype_json_to_jsonb() {
        let prefixes = ["doc-abc".to_string()];
        let props = "ag_catalog.agtype_to_json(v.properties)";
        let legacy = PostgresAGEGraphStorage::build_source_prefix_clause_legacy(props, &prefixes);
        assert!(
            legacy.contains("::jsonb"),
            "jsonb_* functions require jsonb cast: {legacy}"
        );
        assert!(legacy.contains("jsonb_typeof") || legacy.contains("jsonb_array_elements_text"));
        // Modern discovery uses probe JOIN (@>), not the removed giant-OR helper.
        let modern =
            crate::adapters::postgres::graph::helpers::jsonb_matches_doc_source_prefix_modern(
                &format!("({props})::jsonb"),
                "doc-abc",
            );
        assert!(modern.contains("@>") || modern.contains("jsonb_build_array"));
        for key in crate::lineage_canon::INDEXED_LINEAGE_ARRAY_KEYS {
            assert!(
                modern.contains(&format!("->'{key}') @>")),
                "modern clause must GIN-probe indexed lineage key {key}: {modern}"
            );
        }
        assert!(
            !modern.contains("LIKE") && !modern.contains("jsonb_array_elements_text"),
            "modern clause must stay GIN-only on the indexed lineage arrays: {modern}"
        );
    }

    #[test]
    fn source_prefix_legacy_disabled_by_default() {
        // Ensure unset / non-truthy does not enable residual SeqScan path.
        std::env::remove_var("EDGEQUAKE_SOURCE_PREFIX_LEGACY");
        assert!(!PostgresAGEGraphStorage::source_prefix_legacy_enabled());
        std::env::set_var("EDGEQUAKE_SOURCE_PREFIX_LEGACY", "0");
        assert!(!PostgresAGEGraphStorage::source_prefix_legacy_enabled());
        std::env::set_var("EDGEQUAKE_SOURCE_PREFIX_LEGACY", "1");
        assert!(PostgresAGEGraphStorage::source_prefix_legacy_enabled());
        std::env::remove_var("EDGEQUAKE_SOURCE_PREFIX_LEGACY");
    }

    #[test]
    fn source_prefix_probe_sets_dedup_exact_and_chunk_prefix() {
        let (exact, chunks) = PostgresAGEGraphStorage::source_prefix_probe_sets(&[
            "doc-a".to_string(),
            "doc-a".to_string(),
            "doc-a-chunk-".to_string(),
        ]);
        assert_eq!(exact, vec!["doc-a".to_string(), "doc-a-chunk-".to_string()]);
        assert_eq!(chunks, vec!["doc-a-chunk-".to_string()]);
    }

    /// IMP-031-08: source contracts force MATERIALIZED probe-first GIN plan.
    #[test]
    fn source_prefix_discovery_sql_is_probe_first_materialized() {
        let src = include_str!("scan_ops.rs");
        assert!(
            src.contains("probes AS MATERIALIZED")
                && src.contains("hits AS MATERIALIZED")
                && src.contains("IMP-031-08"),
            "source-prefix discovery must use MATERIALIZED probe-first CTEs"
        );
        // Tenant filter must not sit on the GIN join outer scan of Node.
        assert!(
            src.contains("FROM hits h") && src.contains("INNER JOIN {graph}.\"Node\" v"),
            "tenant filter on hits; GIN join on Node from probes"
        );
    }
}
