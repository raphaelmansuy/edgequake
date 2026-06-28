//! SPEC-028 Query Context Service contract tests — code is law.

use std::path::PathBuf;

use utoipa::OpenApi;

use edgequake_api::openapi::ApiDoc;

fn read_crate_src(rel: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(manifest.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

#[test]
fn spec028_query_context_service_exists() {
    let src = read_crate_src("src/services/query_context.rs");
    assert!(src.contains("pub async fn retrieve_context"));
    assert!(src.contains("pub async fn search_context"));
    assert!(src.contains("pub fn fetch_context_by_id"));
}

#[test]
fn spec028_context_bundle_mapper_exists() {
    let src = read_crate_src("src/services/context_bundle_mapper.rs");
    assert!(src.contains("map_engine_response_to_bundle"));
    assert!(src.contains("compute_retrieval_fingerprint"));
}

#[test]
fn spec028_source_reference_builder_dry() {
    let src = read_crate_src("src/services/source_reference_builder.rs");
    assert!(src.contains("build_sources_from_context"));
    let execute = read_crate_src("src/handlers/query/query_execute.rs");
    assert!(execute.contains("build_legacy_query_sources"));
    let chat = read_crate_src("src/handlers/chat/mod.rs");
    assert!(chat.contains("build_sources_from_context"));
}

#[test]
fn spec028_query_request_builder_ssot() {
    let src = read_crate_src("src/services/query_request_builder.rs");
    assert!(src.contains("pub fn build_engine_request"));
    assert!(execute_uses_builder());
}

fn execute_uses_builder() -> bool {
    read_crate_src("src/handlers/query/query_execute.rs").contains("build_engine_request")
}

#[test]
fn spec028_routes_registered() {
    let routes = read_crate_src("src/routes.rs");
    assert!(routes.contains("/query/context\", post"));
    assert!(routes.contains("/query/context/search\", post"));
    assert!(routes.contains("/query/context/{retrieval_id}"));
    assert!(routes.contains("/mcp\", post"));
    assert!(routes.contains("get(handlers::fetch_query_context)"));
}

#[test]
fn spec028_openapi_includes_context_paths() {
    let doc = ApiDoc::openapi();
    let paths = doc.paths.paths;
    for path in [
        "/api/v1/query/context",
        "/api/v1/query/context/search",
        "/api/v1/query/context/{retrieval_id}",
        "/api/v1/mcp",
        "/mcp",
        "/.well-known/oauth-protected-resource",
        "/.well-known/mcp/server.json",
    ] {
        assert!(paths.contains_key(path), "OpenAPI missing path: {path}");
    }
}

#[test]
fn spec028_context_types_agent_granularity_default() {
    let src = read_crate_src("src/handlers/context_types.rs");
    assert!(src.contains("ContentGranularity"));
    assert!(src.contains("ContextBundle"));
    assert!(src.contains("ContextRetrievalResponse"));
}

#[test]
fn spec028_retrieval_id_cache_exists() {
    let src = read_crate_src("src/services/retrieval_id_cache.rs");
    assert!(src.contains("pub fn new_retrieval_id"));
    assert!(src.contains("pub fn global_retrieval_cache"));
}

#[test]
fn spec028_bypass_rejected_in_context_service() {
    let src = read_crate_src("src/services/query_context.rs");
    assert!(src.contains("reject_bypass"));
    assert!(src.contains("bypass is not allowed"));
}

#[test]
fn spec028_chat_default_mode_mix() {
    let chat = read_crate_src("src/handlers/chat/mod.rs");
    assert!(
        chat.contains("unwrap_or(QueryMode::Mix)"),
        "chat default mode must be Mix (QRY-001)"
    );
}

#[test]
fn spec028_context_only_deprecation_header() {
    let execute = read_crate_src("src/handlers/query/query_execute.rs");
    assert!(execute.contains("Deprecation"));
    assert!(execute.contains("/api/v1/query/context"));
}

#[test]
fn spec028_stream_v3_bundle_field() {
    let types = read_crate_src("src/handlers/query_types.rs");
    assert!(types.contains("bundle: Option"));
}

#[test]
fn spec028_include_references_wired() {
    let execute = read_crate_src("src/handlers/query/query_execute.rs");
    assert!(execute.contains("request.include_references"));
}

#[test]
fn spec028_services_mod_exports() {
    let mod_rs = read_crate_src("src/services/mod.rs");
    assert!(mod_rs.contains("pub mod query_context"));
    assert!(mod_rs.contains("pub mod query_generation"));
    assert!(mod_rs.contains("pub mod context_bundle_mapper"));
    assert!(mod_rs.contains("pub mod source_reference_builder"));
}

#[test]
fn spec028_query_generation_service_exists() {
    let src = read_crate_src("src/services/query_generation.rs");
    assert!(src.contains("pub async fn execute_full_query"));
    assert!(src.contains("pub async fn execute_legacy_query_response"));
}

#[test]
fn spec028_mcp_handler_registered() {
    let routes = read_crate_src("src/routes.rs");
    assert!(routes.contains("/mcp\", post"));
    let dispatch = read_crate_src("src/mcp/gateway/dispatch.rs");
    assert!(dispatch.contains("tools/list"));
    assert!(dispatch.contains("edgequake_search"));
    let prm = read_crate_src("src/mcp/auth/protected_resource.rs");
    assert!(prm.contains("protected_resource_metadata"));
    let json_rpc = read_crate_src("src/mcp/gateway/json_rpc.rs");
    assert!(json_rpc.contains("json_rpc_http_status"));
    let validation = read_crate_src("src/mcp/gateway/tool_validation.rs");
    assert!(validation.contains("validate_tool_call"));
    let body_mod = read_crate_src("src/mcp/gateway/body.rs");
    assert!(body_mod.contains("MCP_MAX_BODY_BYTES"));
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(manifest.join("tests/spec028_mcp_oauth_e2e.rs").exists());
    assert!(manifest.join("tests/common/spec028_mcp.rs").exists());
}

#[test]
fn spec028_stream_uses_query_request_builder() {
    let stream = read_crate_src("src/handlers/query/query_stream.rs");
    assert!(stream.contains("build_engine_request"));
    assert!(stream.contains("QueryMode::Mix"));
}

#[test]
fn spec028_api_error_gone_for_expired_retrieval() {
    let err = read_crate_src("src/error.rs");
    assert!(err.contains("Gone(String)"));
    assert!(err.contains("StatusCode::GONE"));
}
