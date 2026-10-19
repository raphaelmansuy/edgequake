//! Facet counts as `GROUP BY` over the plain graph tables.
//!
//! One workspace of 66,682 nodes / 66,588 edges takes about 100 ms per
//! aggregate as `edgequake_tenant_access`, and the three run concurrently. The
//! previous API path read the first 5,000 nodes by name and tallied them in
//! Rust, which took about 1 s and counted under 8% of that workspace.

use sqlx::Row;

use super::super::PostgresAGEGraphStorage;
use super::scan::{edge_filter_sql, node_filter_sql};
use crate::error::{Result, StorageError};
use crate::traits::graph_facets::{DEFAULT_RELATIONSHIP_TYPE, UNKNOWN_ENTITY_TYPE};
use crate::traits::{EdgeListFilter, GraphFacetCounts, NodeListFilter};

/// `community_id` as the API reads it: a string, or an integer rendered as text.
const COMMUNITY_ID_IS_USABLE: &str = "(jsonb_typeof(n.properties->'community_id') = 'string' \
     OR (jsonb_typeof(n.properties->'community_id') = 'number' \
         AND (n.properties->>'community_id') ~ '^-?[0-9]+$'))";

fn to_counts(rows: Vec<sqlx::postgres::PgRow>) -> Vec<(String, usize)> {
    rows.iter()
        .filter_map(|r| {
            let key: String = r.try_get(0).ok()?;
            let n: i64 = r.try_get(1).ok()?;
            Some((key, n.max(0) as usize))
        })
        .collect()
}

fn db_err(what: &str, e: sqlx::Error) -> StorageError {
    StorageError::Database(format!("plain facet_counts {what} failed: {e}"))
}

pub(in crate::adapters::postgres::graph) async fn facet_counts(
    storage: &PostgresAGEGraphStorage,
    nodes: &NodeListFilter,
    edges: &EdgeListFilter,
) -> Result<GraphFacetCounts> {
    let node_scope = node_filter_sql(nodes);
    let edge_scope = edge_filter_sql(edges);
    let pool = storage.pool.get().await?;

    let entity_types_sql = format!(
        "SELECT COALESCE(n.entity_type, '{UNKNOWN_ENTITY_TYPE}'), COUNT(*)::bigint \
         FROM public.eq_graph_nodes n \
         WHERE n.namespace = $1 {node_scope} GROUP BY 1"
    );
    let communities_sql = format!(
        "SELECT n.properties->>'community_id', COUNT(*)::bigint \
         FROM public.eq_graph_nodes n \
         WHERE n.namespace = $1 {node_scope} AND {COMMUNITY_ID_IS_USABLE} GROUP BY 1"
    );
    let relationship_types_sql = format!(
        "SELECT COALESCE(NULLIF(e.rel_type, ''), '{DEFAULT_RELATIONSHIP_TYPE}'), COUNT(*)::bigint \
         FROM public.eq_graph_edges e \
         WHERE e.namespace = $1 {edge_scope} GROUP BY 1"
    );

    let (entity_types, communities, relationship_types) = tokio::try_join!(
        async {
            sqlx::query(&entity_types_sql)
                .bind(&storage.namespace)
                .fetch_all(&pool)
                .await
                .map_err(|e| db_err("entity types", e))
        },
        async {
            sqlx::query(&communities_sql)
                .bind(&storage.namespace)
                .fetch_all(&pool)
                .await
                .map_err(|e| db_err("communities", e))
        },
        async {
            sqlx::query(&relationship_types_sql)
                .bind(&storage.namespace)
                .fetch_all(&pool)
                .await
                .map_err(|e| db_err("relationship types", e))
        },
    )?;

    Ok(GraphFacetCounts {
        communities: to_counts(communities),
        entity_types: to_counts(entity_types),
        relationship_types: to_counts(relationship_types),
    })
}
