//! SPEC-017 edgequake-api DRY/SOLID contract — code is law.
//!
//! Source-level assertions for P0/P1 remediation items in
//! specs/017-dry-and-solid-audit/003-edgequake-api/001-audit.md

use std::path::PathBuf;

fn read_crate_src(rel: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(manifest.join(rel)).unwrap_or_else(|e| {
        panic!("read {}: {e}", rel);
    })
}

fn grep_count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn spec017_single_workspace_pipeline_factory() {
    let factory = read_crate_src("src/workspace_pipeline_factory.rs");
    assert!(
        factory.contains("pub struct WorkspacePipelineFactory"),
        "WorkspacePipelineFactory must exist"
    );
    assert!(
        factory.contains("PipelineFallbackPolicy"),
        "explicit Strict/Lenient policies required (API-SOLID-L-001)"
    );

    let state = read_crate_src("src/state/mod.rs");
    assert!(
        state.contains("WorkspacePipelineFactory::new"),
        "AppState must delegate to factory"
    );

    let processor = read_crate_src("src/processor/workspace_resolver.rs");
    assert!(
        processor.contains("WorkspacePipelineFactory::new"),
        "DocumentTaskProcessor must delegate to factory"
    );
}

#[test]
fn spec017_query_and_chat_share_execution_service() {
    let svc = read_crate_src("src/services/query_execution.rs");
    assert!(
        svc.contains("execute_sota_query_with_auth_fallback"),
        "shared execution service must exist"
    );
    assert!(
        svc.contains("resolve_workspace_query_resources"),
        "shared workspace resource resolver must exist"
    );

    for rel in [
        "src/handlers/query/query_execute.rs",
        "src/handlers/chat/completion.rs",
    ] {
        let src = read_crate_src(rel);
        assert!(
            src.contains("execute_sota_query_with_auth_fallback"),
            "{rel} must call shared execution service (API-DRY-002)"
        );
        assert!(
            src.contains("resolve_workspace_query_resources"),
            "{rel} must call shared resource resolver"
        );
        assert!(
            src.contains("WorkspaceProviderResolver"),
            "{rel} must route LLM resolution through resolver (API-SOLID-D-001)"
        );
    }
}

#[test]
fn spec017_query_error_semantic_mapping() {
    let err = read_crate_src("src/error.rs");
    assert!(
        err.contains("impl From<edgequake_query::error::QueryError> for ApiError"),
        "QueryError From impl required (API-DRY-006)"
    );
    assert!(
        err.contains("QueryError::InvalidQuery"),
        "InvalidQuery must map to BadRequest"
    );

    let svc = read_crate_src("src/services/query_execution.rs");
    assert!(
        svc.contains(".map_err(ApiError::from)"),
        "query execution must use ? / From, not Internal wrap"
    );
}

#[test]
fn spec017_app_state_composed_substates() {
    let state = read_crate_src("src/state/mod.rs");
    for bundle in [
        "StorageRuntime",
        "QueryRuntime",
        "AuthRuntime",
        "TaskRuntime",
    ] {
        assert!(
            state.contains(bundle),
            "AppState must compose {bundle} (API-SOLID-S-001)"
        );
    }
}

#[test]
fn spec017_enqueue_task_centralized() {
    let state = read_crate_src("src/state/mod.rs");
    assert!(
        state.contains("pub async fn enqueue_task"),
        "AppState::enqueue_task required (API-DRY-004)"
    );

    let pdf = read_crate_src("src/handlers/pdf_upload/helpers.rs");
    assert!(
        pdf.contains("state.enqueue_task"),
        "pdf upload must use centralized enqueue"
    );
}

#[test]
fn spec017_parse_workspace_id_unified() {
    let mw = read_crate_src("src/middleware.rs");
    assert!(
        mw.contains("pub fn parse_workspace_id"),
        "parse_workspace_id helper required (API-DRY-005)"
    );

    let resolve = read_crate_src("src/handlers/query/workspace_resolve.rs");
    assert!(
        resolve.contains("parse_workspace_id"),
        "workspace_resolve must use parse_workspace_id"
    );
    assert!(
        resolve.contains("validate_llm_override_pair")
            || read_crate_src("src/handlers/query/query_execute.rs")
                .contains("validate_llm_override_pair"),
        "query handlers must validate partial LLM override pairs"
    );
}

#[test]
fn spec017_no_duplicate_pipeline_builder_bodies() {
    let state = read_crate_src("src/state/mod.rs");
    let processor = read_crate_src("src/processor/workspace_resolver.rs");

    assert_eq!(
        grep_count(&state, "create_safe_llm_provider("),
        0,
        "AppState must not inline provider creation — factory owns it"
    );
    assert_eq!(
        grep_count(&processor, "create_safe_llm_provider("),
        0,
        "processor must not inline provider creation — factory owns it"
    );
}

#[test]
fn spec017_shared_query_bootstrap() {
    let bootstrap = read_crate_src("src/state/query_bootstrap.rs");
    assert!(
        bootstrap.contains("build_production_query_engines"),
        "shared query bootstrap required (API-DRY-003)"
    );
    assert!(
        bootstrap.contains("build_ingestion_pipeline"),
        "shared pipeline bootstrap required (API-DRY-003)"
    );

    for rel in ["src/state/memory.rs", "src/state/postgres.rs"] {
        let src = read_crate_src(rel);
        assert!(
            src.contains("query_bootstrap::build_production_query_engines"),
            "{rel} must use shared query bootstrap"
        );
        assert!(
            src.contains("query_bootstrap::build_ingestion_pipeline"),
            "{rel} must use shared pipeline bootstrap"
        );
    }
}
