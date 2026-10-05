//! Document graph discovery through real HTTP with FORCE RLS and a large label.
#![cfg(feature = "postgres")]
mod common;

use common::provider_access::{harness, http_harness};
use edgequake_storage::traits::{EdgeListFilter, NodeListFilter};
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn large_document_graph_preserves_scope_and_finishes_under_budget() {
    let Some(url) = harness::certification_database_url().expect("strict PG configuration") else {
        return;
    };
    let server = http_harness::boot_anonymous(&url).await;
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    let sibling = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let foreign_workspace = Uuid::new_v4();
    for (t, w, label) in [
        (tenant, workspace, "lineage"),
        (tenant, sibling, "lineage-sibling"),
        (foreign, foreign_workspace, "lineage-foreign"),
    ] {
        http_harness::seed_scope(&server.pool, t, w, label).await;
    }
    let document = http_harness::commit_and_drain(
        &server.pool,
        &server.committer(),
        tenant,
        workspace,
        "LINEAGE_OWN",
        "LINEAGE_OWN_NODE",
        true,
    )
    .await
    .document_id;
    let graph = server.state.storage.graph_storage.clone();
    server.state.storage.kv_storage.upsert(&[(format!("{document}-metadata"),
        json!({"tenant_id":tenant,"workspace_id":workspace,"title":"LINEAGE_OWN","document_id":document}))]).await.unwrap();
    let graph_name = edgequake_storage::PostgresAGEGraphStorage::new(
        edgequake_storage::PostgresConfig::default(),
    )
    .graph_name()
    .to_string();
    // Same document tokens across scopes deliberately test RLS, not accidental ID uniqueness.
    for (t, w, label) in [
        (tenant, workspace, "own"),
        (tenant, sibling, "sibling"),
        (foreign, foreign_workspace, "foreign"),
    ] {
        for n in ["A", "B"] {
            graph.upsert_node(&format!("{workspace}-{label}-{n}"), json!({"tenant_id":t,"workspace_id":w,
                "entity_type":"TEST", "source_ids":[document.to_string(),format!("{document}-chunk-0")],
                "source_chunk_ids":[format!("{document}-chunk-0")]}).as_object().unwrap().clone().into_iter().collect()).await.unwrap();
        }
        graph.upsert_edge(&format!("{workspace}-{label}-A"), &format!("{workspace}-{label}-B"),
            json!({"tenant_id":t,"workspace_id":w,"relation_type":"USES", "source_ids":[format!("{document}-chunk-0")],
                "source_chunk_ids":[format!("{document}-chunk-0")]}).as_object().unwrap().clone().into_iter().collect()).await.unwrap();
    }
    // Bulk fixture avoids thousands of adapter calls. Rows are deliberately scoped
    // to the same tenant so the old non-bypass plan repeatedly scans all 12k rows.
    let insert = format!(
        r#"INSERT INTO {graph_name}."Node" (properties)
        SELECT jsonb_build_object('node_id',$1::text||'-noise-'||i,'tenant_id',$2::text,
            'workspace_id',$3::text,'description',repeat(md5(i::text),40),
            'source_ids',jsonb_build_array('other-doc-chunk-'||i),
            'source_chunk_ids',jsonb_build_array('other-doc-chunk-'||i))::text::ag_catalog.agtype
        FROM generate_series(1,12000) t(i)"#
    );
    sqlx::query(&insert)
        .bind(workspace.to_string())
        .bind(tenant.to_string())
        .bind(workspace.to_string())
        .execute(&server.pool)
        .await
        .unwrap();
    sqlx::query(&format!(r#"ANALYZE {graph_name}."Node""#))
        .execute(&server.pool)
        .await
        .unwrap();
    let nodes = graph
        .find_nodes_by_source_prefixes(
            &NodeListFilter {
                tenant_id: Some(tenant.to_string()),
                workspace_id: Some(workspace.to_string()),
                ..Default::default()
            },
            &[document.to_string()],
        )
        .await
        .unwrap();
    assert_eq!(
        nodes.len(),
        2,
        "duplicates across array keys/probes must collapse"
    );
    assert!(nodes.iter().all(|node| node.id.contains("-own-")));
    let edges = graph
        .find_edges_by_source_prefixes(
            &EdgeListFilter {
                tenant_id: Some(tenant.to_string()),
                workspace_id: Some(workspace.to_string()),
                ..Default::default()
            },
            &[document.to_string()],
        )
        .await
        .unwrap();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].source.contains("-own-"));
    let explain_sql = format!(
        r#"EXPLAIN (ANALYZE, FORMAT JSON, TIMING OFF)
        WITH probes AS MATERIALIZED (SELECT $1::text AS probe_id UNION SELECT $1::text||'-chunk-'||i::text FROM generate_series(0,255)t(i)),
        probe_ids AS MATERIALIZED (SELECT array_agg(probe_id) AS ids FROM probes)
        SELECT v.properties FROM {graph_name}."Node" v CROSS JOIN probe_ids p
        WHERE (ag_catalog.agtype_to_json(v.properties)::jsonb->'source_ids') ?| p.ids
           OR (ag_catalog.agtype_to_json(v.properties)::jsonb->'source_chunk_ids') ?| p.ids LIMIT 5000"#
    );
    let probe = document.to_string();
    let plan: serde_json::Value = edgequake_storage::adapters::postgres::rls::with_rls_transaction(
        &server.pool,
        tenant,
        Some(workspace),
        None,
        move |conn| {
            Box::pin(async move {
                sqlx::query_scalar(&explain_sql)
                    .bind(probe)
                    .fetch_one(conn)
                    .await
                    .map_err(edgequake_storage::StorageError::from)
            })
        },
    )
    .await
    .unwrap();
    fn assert_single_label_scan(plan: &serde_json::Value) -> usize {
        let mut scans = 0;
        if plan["Relation Name"] == "Node" {
            assert_eq!(plan["Actual Loops"], json!(1), "{plan}");
            scans += 1;
        }
        if let Some(children) = plan["Plans"].as_array() {
            for child in children {
                scans += assert_single_label_scan(child);
            }
        }
        scans
    }
    assert_eq!(
        assert_single_label_scan(&plan[0]["Plan"]),
        1,
        "must measure a real label scan"
    );
    let mut samples = Vec::new();
    for _ in 0..5 {
        let start = std::time::Instant::now();
        let response = server
            .client
            .get(format!(
                "{}/api/v1/lineage/documents/{document}",
                server.base
            ))
            .header("X-Tenant-ID", tenant.to_string())
            .header("X-Workspace-ID", workspace.to_string())
            .header("X-User-ID", Uuid::new_v4().to_string())
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        assert_eq!(status, reqwest::StatusCode::OK, "{body}");
        assert!(body.contains("-own-A") && body.contains("-own-B"), "{body}");
        assert!(
            !body.contains("-sibling-") && !body.contains("-foreign-"),
            "{body}"
        );
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    // Legacy records lack both indexed arrays. Exercise their optional fallback
    // with identical source tokens in every scope, under the same RLS envelope.
    for (t, w, label) in [
        (tenant, workspace, "own"),
        (tenant, sibling, "sibling"),
        (foreign, foreign_workspace, "foreign"),
    ] {
        sqlx::query(&format!(
            r#"INSERT INTO {graph_name}."Node" (properties) VALUES($1::text::ag_catalog.agtype)"#
        ))
        .bind(
            json!({"node_id":format!("{workspace}-{label}-legacy"),"tenant_id":t,"workspace_id":w,
                "entity_type":"TEST","source_id":format!("{document}-chunk-0")})
            .to_string(),
        )
        .execute(&server.pool)
        .await
        .unwrap();
        let legacy_edge = sqlx::query(&format!(
            r#"INSERT INTO {graph_name}."EDGE" (start_id,end_id,properties)
                SELECT start_id,end_id,$1::text::ag_catalog.agtype FROM {graph_name}."EDGE"
                WHERE eq_source_id=$2 AND eq_target_id=$3 AND eq_rel_type='USES'"#
        ))
        .bind(
            json!({"tenant_id":t,"workspace_id":w,"relation_type":"LEGACY",
                "source_id":format!("{document}-chunk-0-{label}-legacy"),
                "target_id":format!("{workspace}-{label}-B")})
            .to_string(),
        )
        .bind(format!("{workspace}-{label}-A"))
        .bind(format!("{workspace}-{label}-B"))
        .execute(&server.pool)
        .await
        .unwrap();
        assert_eq!(legacy_edge.rows_affected(), 1);
    }
    std::env::set_var("EDGEQUAKE_SOURCE_PREFIX_LEGACY", "1");
    let legacy = graph
        .find_nodes_by_source_prefixes(
            &NodeListFilter {
                tenant_id: Some(tenant.to_string()),
                workspace_id: Some(workspace.to_string()),
                ..Default::default()
            },
            &[document.to_string()],
        )
        .await
        .unwrap();
    assert_eq!(legacy.len(), 3);
    assert!(legacy.iter().all(|node| node.id.contains("-own-")));
    let legacy_edges = graph
        .find_edges_by_source_prefixes(
            &EdgeListFilter {
                tenant_id: Some(tenant.to_string()),
                workspace_id: Some(workspace.to_string()),
                ..Default::default()
            },
            &[document.to_string()],
        )
        .await
        .unwrap();
    assert_eq!(legacy_edges.len(), 2);
    assert!(legacy_edges
        .iter()
        .all(|edge| edge.source.contains("-own-")));
    std::env::remove_var("EDGEQUAKE_SOURCE_PREFIX_LEGACY");
    if let Ok(path) = std::env::var("EQ_GRAPH_LINEAGE_REPORT") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"fixture_nodes":12000,"samples_ms":samples,"plan":plan,"scope":"real TCP document-lineage, non-bypass RLS"})).unwrap()).unwrap();
    }
    eprintln!("POSTGRES_GRAPH_LINEAGE_PASS samples_ms={samples:?}");
}
