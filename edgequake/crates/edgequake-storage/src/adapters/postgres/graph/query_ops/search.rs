//! Label/node search helpers (SPEC-054 ISP).
//!
//! GH-404 residual: degree/popular paths use `"Node"` + `"EDGE"` with eq_*
//! text endpoints — never `_ag_label_*` parent tables or `start_id::text` joins.

use std::collections::HashMap;

use sqlx::Row;

use super::super::PostgresAGEGraphStorage;
use crate::error::{Result, StorageError};
use crate::traits::{GraphNode, NodeListFilter};

impl PostgresAGEGraphStorage {
    /// Bare entity name for search/display under workspace-scoped `node_id` (032).
    /// Prefer `label` (bare); else strip `{uuid}::` from `node_id`.
    fn sql_vertex_search_text(alias: &str) -> String {
        format!(
            "COALESCE( \
                NULLIF(ag_catalog.agtype_to_json({a}.properties)->>'label', ''), \
                CASE \
                  WHEN ag_catalog.agtype_to_json({a}.properties)->>'node_id' \
                       ~ '^[0-9a-fA-F-]{{36}}::' \
                  THEN split_part( \
                         ag_catalog.agtype_to_json({a}.properties)->>'node_id', '::', 2) \
                  ELSE ag_catalog.agtype_to_json({a}.properties)->>'node_id' \
                END \
             )",
            a = alias
        )
    }

    /// Popular-nodes SQL: child `"Node"` + out-degree via `"EDGE"` eq_* endpoints.
    pub(in crate::adapters::postgres::graph) fn popular_nodes_with_degree_sql(
        graph: &str,
        vertex_where: &str,
        node_id_expr: &str,
        src_rows: &str,
        min_degree: usize,
        limit: usize,
    ) -> String {
        format!(
            "WITH filtered_nodes AS MATERIALIZED ( \
                SELECT {node_id} AS node_id, v.properties \
                FROM {graph}.\"Node\" v \
                {vertex_where} \
            ), \
            edge_counts AS ( \
                SELECT e.node_id, COUNT(*) AS out_degree \
                FROM {src_rows} e \
                INNER JOIN filtered_nodes fn ON e.node_id = fn.node_id \
                GROUP BY 1 \
            ) \
            SELECT \
                ag_catalog.agtype_to_json(fn.properties) AS node_props, \
                COALESCE(ec.out_degree, 0) AS degree \
            FROM filtered_nodes fn \
            LEFT JOIN edge_counts ec ON fn.node_id = ec.node_id \
            WHERE COALESCE(ec.out_degree, 0) >= {min_degree} \
            ORDER BY degree DESC \
            LIMIT {limit}",
            graph = graph,
            vertex_where = vertex_where,
            node_id = node_id_expr,
            src_rows = src_rows,
            min_degree = min_degree,
            limit = limit
        )
    }

    /// Search-nodes SQL: child `"Node"` + total degree via `"EDGE"` eq_* endpoints.
    pub(in crate::adapters::postgres::graph) fn search_nodes_with_degree_sql(
        graph: &str,
        vertex_where: &str,
        node_id_expr: &str,
        src_rows: &str,
        tgt_rows: &str,
        limit: usize,
    ) -> String {
        format!(
            "WITH filtered_nodes AS MATERIALIZED ( \
                SELECT {node_id} AS node_id, ag_catalog.agtype_to_json(v.properties) AS props \
                FROM {graph}.\"Node\" v \
                {vertex_where} \
            ), \
            out_degrees AS ( \
                SELECT e.node_id, COUNT(*) AS out_degree \
                FROM {src_rows} e \
                INNER JOIN filtered_nodes fn ON e.node_id = fn.node_id \
                GROUP BY 1 \
            ), \
            in_degrees AS ( \
                SELECT e.node_id, COUNT(*) AS in_degree \
                FROM {tgt_rows} e \
                INNER JOIN filtered_nodes fn ON e.node_id = fn.node_id \
                GROUP BY 1 \
            ) \
            SELECT \
                fn.props, \
                COALESCE(o.out_degree, 0) + COALESCE(i.in_degree, 0) AS degree \
            FROM filtered_nodes fn \
            LEFT JOIN out_degrees o ON fn.node_id = o.node_id \
            LEFT JOIN in_degrees i ON fn.node_id = i.node_id \
            ORDER BY degree DESC \
            LIMIT {limit}",
            graph = graph,
            vertex_where = vertex_where,
            node_id = node_id_expr,
            src_rows = src_rows,
            tgt_rows = tgt_rows,
            limit = limit
        )
    }

