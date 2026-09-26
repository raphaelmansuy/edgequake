//! SPEC-149 / Migration 158: orphaned document lineage repair on live Postgres.
//!
//! A row that lists a live document in `source_document_ids` but carries no
//! lineage token for it is invisible to that document's scoped graph. M158
//! adds the bare document id (missing-only, idempotent); discovery and the
//! shared count SQL must then see the row.
//!
//! Soft-skips without DATABASE_URL unless EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1.

#![cfg(feature = "postgres")]

#[path = "support/postgres_test_config.rs"]
mod postgres_test_config;
#[path = "support/projection_fixture.rs"]
mod projection_fixture;

use std::collections::HashMap;

use edgequake_storage::{
    canonical_graph_node_id, node_counts_by_source_prefixes_sql,
    traits::{
        GraphPropertyWriteMode, GraphScanOps, GraphStorage, GraphStorageMutateOps,
        GraphStorageReadOps, NodeListFilter,
    },
    PostgresAGEGraphStorage, PostgresPool,
};
use projection_fixture::setup_scope;
use serde_json::{json, Value};
use uuid::Uuid;

const M158: &str = include_str!("../../../migrations/158_graph_lineage_orphan_document_repair.sql");
const M158_VERIFY: &str = include_str!("../../../migrations/support/158/verify.sql");

struct Scope {
    tenant: Uuid,
    workspace: Uuid,
}

impl Scope {
    fn node(&self, lineage: &[String], documents: &[String]) -> HashMap<String, Value> {
        self.node_in(self.workspace, lineage, documents)
    }

