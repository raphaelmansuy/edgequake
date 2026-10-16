//! Plain upserts, deletes, and the post-AGE mirror copy (S4 / S5).

use std::collections::HashMap;

use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

use super::super::PostgresAGEGraphStorage;
use super::rows::text_prop;
use crate::error::{Result, StorageError};
use crate::graph_batch_dedupe::normalize_rel_type;
use crate::traits::GraphPropertyWriteMode;

fn merge_flag(mode: GraphPropertyWriteMode) -> bool {
    matches!(mode, GraphPropertyWriteMode::MergeSources)
}

#[allow(clippy::module_inception)]
pub(in crate::adapters::postgres::graph) mod write {
    use super::*;

    pub(in crate::adapters::postgres::graph) async fn upsert_nodes(
        tx: &mut Transaction<'_, Postgres>,
        namespace: &str,
        nodes: &[(String, HashMap<String, Value>)],
        mode: GraphPropertyWriteMode,
    ) -> Result<()> {
        if nodes.is_empty() {
            return Ok(());
        }
        let mut ids = Vec::new();
        let mut tenants = Vec::new();
        let mut workspaces = Vec::new();
        let mut types = Vec::new();
        let mut props = Vec::new();
        for (id, map) in nodes {
            let mut full = map.clone();
            full.insert("node_id".into(), Value::String(id.clone()));
            ids.push(id.clone());
            tenants.push(text_prop(&full, "tenant_id"));
            workspaces.push(text_prop(&full, "workspace_id"));
            types.push(text_prop(&full, "entity_type"));
            props.push(serde_json::to_string(&full).unwrap_or_else(|_| "{}".into()));
        }
        sqlx::query(
            "INSERT INTO public.eq_graph_nodes
                (namespace, node_id, tenant_id, workspace_id, entity_type, properties)
             SELECT $1, d.node_id, d.tenant_id, d.workspace_id, NULLIF(d.entity_type, ''), d.props::jsonb
             FROM (
                SELECT DISTINCT ON (node_id) node_id, tenant_id, workspace_id, entity_type, props
                FROM unnest($2::text[], $3::text[], $4::text[], $5::text[], $6::text[])
                     WITH ORDINALITY AS p(node_id, tenant_id, workspace_id, entity_type, props, ord)
                ORDER BY node_id, ord DESC
             ) d
             ON CONFLICT (namespace, node_id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                workspace_id = EXCLUDED.workspace_id,
                entity_type = EXCLUDED.entity_type,
                properties = CASE
                    WHEN $7 THEN public.eq_merge_graph_properties(
                        public.eq_graph_nodes.properties, EXCLUDED.properties)
                    ELSE EXCLUDED.properties
                END",
        )
        .bind(namespace)
        .bind(&ids)
        .bind(&tenants)
        .bind(&workspaces)
        .bind(&types)
        .bind(&props)
        .bind(merge_flag(mode))
        .execute(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("plain node upsert failed: {e}")))?;
        Ok(())
    }

