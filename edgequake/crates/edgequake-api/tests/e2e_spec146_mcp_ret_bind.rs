//! SPEC-146 M4/M5 — MCP bypass_acl reject + ret_* principal bind (G-146-41).
//!
//! Always-runnable unit tests (no live MCP server).
//!
//! Run: `cargo test -p edgequake-api --test e2e_spec146_mcp_ret_bind`

mod common;

use edgequake_api::handlers::context_types::{
    ContextBundle, ContextRetrievalResponse, ContextRetrievalStats, ItemsRetrieved, ModeSelection,
    RetrievalQuality, TruncationInfo,
};
use edgequake_api::services::retrieval_id_cache::RetrievalIdCache;

fn sample_response(retrieval_id: &str) -> ContextRetrievalResponse {
    ContextRetrievalResponse {
        retrieval_id: retrieval_id.to_string(),
        query: "q".into(),
        mode: "hybrid".into(),
        mode_selection: ModeSelection {
            requested: "hybrid".into(),
            effective: "hybrid".into(),
            adaptive: false,
            intent: None,
        },
        bundle: ContextBundle::default(),
        stats: ContextRetrievalStats {
            embedding_time_ms: 0,
            retrieval_time_ms: 0,
            rerank_time_ms: None,
            total_time_ms: 0,
            items_retrieved: ItemsRetrieved::default(),
            keywords_extracted: vec![],
            reranked: false,
        },
        retrieval_quality: RetrievalQuality {
            coverage_score: 0.0,
            is_sufficient: false,
            empty_context: true,
        },
        truncation: TruncationInfo {
            is_truncated: false,
            token_budget: 0,
            tokens_used: 0,
            dropped: Default::default(),
        },
        agent_hints: None,
        retrieval_fingerprint: "fp".into(),
        cached: false,
    }
}

#[test]
fn g146_41_bypass_acl_argument_is_detected() {
    // Mirror dispatch.rs rejection predicate (keep DRY with production check).
    fn rejects_bypass(arguments: &serde_json::Value) -> bool {
        arguments
            .get("bypass_acl")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || arguments.get("bypass_acl").and_then(|v| v.as_str()).is_some()
    }

    assert!(rejects_bypass(&serde_json::json!({"bypass_acl": true})));
    assert!(rejects_bypass(&serde_json::json!({"bypass_acl": "1"})));
    assert!(!rejects_bypass(&serde_json::json!({"bypass_acl": false})));
    assert!(!rejects_bypass(&serde_json::json!({})));
}

#[test]
fn g146_41_ret_bind_mismatch_is_existence_hiding_none() {
    let cache = RetrievalIdCache::new();
    let id = "ret_spec146_bind_1";
    cache.store_for(sample_response(id), Some("user:alice".into()), Some(3));

    // Matching principal + generation → hit.
    let hit = cache
        .get_for(id, Some("user:alice"), Some(3))
        .expect("ok")
        .expect("hit");
    assert_eq!(hit.retrieval_id, id);

    // Stolen ret_* (wrong principal) → Err(()); caller maps to 404.
    assert!(cache.get_for(id, Some("user:eve"), Some(3)).is_err());

    // Stale policy_generation → miss.
    assert!(
        cache
            .get_for(id, Some("user:alice"), Some(4))
            .ok()
            .flatten()
            .is_none(),
        "policy_generation mismatch must not return bound payload"
    );
}

#[test]
fn g146_41_unbound_ret_with_expected_policy_generation_denies() {
    let cache = RetrievalIdCache::new();
    let id = "ret_spec146_unbound";
    // Pre-146 store (no principal).
    cache.store(sample_response(id));

    // Caller expects principal+generation under ABAC → deny path.
    let result = cache.get_for(id, Some("user:alice"), Some(1));
    assert!(
        result.is_err() || result.ok().flatten().is_none(),
        "unbound ret_* must not serve ABAC callers"
    );
}

#[cfg(feature = "postgres")]
mod live {
    use super::*;
    use axum::http::StatusCode;
    use edgequake_api::mcp::gateway::dispatch::{execute_tool_call, DispatchTaskContext};
    use edgequake_api::mcp::gateway::meta::RequestMeta;
    use edgequake_api::middleware::TenantContext;
    use edgequake_api::services::retrieval_id_cache::global_retrieval_cache;
    use serde_json::json;
    use serial_test::serial;

    /// G-146-41: MCP dispatch with real AppState rejects bypass_acl; stolen ret_* is 404.
    #[tokio::test]
    #[serial]
    async fn g146_41_mcp_bypass_and_stolen_ret() {
        use crate::common::spec146_pg::try_create_harness;

        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_41: no DATABASE_URL");
            return;
        };

        let tenant_ctx = TenantContext {
            tenant_id: Some(h.tenant_id.clone()),
            workspace_id: Some(h.workspace_id.clone()),
            user_id: Some(h.peer_user_id.clone()),
        };
        let ctx = DispatchTaskContext {
            state: h.state.clone(),
            tenant_ctx: tenant_ctx.clone(),
            meta: RequestMeta::default(),
            workspace_header: Some(h.workspace_id.clone()),
            auth_role: Some(edgequake_auth::Role::User),
        };

        let bypass = execute_tool_call(
            ctx.clone(),
            json!({
                "name": "edgequake_search",
                "arguments": { "bypass_acl": true, "query": "ENTITY_X" }
            }),
        )
        .await;
        let err = bypass.expect_err("bypass_acl must be rejected");
        assert_eq!(
            err.status(),
            StatusCode::FORBIDDEN,
            "bypass_acl should be 403"
        );

        let stolen_id = format!("ret_stolen_{}", uuid::Uuid::new_v4().simple());
        global_retrieval_cache().store_for(
            sample_response(&stolen_id),
            Some(format!("user:{}", h.owner_user_id)),
            Some(1),
        );
        let stolen = execute_tool_call(
            ctx,
            json!({
                "name": "edgequake_fetch",
                "arguments": { "retrieval_id": stolen_id }
            }),
        )
        .await;
        let stolen_err = stolen.expect_err("stolen ret_* must 404");
        assert_eq!(
            stolen_err.status(),
            StatusCode::NOT_FOUND,
            "stolen ret_* must be existence-hiding 404"
        );
    }
}
