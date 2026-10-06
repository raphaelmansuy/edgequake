//! Contract: residual GH-404 cousins must use child tables + eq_* (no parent text-cast).

#[test]
fn contract_list_nodes_filtered_uses_node_child_table() {
    let scan = include_str!("../src/adapters/postgres/graph/scan_ops.rs");

    let page = scan
        .find("pub(super) fn list_nodes_filtered_page_sql")
        .expect("list_nodes_filtered_page_sql");
    let page_body = &scan[page..];
    let page_end = page_body
        .find("pub(super) fn list_nodes_filtered_count_sql")
        .unwrap_or(page_body.len().min(800));
    let page_region = &page_body[..page_end];

    assert!(
        page_region.contains(".\"Node\" v") || page_region.contains(".\\\"Node\\\" v"),
        "list_nodes page must scan child Node table"
    );
    assert!(
        !page_region.contains("_ag_label_vertex"),
        "list_nodes page must not touch _ag_label_vertex"
    );
    assert!(
        page_region.contains("OFFSET {offset}"),
        "list API may keep OFFSET for page/total"
    );

    let count = scan
        .find("pub(super) fn list_nodes_filtered_count_sql")
        .expect("list_nodes_filtered_count_sql");
    let count_body = &scan[count..];
    let count_end = count_body
        .find("pub(super) async fn pg_list_nodes_filtered")
        .unwrap_or(count_body.len().min(400));
    let count_region = &count_body[..count_end];
    assert!(
        count_region.contains(".\"Node\" v") || count_region.contains(".\\\"Node\\\" v"),
        "list_nodes count must scan child Node table"
    );
    assert!(
        !count_region.contains("_ag_label_vertex"),
        "list_nodes count must not touch _ag_label_vertex"
    );
}

#[test]
fn contract_popular_and_search_use_edge_eq_endpoints() {
    let search = include_str!("../src/adapters/postgres/graph/query_ops/search.rs");

    assert!(
        search.contains("fn popular_nodes_with_degree_sql"),
        "popular SQL builder must exist"
    );
    assert!(
        search.contains("fn search_nodes_with_degree_sql"),
        "search-nodes SQL builder must exist"
    );

    for (label, marker, next) in [
        (
            "popular",
            "fn popular_nodes_with_degree_sql",
            "fn search_nodes_with_degree_sql",
        ),
        (
            "search",
            "fn search_nodes_with_degree_sql",
            "fn popular_labels_sql",
        ),
        (
            "popular_labels",
            "fn popular_labels_sql",
            "async fn pg_get_popular_labels",
        ),
    ] {
        let start = search
            .find(marker)
            .unwrap_or_else(|| panic!("{label} builder"));
        let body = &search[start..];
        let end = body.find(next).unwrap_or(body.len().min(1200));
        let region = &body[..end];
        assert!(
            region.contains(".\"Node\"") || region.contains(".\\\"Node\\\""),
            "{label} must scan Node child"
        );
        assert!(
            region.contains("FROM {src_rows} e")
                && include_str!("../src/adapters/postgres/graph/helpers/eq_id_sql.rs")
                    .contains("FROM {graph}.\\\"EDGE\\\" e"),
            "{label} must scan EDGE child"
        );
        assert!(
            !region.contains("_ag_label_vertex"),
            "{label} must not use _ag_label_vertex"
        );
        assert!(
            !region.contains("_ag_label_edge"),
            "{label} must not use _ag_label_edge"
        );
        assert!(
            !region.contains("start_id::text"),
            "{label} must not text-cast JOIN on start_id"
        );
    }
}

#[test]
fn contract_node_degree_delegates_to_batch_eq_path() {
    let read = include_str!("../src/adapters/postgres/graph/nodes_ops/read.rs");
    let start = read
        .find("async fn pg_node_degree")
        .expect("pg_node_degree");
    let body = &read[start..];
    // Stop at end of this fn body — do not include the next fn's historical docs.
    let end = body.find("\n    }\n\n").map(|i| i + 6).unwrap_or(400);
    let region = &body[..end];
    assert!(
        region.contains("pg_node_degrees_batch"),
        "single-node degree must delegate to batch EDGE+eq_* path"
    );
    assert!(
        !region.contains("_ag_label_vertex"),
        "pg_node_degree must not query parent vertex table"
    );
    assert!(
        !region.contains("start_id::text"),
        "pg_node_degree must not text-cast JOIN on start_id"
    );
}

#[test]
fn contract_backfill_is_workspace_scoped() {
    let persist = include_str!("../src/community_persist.rs");
    let start = persist
        .find("pub async fn backfill_communities_if_needed")
        .expect("backfill_communities_if_needed");
    let body = &persist[start..];
    let end = body
        .find("pub async fn refresh_community_index")
        .unwrap_or(body.len().min(2500));
    let region = &body[..end];

    assert!(
        region.contains("workspaces_needing_community_backfill")
            || region.contains("workspace_id: Some"),
        "backfill must discover / set workspace scope"
    );
    assert!(
        !region.contains("detect_and_persist_communities(graph, &CommunityConfig::default())"),
        "backfill must not call unscoped detect_and_persist with default config"
    );
    assert!(
        region.contains("node_count_by_workspace"),
        "backfill size gate must be per-workspace"
    );
}

#[test]
fn contract_fuzzy_list_nodes_is_workspace_scoped() {
    let entity = include_str!("../../edgequake-pipeline/src/merger/entity.rs");
    let start = entity
        .find("async fn apply_graph_fuzzy_resolution")
        .expect("apply_graph_fuzzy_resolution");
    let body = &entity[start..];
    let end = body
        .find("fn collapse_duplicate_keys")
        .unwrap_or(body.len().min(2000));
    let region = &body[..end];

    assert!(
        region.contains("workspace_id: self.workspace_id.clone()")
            || region.contains("workspace_id: self.workspace_id"),
        "fuzzy graph sample must pass merger workspace_id"
    );
    assert!(
        region.contains("tenant_id: self.tenant_id.clone()")
            || region.contains("tenant_id: self.tenant_id"),
        "fuzzy graph sample must pass merger tenant_id"
    );
    assert!(
        !region.contains("list_nodes_filtered(&NodeListFilter::default(), 0, 500)"),
        "fuzzy must not use unscoped NodeListFilter::default()"
    );
}

#[test]
fn contract_detect_communities_guarded_requires_workspace() {
    let api = include_str!("../../edgequake-api/src/services/graph_community.rs");
    let start = api
        .find("pub async fn detect_communities_guarded")
        .expect("detect_communities_guarded");
    let body = &api[start..];
    let end = body.find("#[cfg(test)]").unwrap_or(body.len().min(1200));
    let region = &body[..end];
    assert!(
        region.contains("workspace_id.is_none()"),
        "API detect must reject missing workspace_id"
    );
}
