//! SPEC-180 W3 — querying a `normal` workspace (Postgres, mock providers).
//!
//! ```bash
//! DATABASE_URL=postgres://edgequake:edgequake@localhost:5432/edgequake \
//!   cargo test -p edgequake-api --features postgres --test e2e_spec180_normal_query -- --nocapture
//! ```
//!
//! Covers T-180-query-bm25-rescue, T-180-query-no-keyword-llm, T-180-mode-422,
//! T-180-graph-endpoints-409, T-180-kind-filter and T-180-delete-not-returned.

#![cfg(feature = "postgres")]

mod common;

use axum::http::StatusCode;
use common::spec180_harness::{create_workspace, harness, upload_scoped, Harness, USER_ID};
use serde_json::{json, Value};
use serial_test::serial;

const LAB: &str = "The Zurich research lab studies kestrel migration routes. \
    Kestrel nests were counted along the river terraces each spring.";
const GARDEN: &str = "Tomatoes need warm soil, regular watering and plenty of sunlight. \
    Gardeners prune the lower leaves to keep the plants healthy.";
const BUDGET: &str = "The finance team reviewed the annual budget and approved the hiring plan \
    for the coming fiscal year.";

async fn query(h: &Harness, tenant: &str, ws: &str, body: Value) -> (StatusCode, Value) {
    common::post_json_with_tenant(&h.app, "/api/v1/query", &body, tenant, USER_ID, ws).await
}

fn source_documents(response: &Value) -> Vec<String> {
    response["sources"]
        .as_array()
        .map(|sources| {
            sources
                .iter()
                .filter(|s| s["source_type"] == "chunk")
                .filter_map(|s| s["document_id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
#[serial]
fn normal_query_retrieves_chunks_without_a_graph() {
    common::spec180::block_on_big_stack(normal_query_retrieves_chunks_without_a_graph_body());
}

async fn normal_query_retrieves_chunks_without_a_graph_body() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 normal query: postgres not reachable");
        return;
    };
    let (tenant, ws) = create_workspace(&h, "normal").await;
    let (lab, _, _) = upload_scoped(&h, &tenant, &ws, "lab", LAB).await;
    let (garden, _, _) = upload_scoped(&h, &tenant, &ws, "garden", GARDEN).await;
    let (budget, _, _) = upload_scoped(&h, &tenant, &ws, "budget", BUDGET).await;

    // T-180-query-no-keyword-llm + a lexical rescue: a rare word finds its document.
    // Retrieval does not extract keywords. It does make one rewrite call (fail-open;
    // the mock completion is rejected, so the original question is what is searched).
    let chat_before = h.mock.chat_calls();
    let (st, body) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "kestrel", "context_only": true}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(
        h.mock.chat_calls(),
        chat_before + 1,
        "context-only retrieval is one rewrite and no answer"
    );
    let docs = source_documents(&body);
    assert_eq!(
        docs.first(),
        Some(&lab),
        "kestrel must rank the lab first: {body}"
    );
    assert!(
        body["subgraph"].is_null()
            || body["subgraph"]["entities"]
                .as_array()
                .is_none_or(Vec::is_empty),
        "a normal workspace returns no subgraph: {body}"
    );

    // A full answer is a rewrite plus the answer, never a keyword extraction.
    let (st, body) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "Where are kestrels counted?"}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(
        h.mock.chat_calls(),
        chat_before + 3,
        "rewrite on the context-only query, then rewrite plus one answer: {body}"
    );

    // Stopword-only text is a valid query, not an error.
    let (st, body) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "the of and", "context_only": true}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{body}");

    // T-180-mode-422: graph modes are refused, naive and omitted work.
    for mode in ["local", "global", "hybrid", "mix"] {
        let (st, err) = query(&h, &tenant, &ws, json!({"query": "kestrel", "mode": mode})).await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{mode}: {err}");
        assert_eq!(
            err["code"], "mode_not_supported_by_profile",
            "{mode}: {err}"
        );
    }
    let (st, body) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "kestrel", "mode": "naive", "context_only": true}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{body}");

    // T-180-kind-filter: the SQL predicate, not a post-filter.
    let (_, body) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "kestrel", "context_only": true,
               "document_filter": {"doc_kinds": ["image"]}}),
    )
    .await;
    assert!(
        source_documents(&body).is_empty(),
        "image filter leaked text: {body}"
    );
    let (_, body) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "kestrel", "context_only": true,
               "document_filter": {"source_modalities": ["image"]}}),
    )
    .await;
    assert!(
        source_documents(&body).is_empty(),
        "modality filter leaked: {body}"
    );
    let (st, body) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "kestrel", "context_only": true,
               "document_filter": {"doc_kinds": ["text", "markdown"], "source_modalities": ["text"]}}),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(source_documents(&body).first(), Some(&lab), "{body}");
    let (st, err) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "kestrel", "document_filter": {"doc_kinds": ["spreadsheet"]}}),
    )
    .await;
    assert_eq!(
        (st, err["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("invalid_doc_kind")),
        "{err}"
    );

    // T-180-graph-endpoints-409.
    for uri in [
        "/api/v1/graph",
        "/api/v1/graph/entities",
        "/api/v1/graph/relationships",
        "/api/v1/graph/communities",
    ] {
        let (st, err) = common::get_with_tenant(&h.app, uri, &tenant, USER_ID, &ws).await;
        assert_eq!(st, StatusCode::CONFLICT, "{uri}: {err}");
        assert_eq!(err["code"], "profile_has_no_graph", "{uri}: {err}");
    }

    // T-180-delete-not-returned: a deleted document leaves the results.
    let (st, _) = common::delete_with_tenant(
        &h.app,
        &format!("/api/v1/documents/{lab}"),
        &tenant,
        USER_ID,
        &ws,
    )
    .await;
    assert!(st.is_success());
    let mut docs = vec![lab.clone()];
    for _ in 0..80 {
        let (_, body) = query(
            &h,
            &tenant,
            &ws,
            json!({"query": "kestrel", "context_only": true}),
        )
        .await;
        docs = source_documents(&body);
        if !docs.contains(&lab) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    assert!(!docs.contains(&lab), "deleted document still retrieved");
    let _ = (garden, budget);
    edgequake_api::safety_limits::clear_test_provider_override();
}

/// A graph workspace is untouched: graph modes work, kind filters are refused loudly.
#[test]
#[serial]
fn graph_workspace_keeps_graph_modes_and_refuses_kind_filters() {
    common::spec180::block_on_big_stack(
        graph_workspace_keeps_graph_modes_and_refuses_kind_filters_body(),
    );
}

async fn graph_workspace_keeps_graph_modes_and_refuses_kind_filters_body() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 graph query regression: postgres not reachable");
        return;
    };
    let (tenant, ws) = create_workspace(&h, "graph").await;
    let (st, err) = query(
        &h,
        &tenant,
        &ws,
        json!({"query": "kestrel", "document_filter": {"doc_kinds": ["pdf"]}}),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{err}");
    assert_eq!(err["code"], "kind_filter_not_supported_by_profile", "{err}");

    let (st, body) =
        common::get_with_tenant(&h.app, "/api/v1/graph/entities", &tenant, USER_ID, &ws).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    edgequake_api::safety_limits::clear_test_provider_override();
}