    /// Popular-labels SQL: same child-table degree pattern, project bare label.
    pub(in crate::adapters::postgres::graph) fn popular_labels_sql(
        graph: &str,
        vertex_where: &str,
        node_id_expr: &str,
        src_rows: &str,
        label_expr: &str,
        limit: usize,
    ) -> String {
        format!(
            "WITH filtered_nodes AS MATERIALIZED ( \
                SELECT {node_id} AS node_id, v.properties \
                FROM {graph}.\"Node\" v \
                {vertex_where} \
            ), \
            edge_counts AS ( \
                SELECT e.node_id, COUNT(*) AS out_degree \
                FROM {src_rows} e \
                INNER JOIN filtered_nodes fn ON e.node_id = fn.node_id \
                GROUP BY 1 \
            ) \
            SELECT {label} AS label \
            FROM filtered_nodes fn \
            LEFT JOIN edge_counts ec ON fn.node_id = ec.node_id \
            ORDER BY COALESCE(ec.out_degree, 0) DESC \
            LIMIT {limit}",
            graph = graph,
            vertex_where = vertex_where,
            node_id = node_id_expr,
            src_rows = src_rows,
            label = label_expr,
            limit = limit
        )
    }

    pub(in crate::adapters::postgres::graph) async fn pg_get_popular_labels(
        &self,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let filter = NodeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            ..Default::default()
        };
        let vertex_where = Self::vertex_where_clause("v", &filter);
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let node_id = if eq_present {
            super::super::helpers::coalesce_endpoint("v", "node")
        } else {
            super::super::helpers::prop_only_endpoint("v", "node")
        };
        let src =
            super::super::helpers::degree_endpoint_rows(&self.graph_name, "source", eq_present);
        let label_expr = Self::sql_vertex_search_text("fn");

        let sql = Self::popular_labels_sql(
            &self.graph_name,
            &vertex_where,
            &node_id,
            &src,
            &label_expr,
            limit,
        );

        // SPEC-089 / F-336-15 / LAW-H2: no app timeout on trait path — PG must kill.
        let timeout_ms = super::super::helpers::graph_query_statement_timeout_ms();
        let mut timed = super::super::helpers::LocalTimeoutTx::begin(&mut conn, timeout_ms).await?;
        Self::enforce_graph_read_scope(timed.as_mut(), tenant_id, workspace_id).await?;
        let rows = match sqlx::query(&sql).fetch_all(timed.as_mut()).await {
            Ok(r) => {
                timed.commit().await?;
                r
            }
            Err(e) => {
                let _ = timed.rollback().await;
                return Err(StorageError::Database(format!(
                    "popular labels SQL failed: {}",
                    e
                )));
            }
        };

