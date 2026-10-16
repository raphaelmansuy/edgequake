//! List, keyset, and lineage scans (Q-GRAPH-017..023).

use sqlx::Row;

use super::super::helpers::{lineage_discovery_singular_sql, lineage_discovery_sql};
use super::super::PostgresAGEGraphStorage;
use super::rows::{edge_from_row, lit, node_from_row, tenant_and, TenantMode};
use crate::error::{Result, StorageError};
use crate::traits::{
    EdgeListFilter, GraphEdge, GraphNode, KeysetPage, NodeListFilter, PagedGraphResult,
};

fn node_filter_sql(filter: &NodeListFilter) -> String {
    let mut sql = tenant_and(
        "n",
        filter.tenant_id.as_deref(),
        filter.workspace_id.as_deref(),
        TenantMode::Strict,
    );
    if let Some(etype) = filter.entity_type.as_deref() {
        // `upper(entity_type)` matches idx_eq_graph_nodes_type_upper. Wrapping
        // the column in COALESCE makes that index unusable.
        sql.push_str(&format!(
            " AND n.entity_type IS NOT NULL AND upper(n.entity_type) = upper('{}')",
            lit(etype)
        ));
    }
    if let Some(search) = filter.search.as_deref() {
        let q = lit(&search.to_lowercase());
        sql.push_str(&format!(
            " AND (LOWER(n.node_id) LIKE '%{q}%' \
             OR LOWER(COALESCE(n.properties->>'description', '')) LIKE '%{q}%')"
        ));
    }
    if let Some(ids) = filter.community_ids.as_ref() {
        if ids.is_empty() {
            sql.push_str(" AND FALSE");
        } else {
            let list = ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            sql.push_str(&format!(
                " AND (n.properties->>'community_id')::bigint IN ({list})"
            ));
        }
    }
    sql
}

fn edge_filter_sql(filter: &EdgeListFilter) -> String {
    let mut sql = tenant_and(
        "e",
        filter.tenant_id.as_deref(),
        filter.workspace_id.as_deref(),
        TenantMode::Strict,
    );
    if let Some(rel) = filter.relationship_type.as_deref() {
        sql.push_str(&format!(" AND e.rel_type = '{}'", lit(rel)));
    }
    sql
}

#[allow(clippy::module_inception)]
pub(in crate::adapters::postgres::graph) mod scan {
    use super::*;