    fn node_in(
        &self,
        workspace: Uuid,
        lineage: &[String],
        documents: &[String],
    ) -> HashMap<String, Value> {
        [
            ("entity_type", json!("TEST")),
            ("tenant_id", json!(self.tenant.to_string())),
            ("workspace_id", json!(workspace.to_string())),
            ("source_ids", json!(lineage)),
            ("source_chunk_ids", json!(lineage)),
            ("source_document_ids", json!(documents)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
    }
}

async fn insert_document(pool: &sqlx::PgPool, id: Uuid, tenant: Uuid, workspace: Uuid) {
    sqlx::query(
        "INSERT INTO documents (id, tenant_id, workspace_id, title, content, status)
         VALUES ($1, $2, $3, 'm158', 'm158 content', 'indexed')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(id)
    .bind(tenant)
    .bind(workspace)
    .execute(pool)
    .await
    .expect("seed document");
}

fn lineage(graph_node: &edgequake_storage::GraphNode, key: &str) -> Vec<String> {
    graph_node
        .properties
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

async fn read_node(graph: &PostgresAGEGraphStorage, id: &str) -> edgequake_storage::GraphNode {
    graph
        .get_node(id)
        .await
        .expect("read node")
        .expect("node exists")
}

#[tokio::test]
async fn migration_158_restores_orphaned_live_document_lineage_missing_only() {
    let Some((config, pool, tenant, workspace)) = setup_scope("spec149_m158").await else {
        return;
    };
    let scope = Scope { tenant, workspace };
    let pg_pool = PostgresPool::from_existing(pool.clone(), config.clone());
    let graph = PostgresAGEGraphStorage::with_pool(pg_pool, config.clone());
    graph.initialize().await.expect("init graph");

    let live = Uuid::new_v4();
    let other = Uuid::new_v4();
    let ghost = Uuid::new_v4();
    let foreign_workspace = Uuid::new_v4();
    insert_document(&pool, live, tenant, workspace).await;
    insert_document(&pool, other, tenant, workspace).await;

    let other_chunk = format!("{other}-chunk-0");
    let live_chunk = format!("{live}-chunk-1");
    let (live_s, other_s, ghost_s) = (live.to_string(), other.to_string(), ghost.to_string());

    let orphan = canonical_graph_node_id(workspace, "M158_ORPHAN");
    let healthy = canonical_graph_node_id(workspace, "M158_HEALTHY");
    let ghost_only = canonical_graph_node_id(workspace, "M158_DELETED_DOC");
    let foreign = canonical_graph_node_id(foreign_workspace, "M158_FOREIGN_WS");
    let seeds = vec![
        (
            orphan.clone(),
            scope.node(
                std::slice::from_ref(&other_chunk),
                &[other_s.clone(), live_s.clone()],
            ),
        ),
        (
            healthy.clone(),
            scope.node(
                std::slice::from_ref(&live_chunk),
                std::slice::from_ref(&live_s),
            ),
        ),
        (
            ghost_only.clone(),
            scope.node(
                std::slice::from_ref(&other_chunk),
                &[other_s.clone(), ghost_s.clone()],
            ),
        ),
        (
            foreign.clone(),
            scope.node_in(
                foreign_workspace,
                std::slice::from_ref(&other_chunk),
                &[other_s.clone(), live_s.clone()],
            ),
        ),
    ];
    graph
        .upsert_nodes_batch_with_mode(&seeds, GraphPropertyWriteMode::Replace)
        .await
        .expect("seed nodes");

    let before: Vec<_> = lineage_snapshot(&graph, &[&healthy, &ghost_only, &foreign]).await;
    for _ in 0..2 {
        sqlx::raw_sql(M158)
            .execute(&pool)
            .await
            .expect("apply M158");
    }
    sqlx::raw_sql(M158_VERIFY)
        .execute(&pool)
        .await
        .expect("M158 verify");

    let repaired = read_node(&graph, &orphan).await;
    let expected = {
        let mut v = vec![other_chunk.clone(), live_s.clone()];
        v.sort();
        v
    };
    assert_eq!(lineage(&repaired, "source_ids"), expected);
    assert_eq!(lineage(&repaired, "source_chunk_ids"), expected);
    assert_eq!(
        lineage_snapshot(&graph, &[&healthy, &ghost_only, &foreign]).await,
        before,
        "healthy, deleted-document and foreign-workspace rows must be untouched"
    );

    let found = graph
        .find_nodes_by_source_prefixes(
            &NodeListFilter {
                tenant_id: Some(tenant.to_string()),
                workspace_id: Some(workspace.to_string()),
                ..Default::default()
            },
            std::slice::from_ref(&live_s),
        )
        .await
        .expect("scoped discovery");
    let ids: Vec<&str> = found.iter().map(|n| n.id.as_str()).collect();
    assert!(
        ids.contains(&orphan.as_str()),
        "repaired row is in scope: {ids:?}"
    );
    assert!(ids.contains(&healthy.as_str()));
    assert!(!ids.contains(&ghost_only.as_str()));

    let prefix = format!("{live}-chunk-");
    let rows: Vec<(String, i64)> = sqlx::query_as(&node_counts_by_source_prefixes_sql(
        &config.age_graph_name(),
    ))
    .bind(std::slice::from_ref(&prefix))
    .bind(4_i32)
    .fetch_all(&pool)
    .await
    .expect("shared count SQL");
    assert_eq!(rows, vec![(prefix, 2)], "count matches the scoped graph");
}

async fn lineage_snapshot(
    graph: &PostgresAGEGraphStorage,
    ids: &[&String],
) -> Vec<(Vec<String>, Vec<String>)> {
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let node = read_node(graph, id).await;
        out.push((
            lineage(&node, "source_ids"),
            lineage(&node, "source_chunk_ids"),
        ));
    }
    out
}

#[test]
fn migration_158_support_apply_matches_the_migration() {
    assert_eq!(
        M158,
        include_str!("../../../migrations/support/158/apply.sql"),
        "support/158/apply.sql must stay byte-identical to the migration"
    );
    assert!(M158.contains("starts_with(x.tok, d.doc || '-chunk-')"));
    assert!(M158.contains("JOIN public.documents pd"));
}