    pub(in crate::adapters::postgres::graph) async fn upsert_edges(
        tx: &mut Transaction<'_, Postgres>,
        namespace: &str,
        edges: &[(String, String, HashMap<String, Value>)],
        mode: GraphPropertyWriteMode,
    ) -> Result<u64> {
        if edges.is_empty() {
            return Ok(0);
        }
        let mut sources = Vec::new();
        let mut targets = Vec::new();
        let mut rels = Vec::new();
        let mut tenants = Vec::new();
        let mut workspaces = Vec::new();
        let mut weights: Vec<Option<f64>> = Vec::new();
        let mut props = Vec::new();
        for (src, tgt, map) in edges {
            let rel = normalize_rel_type(map);
            let mut full = map.clone();
            full.insert("source_id".into(), Value::String(src.clone()));
            full.insert("target_id".into(), Value::String(tgt.clone()));
            full.insert("relation_type".into(), Value::String(rel.clone()));
            sources.push(src.clone());
            targets.push(tgt.clone());
            rels.push(rel);
            tenants.push(text_prop(&full, "tenant_id"));
            workspaces.push(text_prop(&full, "workspace_id"));
            weights.push(full.get("weight").and_then(|v| v.as_f64()).or_else(|| {
                full.get("weight")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse().ok())
            }));
            props.push(serde_json::to_string(&full).unwrap_or_else(|_| "{}".into()));
        }
        let result = sqlx::query(
            "INSERT INTO public.eq_graph_edges
                (namespace, source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
             SELECT $1, d.source_id, d.target_id, d.rel_type, d.tenant_id, d.workspace_id, d.weight, d.props::jsonb
             FROM (
                SELECT DISTINCT ON (source_id, target_id, rel_type)
                    source_id, target_id, rel_type, tenant_id, workspace_id, weight, props
                FROM unnest($2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::float8[], $8::text[])
                     WITH ORDINALITY AS p(source_id, target_id, rel_type, tenant_id, workspace_id, weight, props, ord)
                ORDER BY source_id, target_id, rel_type, ord DESC
             ) d
             ON CONFLICT (namespace, source_id, target_id, rel_type) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                workspace_id = EXCLUDED.workspace_id,
                weight = EXCLUDED.weight,
                properties = CASE
                    WHEN $9 THEN public.eq_merge_graph_properties(
                        public.eq_graph_edges.properties, EXCLUDED.properties)
                    ELSE EXCLUDED.properties
                END",
        )
        .bind(namespace)
        .bind(&sources)
        .bind(&targets)
        .bind(&rels)
        .bind(&tenants)
        .bind(&workspaces)
        .bind(&weights)
        .bind(&props)
        .bind(merge_flag(mode))
        .execute(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("plain edge upsert failed: {e}")))?;
        Ok(result.rows_affected())
    }

    /// Copy the post-merge AGE rows. Replace, do not union again (S4).
    pub(in crate::adapters::postgres::graph) async fn mirror_nodes_from_age(
        tx: &mut Transaction<'_, Postgres>,
        graph: &str,
        namespace: &str,
        node_ids: &[String],
    ) -> Result<()> {
        if !safe_ident(graph) || node_ids.is_empty() {
            return Ok(());
        }
        let sql = format!(
            r#"INSERT INTO public.eq_graph_nodes
                 (namespace, node_id, tenant_id, workspace_id, entity_type, properties)
               SELECT $1,
                      n.eq_node_id,
                      COALESCE(ag_catalog.agtype_to_json(n.properties)->>'tenant_id', ''),
                      COALESCE(ag_catalog.agtype_to_json(n.properties)->>'workspace_id', ''),
                      NULLIF(ag_catalog.agtype_to_json(n.properties)->>'entity_type', ''),
                      ag_catalog.agtype_to_json(n.properties)::jsonb
               FROM {graph}."Node" n
               WHERE n.eq_node_id = ANY($2::text[])
               ON CONFLICT (namespace, node_id) DO UPDATE SET
                 tenant_id = EXCLUDED.tenant_id,
                 workspace_id = EXCLUDED.workspace_id,
                 entity_type = EXCLUDED.entity_type,
                 properties = EXCLUDED.properties"#
        );
        sqlx::query(&sql)
            .bind(namespace)
            .bind(node_ids)
            .execute(&mut **tx)
            .await
            .map_err(|e| StorageError::Database(format!("plain mirror nodes failed: {e}")))?;
        Ok(())
    }

    pub(in crate::adapters::postgres::graph) async fn mirror_edges_from_age(
        tx: &mut Transaction<'_, Postgres>,
        graph: &str,
        namespace: &str,
        sources: &[String],
        targets: &[String],
        rels: &[String],
    ) -> Result<()> {
        if !safe_ident(graph) || sources.is_empty() {
            return Ok(());
        }
        let sql = format!(
            r#"INSERT INTO public.eq_graph_edges
                 (namespace, source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
               SELECT $1,
                      e.eq_source_id,
                      e.eq_target_id,
                      COALESCE(e.eq_rel_type, ''),
                      COALESCE(ag_catalog.agtype_to_json(e.properties)->>'tenant_id', ''),
                      COALESCE(ag_catalog.agtype_to_json(e.properties)->>'workspace_id', ''),
                      CASE
                        WHEN COALESCE(ag_catalog.agtype_to_json(e.properties)->>'weight', '')
                             ~ '^-?[0-9]+(\.[0-9]+)?$'
                        THEN (ag_catalog.agtype_to_json(e.properties)->>'weight')::float8
                        ELSE NULL
                      END,
                      ag_catalog.agtype_to_json(e.properties)::jsonb
               FROM {graph}."EDGE" e
               JOIN unnest($2::text[], $3::text[], $4::text[]) AS k(src, tgt, rel)
                 ON e.eq_source_id = k.src AND e.eq_target_id = k.tgt AND e.eq_rel_type = k.rel
               ON CONFLICT (namespace, source_id, target_id, rel_type) DO UPDATE SET
                 tenant_id = EXCLUDED.tenant_id,
                 workspace_id = EXCLUDED.workspace_id,
                 weight = EXCLUDED.weight,
                 properties = EXCLUDED.properties"#
        );
        sqlx::query(&sql)
            .bind(namespace)
            .bind(sources)
            .bind(targets)
            .bind(rels)
            .execute(&mut **tx)
            .await
            .map_err(|e| StorageError::Database(format!("plain mirror edges failed: {e}")))?;
        Ok(())
    }

    pub(in crate::adapters::postgres::graph) async fn delete_nodes(
        tx: &mut Transaction<'_, Postgres>,
        namespace: &str,
        node_ids: &[String],
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM public.eq_graph_edges
             WHERE namespace = $1 AND (source_id = ANY($2::text[]) OR target_id = ANY($2::text[]))",
        )
        .bind(namespace)
        .bind(node_ids)
        .execute(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("plain detach edges failed: {e}")))?;
        sqlx::query(
            "DELETE FROM public.eq_graph_nodes WHERE namespace = $1 AND node_id = ANY($2::text[])",
        )
        .bind(namespace)
        .bind(node_ids)
        .execute(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("plain delete nodes failed: {e}")))?;
        Ok(())
    }

    pub(in crate::adapters::postgres::graph) async fn delete_edges(
        tx: &mut Transaction<'_, Postgres>,
        namespace: &str,
        sources: &[String],
        targets: &[String],
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM public.eq_graph_edges e
             USING unnest($2::text[], $3::text[]) AS k(src, tgt)
             WHERE e.namespace = $1 AND e.source_id = k.src AND e.target_id = k.tgt",
        )
        .bind(namespace)
        .bind(sources)
        .bind(targets)
        .execute(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("plain delete edges failed: {e}")))?;
        Ok(())
    }

    /// Delete one multigraph sister per triple. `rel_type` uses the same
    /// `UPPER(TRIM)` arbiter as the AGE edge trigger. Optional tenant and
    /// workspace fences compare the plain columns. Degree triggers stay on.
    pub(in crate::adapters::postgres::graph) async fn delete_edges_by_rel(
        tx: &mut Transaction<'_, Postgres>,
        namespace: &str,
        sources: &[String],
        targets: &[String],
        rels: &[String],
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<u64> {
        if sources.is_empty() {
            return Ok(0);
        }
        let row_rel = crate::graph_batch_dedupe::sql_eq_rel_type_arbiter_expr("e.rel_type");
        let pair_rel = crate::graph_batch_dedupe::sql_eq_rel_type_arbiter_expr("k.rel");
        let sql = format!(
            "DELETE FROM public.eq_graph_edges e
             USING unnest($2::text[], $3::text[], $4::text[]) AS k(src, tgt, rel)
             WHERE e.namespace = $1
               AND e.source_id = k.src
               AND e.target_id = k.tgt
               AND {row_rel} = {pair_rel}
               AND ($5::text IS NULL OR e.tenant_id = $5)
               AND ($6::text IS NULL OR e.workspace_id = $6)"
        );
        let result = sqlx::query(&sql)
            .bind(namespace)
            .bind(sources)
            .bind(targets)
            .bind(rels)
            .bind(tenant_id)
            .bind(workspace_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| {
                StorageError::Database(format!("plain delete edges by rel failed: {e}"))
            })?;
        Ok(result.rows_affected())
    }

    /// Delete every relationship between one pair, optionally inside a tenant fence.
    pub(in crate::adapters::postgres::graph) async fn delete_edges_between(
        tx: &mut Transaction<'_, Postgres>,
        namespace: &str,
        source: &str,
        target: &str,
        tenant_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM public.eq_graph_edges
             WHERE namespace = $1 AND source_id = $2 AND target_id = $3
               AND ($4::text IS NULL OR tenant_id = $4)
               AND ($5::text IS NULL OR workspace_id = $5)",
        )
        .bind(namespace)
        .bind(source)
        .bind(target)
        .bind(tenant_id)
        .bind(workspace_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("plain delete edges between failed: {e}")))?;
        Ok(result.rows_affected())
    }

    /// Tenant-scoped node delete. Incident plain edges of the doomed nodes are detached.
    pub(in crate::adapters::postgres::graph) async fn delete_nodes_scoped(
        tx: &mut Transaction<'_, Postgres>,
        namespace: &str,
        node_ids: &[String],
        tenant_id: &str,
        workspace_id: &str,
    ) -> Result<u64> {
        if node_ids.is_empty() {
            return Ok(0);
        }
        let rows = sqlx::query(
            "WITH doomed AS (
               SELECT node_id FROM public.eq_graph_nodes
               WHERE namespace = $1 AND node_id = ANY($2::text[])
                 AND tenant_id = $3 AND workspace_id = $4
             ),
             detached AS (
               DELETE FROM public.eq_graph_edges e
               WHERE e.namespace = $1
                 AND (
                   e.source_id IN (SELECT node_id FROM doomed)
                   OR e.target_id IN (SELECT node_id FROM doomed)
                 )
               RETURNING 1
             )
             DELETE FROM public.eq_graph_nodes n
             WHERE n.namespace = $1
               AND n.node_id IN (SELECT node_id FROM doomed)
             RETURNING 1",
        )
        .bind(namespace)
        .bind(node_ids)
        .bind(tenant_id)
        .bind(workspace_id)
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| StorageError::Database(format!("plain scoped node delete failed: {e}")))?;
        Ok(rows.len() as u64)
    }

    pub(in crate::adapters::postgres::graph) async fn clear_namespace(
        pool: &PgPool,
        namespace: &str,
    ) -> Result<()> {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(format!("plain clear begin failed: {e}")))?;
        // The namespace's nodes are deleted next, so per-edge degree updates
        // would rewrite rows that are about to disappear.
        sqlx::query("SELECT set_config('eq.skip_graph_degree', 'on', true)")
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("plain clear skip degree failed: {e}")))?;
        sqlx::query("DELETE FROM public.eq_graph_edges WHERE namespace = $1")
            .bind(namespace)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("plain clear edges failed: {e}")))?;
        sqlx::query("DELETE FROM public.eq_graph_nodes WHERE namespace = $1")
            .bind(namespace)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(format!("plain clear nodes failed: {e}")))?;
        tx.commit()
            .await
            .map_err(|e| StorageError::Database(format!("plain clear commit failed: {e}")))?;
        Ok(())
    }

    pub(in crate::adapters::postgres::graph) async fn clear_workspace(
        storage: &PostgresAGEGraphStorage,
        workspace_id: &str,
    ) -> Result<(usize, usize)> {
        let pool = storage.pool.get().await?;
        let node_ids: Vec<String> = sqlx::query_scalar(
            "SELECT node_id FROM public.eq_graph_nodes WHERE namespace = $1 AND workspace_id = $2",
        )
        .bind(&storage.namespace)
        .bind(workspace_id)
        .fetch_all(&pool)
        .await
        .map_err(|e| StorageError::Database(format!("plain clear_workspace list failed: {e}")))?;
        if node_ids.is_empty() {
            return Ok((0, 0));
        }
        let edges: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM public.eq_graph_edges
             WHERE namespace = $1 AND (source_id = ANY($2::text[]) OR target_id = ANY($2::text[]))",
        )
        .bind(&storage.namespace)
        .bind(&node_ids)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);
        let mut tx = pool.begin().await.map_err(|e| {
            StorageError::Database(format!("plain clear_workspace begin failed: {e}"))
        })?;
        delete_nodes(&mut tx, &storage.namespace, &node_ids).await?;
        tx.commit().await.map_err(|e| {
            StorageError::Database(format!("plain clear_workspace commit failed: {e}"))
        })?;
        Ok((node_ids.len(), edges.max(0) as usize))
    }
}

fn safe_ident(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