        Ok(rows
            .iter()
            .filter_map(|row| row.get::<Option<String>, _>("label"))
            .collect())
    }

    /// FAST OPTIMIZED: Search node labels with full-text search and fuzzy matching.
    ///
    /// Uses PostgreSQL's full-text search (ts_vector) and trigram similarity (pg_trgm).
    /// Supports fuzzy matching, ranking by relevance, and handles typos.
    ///
    /// Performance: <100ms for fuzzy search across 10k+ nodes
    pub(in crate::adapters::postgres::graph) async fn pg_search_labels(
        &self,
        query: &str,
        limit: usize,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let tenant_predicate = Self::build_vertex_property_where(
            "v",
            &NodeListFilter {
                tenant_id: tenant_id.map(str::to_string),
                workspace_id: workspace_id.map(str::to_string),
                ..Default::default()
            },
        );
        let tenant_and = if tenant_predicate == "TRUE" {
            String::new()
        } else {
            format!(" AND {tenant_predicate}")
        };

        let escaped_query = Self::escape_sql_string(query);
        tracing::debug!(query = %query, escaped = %escaped_query, "search_labels starting");
        let search_text = Self::sql_vertex_search_text("v");

        // SPEC-089 / F-336-15 / LAW-H2: autocomplete has no tokio budget — PG kill.
        let timeout_ms = super::super::helpers::graph_query_statement_timeout_ms();
        let mut timed = super::super::helpers::LocalTimeoutTx::begin(&mut conn, timeout_ms).await?;
        Self::enforce_graph_read_scope(timed.as_mut(), tenant_id, workspace_id).await?;

        // Try full-text search first (best for word matching).
        // 032: FTS must use bare label — scoped node_id (`{ws}::NAME`) breaks
        // keyword validation / prefix match against natural-language terms.
        // GH-404 residual: scan `"Node"` child (indexed), not parent vertex table.
        let fts_sql = format!(
            "SELECT \
                {search_text} as label, \
                ts_rank( \
                    to_tsvector('english', coalesce({search_text}, '')), \
                    plainto_tsquery('english', '{0}') \
                ) as rank \
             FROM {1}.\"Node\" v \
             WHERE to_tsvector('english', coalesce({search_text}, '')) \
                   @@ plainto_tsquery('english', '{0}'){2} \
             ORDER BY rank DESC \
             LIMIT {3}",
            escaped_query, self.graph_name, tenant_and, limit
        );

        let fts_rows = sqlx::query(&fts_sql).fetch_all(timed.as_mut()).await;

        // If full-text search finds results, return them
        if let Ok(rows) = &fts_rows {
            if !rows.is_empty() {
                let labels: Vec<String> = rows
                    .iter()
                    .filter_map(|row| row.get::<Option<String>, _>("label"))
                    .collect();

                if !labels.is_empty() {
                    timed.commit().await?;
                    return Ok(labels);
                }
            }
        } else {
            // FTS error aborts the local txn — reopen before falling back.
            let _ = timed.rollback().await;
            timed = super::super::helpers::LocalTimeoutTx::begin(&mut conn, timeout_ms).await?;
            Self::enforce_graph_read_scope(timed.as_mut(), tenant_id, workspace_id).await?;
        }

        // pg_trgm can be installed outside ag_catalog (commonly public).
        // Resolve its namespace instead of aborting the transaction on every
        // fuzzy lookup. quote_ident keeps extension schema names safe in SQL.
        let trgm_schema: Option<String> = sqlx::query_scalar(
            "SELECT pg_catalog.quote_ident(n.nspname) FROM pg_catalog.pg_extension e \
             JOIN pg_catalog.pg_namespace n ON n.oid=e.extnamespace WHERE e.extname='pg_trgm'",
        )
        .fetch_optional(timed.as_mut())
        .await?;
        if let Some(schema) = trgm_schema {
            let trgm_sql = format!(
                "SELECT {search_text} AS label, \
                        {schema}.similarity(coalesce({search_text}, ''), $1) AS sim \
                 FROM {graph}.\"Node\" v \
                 WHERE coalesce({search_text}, '') OPERATOR({schema}.%) $1 \
                       {tenant_and} \
                 ORDER BY sim DESC LIMIT $2",
                graph = self.graph_name,
            );
            let trgm_rows = sqlx::query(&trgm_sql)
                .bind(query)
                .bind(limit as i64)
                .fetch_all(timed.as_mut())
                .await;
            if let Ok(rows) = &trgm_rows {
                let labels: Vec<String> = rows
                    .iter()
                    .filter_map(|row| row.get::<Option<String>, _>("label"))
                    .collect();
                if !labels.is_empty() {
                    timed.commit().await?;
                    return Ok(labels);
                }
            } else {
                let _ = timed.rollback().await;
                timed = super::super::helpers::LocalTimeoutTx::begin(&mut conn, timeout_ms).await?;
                Self::enforce_graph_read_scope(timed.as_mut(), tenant_id, workspace_id).await?;
            }
        }

        // Final fallback to simple ILIKE prefix matching (always works)
        let prefix_sql = format!(
            "SELECT {search_text} as label \
             FROM {0}.\"Node\" v \
             WHERE LOWER(coalesce({search_text}, '')) LIKE LOWER('{1}%') \
             {2} \
             ORDER BY {search_text} \
             LIMIT {3}",
            self.graph_name, escaped_query, tenant_and, limit
        );

        let prefix_rows = match sqlx::query(&prefix_sql).fetch_all(timed.as_mut()).await {
            Ok(r) => {
                timed.commit().await?;
                r
            }
            Err(e) => {
                let _ = timed.rollback().await;
                return Err(StorageError::Database(format!(
                    "Search labels query failed: {}",
                    e
                )));
            }
        };

        let labels: Vec<String> = prefix_rows
            .iter()
            .filter_map(|row| row.get::<Option<String>, _>("label"))
            .collect();

        Ok(labels)
    }

    /// Search for nodes with full text matching on label and description.
    ///
    /// Returns nodes with their degree, filtered by tenant/workspace context.
    /// Uses a combination of full-text search and ILIKE for best coverage.
    pub(in crate::adapters::postgres::graph) async fn pg_search_nodes(
        &self,
        query: &str,
        limit: usize,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        tracing::debug!(query = %query, "search_nodes starting");

        let filter = NodeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            entity_type: entity_type.map(str::to_string),
            search: Some(query.to_string()),
            ..Default::default()
        };
        let vertex_where = Self::vertex_where_clause("v", &filter);
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let node_id = if eq_present {
            super::super::helpers::coalesce_endpoint("v", "node")
        } else {
            super::super::helpers::prop_only_endpoint("v", "node")
        };
        let src =
            super::super::helpers::degree_endpoint_rows(&self.graph_name, "source", eq_present);
        let tgt =
            super::super::helpers::degree_endpoint_rows(&self.graph_name, "target", eq_present);

        // GH-404 residual: filter `"Node"` first, degree via `"EDGE"` eq_* — never
        // parent-table `start_id::text` joins (those nested-looped for 22s+).
        let sql = Self::search_nodes_with_degree_sql(
            &self.graph_name,
            &vertex_where,
            &node_id,
            &src,
            &tgt,
            limit,
        );

        tracing::debug!(sql = %sql, "search_nodes SQL");

        // SPEC-089 Wave 3 / F-336-10: match run_timed_graph_query with PG kill.
        let timeout_ms = super::super::helpers::graph_query_statement_timeout_ms();
        let mut timed = super::super::helpers::LocalTimeoutTx::begin(&mut conn, timeout_ms).await?;
        Self::enforce_graph_read_scope(timed.as_mut(), tenant_id, workspace_id).await?;
        let rows = match sqlx::query(&sql).fetch_all(timed.as_mut()).await {
            Ok(r) => {
                timed.commit().await?;
                r
            }
            Err(e) => {
                let _ = timed.rollback().await;
                return Err(StorageError::Database(format!(
                    "Search nodes query failed: {e}"
                )));
            }
        };

        let results: Vec<(GraphNode, usize)> = rows
            .iter()
            .filter_map(|row| {
                let props: serde_json::Value = row.get("props");
                let degree: i64 = row.get("degree");

                // Extract node_id from properties
                let node_id = props.get("node_id")?.as_str()?.to_string();

                let node = GraphNode {
                    id: node_id,
                    properties: props.as_object()?.clone().into_iter().collect(),
                };

                Some((node, degree as usize))
            })
            .collect();

        tracing::debug!(results_count = results.len(), "search_nodes completed");
        Ok(results)
    }

    pub(in crate::adapters::postgres::graph) async fn pg_get_popular_nodes_with_degree(
        &self,
        limit: usize,
        min_degree: Option<usize>,
        entity_type: Option<&str>,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<(GraphNode, usize)>> {
        let pool = self.pool.get().await?;
        let mut conn = pool.acquire().await.map_err(|e| {
            StorageError::Connection(format!("Failed to acquire connection: {}", e))
        })?;

        let filter = NodeListFilter {
            tenant_id: tenant_id.map(str::to_string),
            workspace_id: workspace_id.map(str::to_string),
            entity_type: entity_type.map(str::to_string),
            ..Default::default()
        };
        let vertex_where = Self::vertex_where_clause("v", &filter);
        let min_degree_val = min_degree.unwrap_or(0);
        let eq_present = self.eq_columns_present(&mut conn).await?;
        let node_id = if eq_present {
            super::super::helpers::coalesce_endpoint("v", "node")
        } else {
            super::super::helpers::prop_only_endpoint("v", "node")
        };
        let src =
            super::super::helpers::degree_endpoint_rows(&self.graph_name, "source", eq_present);

        // WHY: Filter `"Node"` first (MATERIALIZED), then hash-join EDGE counts on
        // eq_* text endpoints. Avoids AGE parent `start_id::text` nested loops.
        let sql = Self::popular_nodes_with_degree_sql(
            &self.graph_name,
            &vertex_where,
            &node_id,
            &src,
            min_degree_val,
            limit,
        );

        // SPEC-089 Wave 3 / F-336-10: PG kill aligned with run_timed_graph_query.
        let timeout_ms = super::super::helpers::graph_query_statement_timeout_ms();
        let mut timed = super::super::helpers::LocalTimeoutTx::begin(&mut conn, timeout_ms).await?;
        Self::enforce_graph_read_scope(timed.as_mut(), tenant_id, workspace_id).await?;
        let rows = match sqlx::query(&sql).fetch_all(timed.as_mut()).await {
            Ok(r) => {
                timed.commit().await?;
                r
            }
            Err(e) => {
                let _ = timed.rollback().await;
                return Err(StorageError::Database(format!(
                    "Optimized SQL query failed: {e}"
                )));
            }
        };

        let mut results = Vec::with_capacity(limit);

        for row in rows {
            let json_value: serde_json::Value = row.get("node_props");
            let degree: i64 = row.get("degree");

            // Parse node properties
            if let Ok(properties_map) =
                serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(json_value)
            {
                // Convert Map to HashMap
                let properties: HashMap<String, serde_json::Value> =
                    properties_map.into_iter().collect();

                let node = GraphNode {
                    id: properties
                        .get("node_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    properties,
                };
                results.push((node, degree as usize));
            }
        }

        Ok(results)
    }
}