    pub(in crate::adapters::postgres::graph) async fn list_nodes(
        storage: &PostgresAGEGraphStorage,
        filter: &NodeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphNode>> {
        let extra = node_filter_sql(filter);
        let pool = storage.pool.get().await?;
        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*)::bigint FROM public.eq_graph_nodes n WHERE n.namespace = $1 {extra}"
        ))
        .bind(&storage.namespace)
        .fetch_one(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain list_nodes count failed: {e}")))?;
        let rows = sqlx::query(&format!(
            "SELECT node_id, properties FROM public.eq_graph_nodes n
             WHERE n.namespace = $1 {extra}
             ORDER BY n.node_id
             OFFSET $2 LIMIT $3"
        ))
        .bind(&storage.namespace)
        .bind(offset as i64)
        .bind(limit as i64)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain list_nodes failed: {e}")))?;
        Ok(PagedGraphResult {
            items: rows.iter().filter_map(node_from_row).collect(),
            total: total.max(0) as usize,
            offset,
            limit,
        })
    }

    pub(in crate::adapters::postgres::graph) async fn list_edges(
        storage: &PostgresAGEGraphStorage,
        filter: &EdgeListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<PagedGraphResult<GraphEdge>> {
        let extra = edge_filter_sql(filter);
        let pool = storage.pool.get().await?;
        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*)::bigint FROM public.eq_graph_edges e WHERE e.namespace = $1 {extra}"
        ))
        .bind(&storage.namespace)
        .fetch_one(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain list_edges count failed: {e}")))?;
        let rows = sqlx::query(&format!(
            "SELECT source_id, target_id, properties FROM public.eq_graph_edges e
             WHERE e.namespace = $1 {extra}
             ORDER BY e.source_id, e.target_id, e.rel_type
             OFFSET $2 LIMIT $3"
        ))
        .bind(&storage.namespace)
        .bind(offset as i64)
        .bind(limit as i64)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain list_edges failed: {e}")))?;
        Ok(PagedGraphResult {
            items: rows.iter().filter_map(edge_from_row).collect(),
            total: total.max(0) as usize,
            offset,
            limit,
        })
    }

    pub(in crate::adapters::postgres::graph) async fn scan_nodes_after(
        storage: &PostgresAGEGraphStorage,
        filter: &NodeListFilter,
        after: Option<&str>,
        limit: usize,
    ) -> Result<KeysetPage<GraphNode>> {
        let limit = limit.max(1);
        let extra = node_filter_sql(filter);
        let after_sql = if after.is_some() {
            " AND n.node_id > $2"
        } else {
            ""
        };
        let pool = storage.pool.get().await?;
        let sql = format!(
            "SELECT node_id, properties FROM public.eq_graph_nodes n
             WHERE n.namespace = $1 {extra} {after_sql}
             ORDER BY n.node_id
             LIMIT {limit}"
        );
        let mut q = sqlx::query(&sql).bind(&storage.namespace);
        if let Some(cursor) = after {
            q = q.bind(cursor);
        }
        let rows = q
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain scan_nodes_after failed: {e}")))?;
        let items: Vec<GraphNode> = rows.iter().filter_map(node_from_row).collect();
        let next_after = if items.len() >= limit {
            items.last().map(|n| n.id.clone())
        } else {
            None
        };
        Ok(KeysetPage { items, next_after })
    }

    pub(in crate::adapters::postgres::graph) async fn scan_edges_after(
        storage: &PostgresAGEGraphStorage,
        filter: &EdgeListFilter,
        after: Option<&str>,
        limit: usize,
    ) -> Result<KeysetPage<GraphEdge>> {
        let limit = limit.max(1);
        let extra = edge_filter_sql(filter);
        // Row comparison uses the primary key. Comparing the tab-joined cursor
        // as one string filters index entries from the start of the namespace
        // (hundreds of thousands of rows when the cursor is late). split_part
        // is an InitPlan, so the bounds are constants. Tab is smaller than any
        // id character, so the cursor order matches tuple order under C collation.
        let after_sql = if after.is_some() {
            " AND (e.source_id, e.target_id, e.rel_type) > (
                 (SELECT split_part($2, chr(9), 1)),
                 (SELECT split_part($2, chr(9), 2)),
                 (SELECT split_part($2, chr(9), 3))
              )"
        } else {
            ""
        };
        let pool = storage.pool.get().await?;
        let sql = format!(
            "SELECT source_id, target_id, rel_type, properties,
                    (source_id || chr(9) || target_id || chr(9) || rel_type) AS cursor_id
             FROM public.eq_graph_edges e
             WHERE e.namespace = $1 {extra} {after_sql}
             ORDER BY e.source_id, e.target_id, e.rel_type
             LIMIT {limit}"
        );
        let mut q = sqlx::query(&sql).bind(&storage.namespace);
        if let Some(cursor) = after {
            q = q.bind(cursor);
        }
        let rows = q
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain scan_edges_after failed: {e}")))?;
        let mut items = Vec::new();
        let mut last = None;
        for row in &rows {
            last = row.try_get::<String, _>("cursor_id").ok();
            if let Some(edge) = edge_from_row(row) {
                items.push(edge);
            }
        }
        let next_after = if rows.len() >= limit { last } else { None };
        Ok(KeysetPage { items, next_after })
    }

    pub(in crate::adapters::postgres::graph) async fn find_nodes_by_source_prefixes(
        storage: &PostgresAGEGraphStorage,
        filter: &NodeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphNode>> {
        lineage_nodes(storage, filter, source_prefixes).await
    }

    pub(in crate::adapters::postgres::graph) async fn find_edges_by_source_prefixes(
        storage: &PostgresAGEGraphStorage,
        filter: &EdgeListFilter,
        source_prefixes: &[String],
    ) -> Result<Vec<GraphEdge>> {
        if source_prefixes.is_empty() {
            return Ok(Vec::new());
        }
        let (exact, chunks) = probe_sets(source_prefixes);
        let extra = edge_filter_sql(filter);
        let sql = lineage_discovery_sql(
            "e.source_id, e.target_id, e.properties",
            "public.eq_graph_edges e",
            "e.properties",
            &format!("e.namespace = $4 {extra}"),
        );
        let pool = storage.pool.get().await?;
        let rows = sqlx::query(&sql)
            .bind(&exact)
            .bind(&chunks)
            .bind(super::super::super::helpers::SOURCE_CHUNK_PROBE_LIMIT as i32)
            .bind(&storage.namespace)
            .fetch_all(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("plain edge lineage failed: {e}")))?;
        let singular_sql = lineage_discovery_singular_sql(
            "e.source_id, e.target_id, e.properties",
            "public.eq_graph_edges e",
            "e.properties",
            &format!("e.namespace = $4 {extra}"),
        );
        let singular_rows = sqlx::query(&singular_sql)
            .bind(&exact)
            .bind(&chunks)
            .bind(super::super::super::helpers::SOURCE_CHUNK_PROBE_LIMIT as i32)
            .bind(&storage.namespace)
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!("plain singular edge lineage failed: {e}"))
            })?;
        let mut by_key: std::collections::HashMap<(String, String, String), GraphEdge> =
            std::collections::HashMap::new();
        for row in rows.iter().chain(singular_rows.iter()) {
            if let Some(edge) = edge_from_row(row) {
                let rel = crate::graph_batch_dedupe::normalize_rel_type(&edge.properties);
                by_key.insert((edge.source.clone(), edge.target.clone(), rel), edge);
            }
        }
        let mut out: Vec<GraphEdge> = by_key.into_values().collect();
        out.sort_by(|a, b| {
            let ar = crate::graph_batch_dedupe::normalize_rel_type(&a.properties);
            let br = crate::graph_batch_dedupe::normalize_rel_type(&b.properties);
            (&a.source, &a.target, ar).cmp(&(&b.source, &b.target, br))
        });
        Ok(out)
    }

    pub(in crate::adapters::postgres::graph) async fn find_edge_by_relationship_id(
        storage: &PostgresAGEGraphStorage,
        filter: &EdgeListFilter,
        relationship_id: &str,
    ) -> Result<Option<GraphEdge>> {
        if relationship_id.is_empty() {
            return Ok(None);
        }
        let extra = edge_filter_sql(filter);
        let id = lit(relationship_id);
        let pool = storage.pool.get().await?;
        let row = sqlx::query(&format!(
            "SELECT source_id, target_id, properties
             FROM public.eq_graph_edges e
             WHERE e.namespace = $1 {extra}
               AND (e.properties->>'id' = '{id}'
                    OR (e.source_id || '_' || e.target_id) = '{id}')
             LIMIT 1"
        ))
        .bind(&storage.namespace)
        .fetch_optional(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain relationship id lookup failed: {e}")))?;
        Ok(row.as_ref().and_then(edge_from_row))
    }
}

async fn lineage_nodes(
    storage: &PostgresAGEGraphStorage,
    filter: &NodeListFilter,
    source_prefixes: &[String],
) -> Result<Vec<GraphNode>> {
    if source_prefixes.is_empty() {
        return Ok(Vec::new());
    }
    let (exact, chunks) = probe_sets(source_prefixes);
    let extra = node_filter_sql(filter);
    let sql = lineage_discovery_sql(
        "n.node_id, n.properties",
        "public.eq_graph_nodes n",
        "n.properties",
        &format!("n.namespace = $4 {extra}"),
    );
    let pool = storage.pool.get().await?;
    let rows = sqlx::query(&sql)
        .bind(&exact)
        .bind(&chunks)
        .bind(super::super::helpers::SOURCE_CHUNK_PROBE_LIMIT as i32)
        .bind(&storage.namespace)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain node lineage failed: {e}")))?;
    Ok(rows.iter().filter_map(node_from_row).collect())
}

fn probe_sets(source_prefixes: &[String]) -> (Vec<String>, Vec<String>) {
    let mut exact: Vec<String> = source_prefixes
        .iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let mut chunks: Vec<String> = exact
        .iter()
        .map(|p| super::super::helpers::normalize_doc_chunk_prefix(p))
        .collect();
    exact.sort();
    exact.dedup();
    chunks.sort();
    chunks.dedup();
    (exact, chunks)
}
