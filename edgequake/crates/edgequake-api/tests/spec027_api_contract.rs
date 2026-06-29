//! SPEC-027 API security + OpenAPI contract tests — code is law.

use std::path::PathBuf;

use utoipa::OpenApi;

use edgequake_api::openapi::ApiDoc;

fn read_crate_src(rel: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(manifest.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

#[test]
fn spec027_auth_validation_service_exists() {
    let src = read_crate_src("src/services/auth_validation.rs");
    assert!(src.contains("validate_presented_token"));
    assert!(src.contains("validate_stored_api_key"));
}

#[test]
fn spec027_admin_handlers_require_admin_guard() {
    let admin = read_crate_src("src/handlers/admin.rs");
    assert!(
        admin.matches("ApiRequireAdmin").count() >= 8,
        "all admin entrypoints must use ApiRequireAdmin extractor (ARCH-D-001)"
    );
    assert!(
        !admin.contains("require_admin_request"),
        "admin handlers must not call require_admin_request directly"
    );
}

#[test]
fn spec027_startup_security_module_wired() {
    let startup = read_crate_src("src/startup_security.rs");
    assert!(startup.contains("validate_startup_security"));
    let main_rs = read_crate_src("../../src/main.rs");
    assert!(main_rs.contains("validate_startup_security"));
}

#[test]
fn spec027_openapi_includes_admin_paths() {
    let doc = ApiDoc::openapi();
    let paths = doc.paths.paths;
    for path in [
        "/api/v1/admin/tenants/{tenant_id}/quota",
        "/api/v1/admin/config/defaults",
        "/api/v1/admin/storage/inspect",
        "/api/v1/admin/entities/reconcile",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI missing admin path: {path}"
        );
    }
}

#[test]
fn spec027_models_openapi_paths_use_v1_prefix() {
    let models = read_crate_src("src/handlers/models.rs");
    assert!(!models.contains("path = \"/api/models\""));
    assert!(models.contains("path = \"/api/v1/models\""));
}

#[test]
fn spec027_entity_list_uses_batch_degrees() {
    let crud = read_crate_src("src/handlers/entities/entity_crud.rs");
    assert!(crud.contains("node_degrees_batch"));
    assert!(!crud.contains("node_degree(&node.id)"));
}

#[test]
fn spec027_search_nodes_uses_batch_degrees() {
    let search = read_crate_src("src/handlers/graph/graph_query/search.rs");
    assert!(
        search.contains("node_degrees_batch"),
        "neighbor expansion must batch degree lookups"
    );
    assert!(
        !search.contains(".node_degree(&neighbor.id)"),
        "search_nodes must not N+1 node_degree per neighbor"
    );
}

#[test]
fn spec027_merge_entities_uses_batch_edge_upsert() {
    let merge = read_crate_src("src/services/entity_merge.rs");
    assert!(
        merge.contains("upsert_edges_batch"),
        "merge must batch edge writes"
    );
    assert!(
        merge.contains("get_edges_for_node_set"),
        "merge must batch edge reads"
    );
    assert!(
        !merge.contains(".get_edge("),
        "merge must not N+1 get_edge per edge"
    );
    let ops = read_crate_src("src/handlers/entities/entity_ops.rs");
    assert!(ops.contains("rewire_merged_entity_edges"));
}

#[test]
fn spec027_pipeline_checkpoint_cleanup_uses_suffix_scan() {
    let cp = read_crate_src("src/processor/pipeline_checkpoint.rs");
    assert!(
        cp.contains("keys_with_suffix(CHECKPOINT_KEY_SUFFIX)"),
        "checkpoint cleanup must use suffix scan SSOT"
    );
    assert!(
        !cp.contains("keys_like(\"%-pipeline-checkpoint\")"),
        "checkpoint cleanup must not use leading-wildcard keys_like"
    );
    assert!(
        cp.contains("get_by_ids(&checkpoint_keys)"),
        "checkpoint cleanup must batch-read values"
    );
}

#[test]
fn spec027_document_filter_resolver_uses_scoped_metadata_ssot() {
    let resolver = read_crate_src("src/handlers/query/document_filter_resolver.rs");
    assert!(
        resolver.contains("load_scoped_document_metadata_entries"),
        "query filter must use scoped metadata SSOT"
    );
    assert!(
        !resolver.contains("load_all_document_metadata"),
        "query filter must not bypass scoped SSOT"
    );
}

#[test]
fn spec027_entity_merge_service_extracted() {
    let merge = read_crate_src("src/services/entity_merge.rs");
    assert!(merge.contains("rewire_merged_entity_edges"));
    let mod_rs = read_crate_src("src/services/mod.rs");
    assert!(mod_rs.contains("pub mod entity_merge"));
}

#[test]
fn spec027_reliability_graph_query_timeout_ssot() {
    let mat = read_crate_src("src/services/graph_materialization.rs");
    assert!(mat.contains("run_timed_graph_query"));
    assert!(mat.contains("graph_query_timeout"));
    assert!(mat.contains("GraphQueryRuntime"));
    assert!(!mat.contains("admit_graph_materialization_from_state"));
    let stream = read_crate_src("src/handlers/graph/graph_stream.rs");
    assert!(stream.contains("State<GraphQueryRuntime>"));
    assert!(stream.contains("State<StorageRuntime>"));
    assert!(!stream.contains("State<AppState>"));
    let health = read_crate_src("src/handlers/health_probes.rs");
    assert!(health.contains("COMPONENT_PING_TIMEOUT"));
    assert!(health.contains("probe_with_timeout"));
}

#[test]
fn spec027_rate_limit_middleware_wired_in_routes() {
    let routes = read_crate_src("src/routes.rs");
    assert!(routes.contains("tenant_rate_limit_from_state"));
    assert!(routes.contains("ollama_compat_gate"));
}

#[test]
fn spec027_neighborhood_uses_batch_incident_edges() {
    let svc = read_crate_src("src/services/entity_neighborhood.rs");
    assert!(svc.contains("get_incident_edges_batch"));
    assert!(!svc.contains("get_node_edges("));
    let ops = read_crate_src("src/handlers/entities/entity_ops.rs");
    assert!(ops.contains("build_entity_neighborhood"));
}

#[test]
fn spec027_tenant_guard_wired_in_handlers() {
    let list = read_crate_src("src/handlers/documents/query/list.rs");
    assert!(list.contains("tenant_guard"));
    assert!(list.contains("load_scoped_document_metadata"));
    let traversal = read_crate_src("src/handlers/graph/graph_query/traversal.rs");
    assert!(traversal.contains("empty_graph_response"));
    let costs = read_crate_src("src/handlers/costs.rs");
    assert!(costs.contains("empty_cost_summary"));
}

#[test]
fn spec027_document_list_uses_metadata_scan_ssot() {
    let list = read_crate_src("src/handlers/documents/query/list.rs");
    assert!(list.contains("load_scoped_document_metadata"));
    let stuck = read_crate_src("src/handlers/documents/recovery/stuck.rs");
    assert!(stuck.contains("load_scoped_document_metadata"));
    let reprocess = read_crate_src("src/handlers/documents/recovery/reprocess.rs");
    assert!(reprocess.contains("load_scoped_document_metadata"));
    let bulk = read_crate_src("src/handlers/documents/delete/bulk.rs");
    assert!(bulk.contains("load_scoped_document_metadata_entries"));
    let tasks = read_crate_src("src/handlers/tasks.rs");
    assert!(tasks.contains("load_scoped_document_metadata_entries"));
    let track = read_crate_src("src/handlers/documents/query/track_status.rs");
    assert!(track.contains("load_scoped_document_metadata"));
    let filter = read_crate_src("src/handlers/query/document_filter_resolver.rs");
    assert!(filter.contains("load_scoped_document_metadata_entries"));
    let stats = read_crate_src("src/handlers/workspaces/stats.rs");
    assert!(stats.contains("load_workspace_metadata_values"));
}

#[test]
fn spec027_tenant_guard_warn_helper_wired() {
    let guard = read_crate_src("src/services/tenant_guard.rs");
    assert!(guard.contains("warn_missing_tenant_context"));
    let list = read_crate_src("src/handlers/documents/query/list.rs");
    assert!(list.contains("warn_missing_tenant_context"));
}

#[test]
fn spec027_isolation_context_ssot() {
    let iso = read_crate_src("src/services/isolation_context.rs");
    assert!(iso.contains("IsolationMode"));
    let scope = read_crate_src("src/workspace_scope.rs");
    assert!(scope.contains("isolation_context::metadata_matches"));
}

#[test]
fn spec027_cost_aggregation_ssot() {
    let agg = read_crate_src("src/services/cost_aggregation.rs");
    assert!(agg.contains("load_scoped_document_metadata"));
    let costs = read_crate_src("src/handlers/costs.rs");
    assert!(costs.contains("cost_aggregation"));
    assert!(costs.contains("load_scoped_document_cost_rows"));
}

#[test]
fn spec027_document_metadata_scan_ssot() {
    let scan = read_crate_src("src/services/document_metadata_scan.rs");
    assert!(scan.contains("DOCUMENT_METADATA_SUFFIX"));
    assert!(scan.contains("metadata_matches_tenant_context"));
    assert!(scan.contains("load_scoped_document_metadata"));
}

#[test]
fn spec027_entity_neighborhood_extracted() {
    let svc = read_crate_src("src/services/entity_neighborhood.rs");
    assert!(svc.contains("build_entity_neighborhood"));
    assert!(svc.contains("get_incident_edges_batch"));
    let ops = read_crate_src("src/handlers/entities/entity_ops.rs");
    assert!(ops.contains("build_entity_neighborhood"));
}

#[test]
fn spec027_openapi_v1_path_coverage_threshold() {
    let doc = ApiDoc::openapi();
    let v1_count = doc
        .paths
        .paths
        .keys()
        .filter(|path| path.starts_with("/api/v1"))
        .count();
    assert!(
        v1_count >= 105,
        "expected >= 105 documented /api/v1 paths, got {v1_count}"
    );
}

#[test]
fn spec027_openapi_documents_and_settings_paths() {
    let doc = ApiDoc::openapi();
    let paths = doc.paths.paths;
    for path in [
        "/api/v1/documents/{document_id}",
        "/api/v1/documents",
        "/api/v1/settings/provider/status",
        "/api/v1/config/effective",
        "/api/v1/graph/labels/popular",
        "/api/v1/costs/history",
        "/api/v1/workspaces/{workspace_id}/rebuild-embeddings",
    ] {
        assert!(paths.contains_key(path), "OpenAPI missing path: {path}");
    }
}

#[test]
fn spec027_traversal_pushes_tenant_scope_to_storage() {
    let traversal = read_crate_src("src/handlers/graph/graph_query/traversal.rs");
    assert!(traversal.contains("get_knowledge_graph("));
    assert!(traversal.contains("tenant_for_kg.as_deref()"));
    assert!(traversal.contains("workspace_for_kg.as_deref()"));
    let query_ops = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../edgequake-storage/src/adapters/postgres/graph/query_ops.rs"),
    )
    .unwrap_or_else(|e| panic!("read query_ops.rs: {e}"));
    assert!(query_ops.contains("pg_get_knowledge_graph_scoped"));
}

#[test]
fn spec027_security_config_on_app_state() {
    let state_mod = read_crate_src("src/state/mod.rs");
    assert!(state_mod.contains("pub security: ApiSecurityConfig"));
}

#[test]
fn spec027_migration_046_startup_reconcile_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("MIGRATION_046_VERSION"));
    assert!(bootstrap.contains("reconcile_migration_046"));
    assert!(bootstrap.contains("migration_046"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m046.rs").exists());
    let postgres = read_crate_src("src/state/postgres.rs");
    assert!(postgres.contains("run_postgres_migrations"));
}

#[test]
fn spec027_openapi_pdf_progress_schema_registered() {
    let doc = ApiDoc::openapi();
    let components = doc.components.expect("components");
    let schemas = components.schemas;
    for name in [
        "PdfUploadProgress",
        "PhaseProgress",
        "QueryStreamEvent",
        "ChatStreamEvent",
    ] {
        assert!(schemas.contains_key(name), "OpenAPI missing schema: {name}");
    }
    let progress_path = doc
        .paths
        .paths
        .get("/api/v1/documents/pdf/progress/{track_id}")
        .expect("pdf progress path");
    let get = progress_path.get.as_ref().expect("GET progress");
    assert!(get.responses.responses.contains_key("200"));
}

#[test]
fn spec027_openapi_sse_streams_documented() {
    let doc = ApiDoc::openapi();
    for path in [
        "/api/v1/documents/pdf/progress/stream/{track_id}",
        "/api/v1/query/stream",
        "/api/v1/chat/completions/stream",
        "/api/v1/graph/stream",
    ] {
        let item = doc
            .paths
            .paths
            .get(path)
            .unwrap_or_else(|| panic!("missing {path}"));
        let op = item
            .post
            .as_ref()
            .or(item.get.as_ref())
            .unwrap_or_else(|| panic!("no operation for {path}"));
        let response = op
            .responses
            .responses
            .get("200")
            .unwrap_or_else(|| panic!("no 200 for {path}"));
        let content = match response {
            utoipa::openapi::RefOr::T(r) => &r.content,
            utoipa::openapi::RefOr::Ref(_) => panic!("unexpected ref response for {path}"),
        };
        assert!(
            content.contains_key("text/event-stream"),
            "{path} missing text/event-stream content"
        );
    }
}

#[test]
fn spec027_share_url_uses_v1_path() {
    assert_eq!(
        edgequake_api::handlers::share_api_path("abc"),
        "/api/v1/shared/abc"
    );
    let sharing = read_crate_src("src/handlers/conversations/sharing.rs");
    assert!(sharing.contains("share_api_path"));
}

#[test]
fn spec027_error_response_includes_rfc7807_fields() {
    let err = edgequake_api::error::ErrorResponse::new("NOT_FOUND", "missing");
    assert_eq!(
        err.problem_type.as_deref(),
        Some("https://edgequake.dev/problems/not-found")
    );
    assert_eq!(err.title.as_deref(), Some("Not Found"));
}

#[test]
fn spec027_health_exposes_api_capabilities() {
    let health_types = read_crate_src("src/handlers/health_types.rs");
    assert!(health_types.contains("pub struct ApiCapabilities"));
    let health = read_crate_src("src/handlers/health.rs");
    assert!(health.contains("capabilities: Some(ApiCapabilities"));
}

#[test]
fn spec027_openapi_public_paths_have_empty_security() {
    let doc = ApiDoc::openapi();
    for path in ["/health", "/api/v1/auth/login"] {
        let item = doc.paths.paths.get(path).expect(path);
        let op = item.get.as_ref().or(item.post.as_ref()).expect("operation");
        assert_eq!(
            op.security.as_ref().map(|s| s.len()),
            Some(0),
            "{path} should be public"
        );
    }
}

#[test]
fn spec027_lineage_uses_normalize_ssot() {
    let normalize = read_crate_src("src/handlers/lineage/normalize.rs");
    assert!(normalize.contains("normalize_entity_name"));
    let queries = read_crate_src("src/handlers/lineage/queries.rs");
    assert!(queries.contains("lookup_entity_node_for_context"));
    let provenance = read_crate_src("src/handlers/lineage/entity_provenance.rs");
    assert!(provenance.contains("lookup_entity_node_for_context"));
}

#[test]
fn spec027_v2_jobs_openapi_wired() {
    let doc = ApiDoc::openapi();
    let collection = "/api/v2/workspaces/{workspace_id}/jobs";
    let resource = "/api/v2/workspaces/{workspace_id}/jobs/{job_id}";
    let catalog = "/api/v2/workspaces/{workspace_id}/jobs/catalog";
    assert!(doc.paths.paths.contains_key(collection));
    assert!(doc.paths.paths.contains_key(resource));
    assert!(doc.paths.paths.contains_key(catalog));
    let jobs_get = doc.paths.paths.get(collection).expect(collection);
    assert!(
        jobs_get.get.is_some(),
        "GET workspace jobs must be registered"
    );
    assert!(
        jobs_get.post.is_some(),
        "POST workspace jobs must be registered"
    );
    let job_item = doc.paths.paths.get(resource).expect(resource);
    assert!(job_item.get.is_some(), "GET job by id must be registered");
    assert!(
        job_item.delete.is_some(),
        "DELETE job (cancel) must be registered on resource path"
    );
    let components = doc.components.expect("components");
    assert!(components.schemas.contains_key("JobResponse"));
    assert!(components.schemas.contains_key("JobListResponse"));
    assert!(components.schemas.contains_key("JobCatalogResponse"));
    assert!(components.schemas.contains_key("JobCatalogLinks"));
    let routes = read_crate_src("src/routes.rs");
    assert!(routes.contains("api_v2_routes"));
    assert!(routes.contains("create_workspace_job"));
    assert!(routes.contains("cancel_workspace_job"));
}

#[test]
fn spec027_job_registry_ssot() {
    let registry = read_crate_src("src/services/job_registry.rs");
    assert!(registry.contains("pub fn job_catalog("));
    assert!(registry.contains("is_creatable_v2_job_type"));
    assert!(registry.contains("CREATABLE_V2_JOB_TYPES"));
    assert!(registry.contains("rebuild_embeddings"));
    assert!(registry.contains("creatable_via_v2: true"));
    let submission = read_crate_src("src/handlers/v2/jobs/submission.rs");
    assert!(submission.contains("submit_workspace_job"));
    assert!(submission.contains("run_rebuild_embeddings"));
    assert!(submission.contains("run_reanalyze_multimodal"));
    assert!(submission.contains("is_creatable_v2_job_type"));
    let handlers = read_crate_src("src/handlers/v2/jobs/handlers.rs");
    assert!(handlers.contains("list_workspace_job_catalog"));
    let routes = read_crate_src("src/routes.rs");
    assert!(routes.contains("/workspaces/{workspace_id}/jobs/catalog"));
}

#[test]
fn spec027_v1_rpc_openapi_v2_migration_extensions() {
    use edgequake_api::openapi_enrichment::apply_openapi_enrichment;
    use edgequake_api::openapi_security::apply_path_security;

    let mut doc = ApiDoc::openapi();
    apply_openapi_enrichment(&mut doc);
    apply_path_security(&mut doc);

    let path = "/api/v1/workspaces/{workspace_id}/rebuild-embeddings";
    let item = doc.paths.paths.get(path).expect(path);
    let op = item.post.as_ref().expect("POST");
    let ext = op.extensions.as_ref().expect("extensions");
    assert_eq!(
        ext.get("x-edgequake-v2-job-type").and_then(|v| v.as_str()),
        Some("rebuild_embeddings")
    );
}

#[test]
fn spec027_v1_rpc_responses_include_v2_migration_field() {
    let rebuild = read_crate_src("src/handlers/workspaces_types/rebuild.rs");
    assert!(rebuild.contains("v2_migration: Option"));
    let recovery = read_crate_src("src/handlers/documents_types/recovery.rs");
    assert!(recovery.contains("v2_migration: Option"));
    let hint = read_crate_src("src/services/job_registry.rs");
    assert!(hint.contains("pub fn v2_migration_hint"));
}

#[test]
fn spec027_v2_level4_workspace_scoped_routes_only() {
    let routes = read_crate_src("src/routes.rs");
    assert!(routes.contains("/workspaces/{workspace_id}/jobs"));
    assert!(
        !routes.contains(".route(\"/jobs\""),
        "flat /api/v2/jobs removed — Level 4 workspace nesting only"
    );
}

#[test]
fn spec027_v1_rpc_migration_headers_ssot() {
    let migration = read_crate_src("src/services/v1_rpc_migration.rs");
    assert!(migration.contains("V1_RPC_SUNSET_RFC7231"));
    assert!(migration.contains("respond_v1_async_rpc"));
    assert!(migration.contains("successor-version"));
    let security = read_crate_src("src/state/security_config.rs");
    assert!(security.contains("v1_rpc_return_202"));
    assert!(security.contains("EDGEQUAKE_V1_RPC_RETURN_202"));
    assert!(
        security.contains("v1_rpc_return_202: true"),
        "Default ApiSecurityConfig must enable REST-025 202"
    );
    for path in [
        "src/handlers/workspaces/bulk_ops/rebuild_embeddings.rs",
        "src/handlers/documents/recovery/stuck.rs",
    ] {
        let src = read_crate_src(path);
        assert!(
            src.contains("respond_v1_async_rpc"),
            "{path} must use respond_v1_async_rpc SSOT"
        );
    }
}

#[test]
fn spec027_auth_extractors_arch_d001() {
    let extractors = read_crate_src("src/handlers/auth/extractors.rs");
    assert!(extractors.contains("ApiAuthenticated"));
    assert!(extractors.contains("ApiRequireAdmin"));
    assert!(extractors.contains("ApiOptionalAuth"));
    assert!(extractors.contains("FromRequestParts<AppState>"));
    let runtime = read_crate_src("src/state/runtime_extractors.rs");
    assert!(
        runtime.contains("impl FromRef<AppState> for AuthState"),
        "AuthState FromRef for JWT-only handlers"
    );
    let user_mgmt = read_crate_src("src/handlers/auth/user_management.rs");
    assert!(
        user_mgmt.matches("ApiRequireAdmin").count() >= 4,
        "admin user CRUD must use ApiRequireAdmin"
    );
    assert!(
        user_mgmt.contains("ApiOptionalAuth"),
        "create_user must use ApiOptionalAuth"
    );
    assert!(
        user_mgmt.contains("State<AuthRuntime>"),
        "create_user must use AuthRuntime ISP"
    );
    assert!(
        !user_mgmt.contains("require_admin_request"),
        "user_management must not call require_admin_request directly"
    );
    assert!(
        !user_mgmt.contains("authenticate_request"),
        "user_management must not call authenticate_request directly"
    );
    let api_keys = read_crate_src("src/handlers/auth/api_keys.rs");
    assert!(
        api_keys.matches("ApiAuthenticated").count() >= 3,
        "api_keys handlers must use ApiAuthenticated"
    );
    assert!(
        !api_keys.contains("require_authenticated_request"),
        "api_keys must not call require_authenticated_request directly"
    );
    let session = read_crate_src("src/handlers/auth/session.rs");
    let login_fn = session
        .split("pub async fn login")
        .nth(1)
        .expect("login handler");
    assert!(login_fn.contains("State<AuthRuntime>"));
    assert!(login_fn.contains("State<StorageRuntime>"));
    assert!(login_fn.contains("State<ComplianceRuntime>"));
    let refresh_fn = session
        .split("pub async fn refresh_token")
        .nth(1)
        .expect("refresh handler");
    assert!(refresh_fn.contains("State<AuthRuntime>"));
    assert!(refresh_fn.contains("State<StorageRuntime>"));
    let logout_fn = session
        .split("pub async fn logout")
        .nth(1)
        .expect("logout handler");
    assert!(logout_fn.contains("State<ComplianceRuntime>"));
    let get_me_fn = session
        .split("pub async fn get_me")
        .nth(1)
        .expect("get_me handler");
    assert!(
        get_me_fn.contains("ApiAuthenticated"),
        "get_me must use ApiAuthenticated extractor"
    );
    assert!(
        get_me_fn.contains("State<StorageRuntime>"),
        "get_me must use StorageRuntime ISP"
    );
    assert!(
        !get_me_fn.contains("verify_token"),
        "get_me must not manually verify JWT"
    );
}

#[test]
fn spec027_rest025_default_202_with_legacy_opt_out() {
    let security = read_crate_src("src/state/security_config.rs");
    assert!(
        security.contains("v1_rpc_return_202: true"),
        "REST-025 default must be 202"
    );
    assert!(security.contains("EDGEQUAKE_V1_RPC_RETURN_202"));
    for path in [
        "src/handlers/workspaces/bulk_ops/rebuild_embeddings.rs",
        "src/handlers/workspaces/bulk_ops/rebuild_knowledge_graph.rs",
        "src/handlers/workspaces/bulk_ops/reprocess_documents.rs",
        "src/handlers/documents/recovery/stuck.rs",
        "src/handlers/documents/recovery/reprocess.rs",
        "src/handlers/documents/recovery/reanalyze.rs",
    ] {
        let src = read_crate_src(path);
        assert!(
            src.contains("status = 202"),
            "{path} OpenAPI must document REST-025 202 response"
        );
        assert!(
            src.contains("respond_v1_async_rpc"),
            "{path} must use respond_v1_async_rpc SSOT"
        );
    }
}

#[test]
fn spec027_scan_directory_partial_isp() {
    let scan = read_crate_src("src/handlers/documents/query/scan.rs");
    let scan_fn = scan
        .split("pub async fn scan_directory")
        .nth(1)
        .expect("scan_directory");
    assert!(scan_fn.contains("State<StorageRuntime>"));
    assert!(scan_fn.contains("State<PathValidationConfig>"));
    assert!(scan_fn.contains("State<TaskRuntime>"));
    assert!(scan_fn.contains("State<AppConfig>"));
    assert!(!scan_fn.contains("State<AppState>"));
}

#[test]
fn spec027_v2_catalog_validates_workspace_scope() {
    let handlers = read_crate_src("src/handlers/v2/jobs/handlers.rs");
    assert!(handlers.contains("list_workspace_job_catalog"));
    let catalog_fn = handlers
        .split("pub async fn list_workspace_job_catalog")
        .nth(1)
        .expect("catalog handler");
    assert!(catalog_fn.contains("ensure_workspace_scope"));
}

#[test]
fn spec027_run_reanalyze_multimodal_extracted() {
    let reanalyze = read_crate_src("src/handlers/documents/recovery/reanalyze.rs");
    assert!(reanalyze.contains("pub(crate) async fn run_reanalyze_multimodal"));
    assert!(reanalyze.contains("v2_migration"));
    let recovery = read_crate_src("src/handlers/documents_types/recovery.rs");
    assert!(recovery.contains("ReanalyzeMultimodalResponse"));
    let idx = recovery
        .find("struct ReanalyzeMultimodalResponse")
        .expect("struct");
    let slice = &recovery[idx..recovery.len().min(idx + 400)];
    assert!(slice.contains("v2_migration"));
}

#[test]
fn spec027_graph_edge_response_from_storage_edge_ssot() {
    let graph_types = read_crate_src("src/handlers/graph_types.rs");
    assert!(graph_types.contains("fn from_storage_edge"));
    assert!(graph_types.contains("relationship_type"));
    assert!(graph_types.contains("relation_type"));

    for path in [
        "src/handlers/graph/graph_query/search.rs",
        "src/handlers/graph/graph_query/traversal.rs",
        "src/handlers/graph/graph_stream.rs",
    ] {
        let src = read_crate_src(path);
        assert!(
            src.contains("GraphEdgeResponse::from_storage_edge"),
            "{path} must use GraphEdgeResponse SSOT"
        );
        assert!(
            !src.contains("GraphEdgeResponse {"),
            "{path} must not inline GraphEdgeResponse construction"
        );
    }
}

#[test]
fn spec027_relationship_handlers_use_storage_runtime_isp() {
    for path in [
        "src/handlers/relationships/get.rs",
        "src/handlers/relationships/list.rs",
        "src/handlers/relationships/create.rs",
        "src/handlers/relationships/update.rs",
        "src/handlers/relationships/delete.rs",
        "src/handlers/lineage/entity_provenance.rs",
        "src/handlers/lineage/chunk_detail.rs",
        "src/handlers/lineage/queries.rs",
        "src/handlers/lineage/export.rs",
        "src/handlers/graph/graph_query/node.rs",
        "src/handlers/documents/query/track_status.rs",
        "src/handlers/documents/query/list.rs",
    ] {
        let src = read_crate_src(path);
        assert!(
            src.contains("State<StorageRuntime>"),
            "{path} must use StorageRuntime ISP extractor (API-SOLID-I-001)"
        );
        assert!(
            !src.contains("State<AppState>"),
            "{path} must not take full AppState when storage-only"
        );
    }
    for path in [
        "src/handlers/graph/graph_query/search.rs",
        "src/handlers/graph/graph_query/traversal.rs",
        "src/handlers/graph/graph_stream.rs",
    ] {
        let src = read_crate_src(path);
        assert!(
            src.contains("State<GraphQueryRuntime>"),
            "{path} must use GraphQueryRuntime for materialization guard"
        );
        assert!(
            src.contains("State<StorageRuntime>"),
            "{path} must use StorageRuntime ISP"
        );
    }
    let popular = read_crate_src("src/handlers/graph/graph_query/popular.rs");
    let popular_fn = popular
        .split("pub async fn get_popular_labels")
        .nth(1)
        .expect("get_popular_labels");
    assert!(popular_fn.contains("State<GraphQueryRuntime>"));
    assert!(popular_fn.contains("State<StorageRuntime>"));
    let batch_fn = popular
        .split("pub async fn get_degrees_batch")
        .nth(1)
        .expect("get_degrees_batch");
    assert!(
        batch_fn.contains("State<StorageRuntime>"),
        "get_degrees_batch must use StorageRuntime ISP"
    );
    assert!(
        !batch_fn.contains("State<AppState>"),
        "get_degrees_batch must not take full AppState"
    );
    let list = read_crate_src("src/handlers/relationships/list.rs");
    assert!(
        list.contains("State<ResourceBudgetConfig>"),
        "list_relationships must extract budget without full AppState"
    );
}

#[test]
fn spec027_entity_graph_lookup_ssot() {
    let svc = read_crate_src("src/services/entity_graph_lookup.rs");
    assert!(svc.contains("lookup_entity_node_for_context"));
    assert!(svc.contains("entity_lookup_candidates"));
    let provenance = read_crate_src("src/handlers/lineage/entity_provenance.rs");
    assert!(provenance.contains("lookup_entity_node_for_context"));
    let queries = read_crate_src("src/handlers/lineage/queries.rs");
    assert!(queries.contains("lookup_entity_node_for_context"));
}

#[test]
fn spec027_list_documents_uses_pagination_ssot() {
    let list = read_crate_src("src/handlers/documents/query/list.rs");
    assert!(list.contains("paginate_vec"));
    assert!(list.contains("clamp_page_size"));
    assert!(!list.contains("let page = 1usize"));
    let pagination = read_crate_src("src/services/list_pagination.rs");
    assert!(pagination.contains("pub fn paginate_vec"));
}

#[test]
fn spec027_list_documents_isp() {
    let list = read_crate_src("src/handlers/documents/query/list.rs");
    let list_fn = list
        .split("pub async fn list_documents")
        .nth(1)
        .expect("list_documents");
    assert!(list_fn.contains("State<StorageRuntime>"));
    assert!(list_fn.contains("State<PostgresRuntime>"));
    assert!(list_fn.contains("State<ResourceBudgetConfig>"));
    assert!(!list_fn.contains("State<AppState>"));
}

#[test]
fn spec027_get_document_isp() {
    let detail = read_crate_src("src/handlers/documents/query/detail.rs");
    let get_fn = detail
        .split("pub async fn get_document")
        .nth(1)
        .expect("get_document");
    assert!(get_fn.contains("State<StorageRuntime>"));
    assert!(get_fn.contains("State<PostgresRuntime>"));
    assert!(!get_fn.contains("State<AppState>"));
}

#[test]
fn spec027_login_lockout_sec011() {
    let session = read_crate_src("src/handlers/auth/session.rs");
    assert!(
        session.contains("login_lockout::ensure_login_allowed"),
        "login must check lockout before password verify"
    );
    assert!(
        session.contains("login_lockout::record_failed_login"),
        "login must record failed attempts"
    );
    assert!(
        session.contains("login_lockout::record_successful_login"),
        "login must clear lockout on success"
    );
    let lockout = read_crate_src("src/services/login_lockout.rs");
    assert!(lockout.contains("failed_login_attempts"));
    assert!(lockout.contains("locked_until"));
    assert!(lockout.contains("max_login_attempts"));
    let user_record = read_crate_src("src/handlers/auth/mod.rs");
    assert!(user_record.contains("failed_login_attempts"));
    assert!(user_record.contains("locked_until"));
    let error = read_crate_src("src/error.rs");
    assert!(error.contains("AccountLocked"));
    assert!(error.contains("account_locked"));
}

#[test]
fn spec027_identity_storage_ssot_phase33() {
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(identity.contains("sync_auth_user_to_postgres"));
    assert!(identity.contains("constant_time_str_eq"));
    assert!(identity.contains("IdentityPolicy"));
    assert!(identity.contains("load_user_record"));
    assert!(identity.contains("persist_user_record"));
    let auth_mod = read_crate_src("src/handlers/auth/mod.rs");
    assert!(auth_mod.contains("persist_user_record"));
    assert!(auth_mod.contains("get_record_by_id"));
    let session = read_crate_src("src/handlers/auth/session.rs");
    assert!(session.contains("access_token_claims"));
    let middleware = read_crate_src("src/middleware.rs");
    assert!(middleware.contains("membership_bind_scope"));
    assert!(middleware.contains("enforce_membership_bind"));
}

#[test]
fn spec027_conversation_rls_acquired_phase36() {
    let conversation = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../edgequake-storage/src/adapters/postgres/conversation.rs"),
    )
    .expect("conversation.rs");
    assert!(conversation.contains("acquire_rls_connection"));
    assert!(
        !conversation.contains("acquire_tenant_conn"),
        "must use rls.rs SSOT acquire_rls_connection"
    );
    assert!(
        !conversation.contains("set_context("),
        "legacy pool-level set_context must be removed"
    );
    assert!(
        !conversation.contains("set_tenant_context(&self.pool"),
        "must not set RLS on pool directly"
    );
}

#[test]
fn spec027_rls_acquire_ssot_phase37() {
    let rls = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../edgequake-storage/src/adapters/postgres/rls.rs"),
    )
    .expect("rls.rs");
    assert!(rls.contains("pub async fn acquire_rls_connection"));
    assert!(rls.contains("pub async fn release_rls_connection"));
    assert!(
        rls.contains("#[deprecated"),
        "legacy pool-level RlsContext must be deprecated"
    );
    assert!(
        rls.contains("acquire_rls_connection(pool, tenant_id, workspace_id, user_id)"),
        "with_acquired_tenant_context must delegate to acquire_rls_connection"
    );
    let postgres_mod = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../edgequake-storage/src/adapters/postgres/mod.rs"),
    )
    .expect("postgres/mod.rs");
    assert!(postgres_mod.contains("acquire_rls_connection"));
    assert!(postgres_mod.contains("release_rls_connection"));
}

#[test]
fn spec027_migration_050_pg_rls_ssot_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_050"));
    assert!(bootstrap.contains("migration_050"));
    assert!(bootstrap.contains("MIGRATION_050_VERSION"));
    assert!(bootstrap.contains("SQL_050_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m050.rs").exists());
    assert!(std::path::Path::new("../../migrations/050_pg_rls_context_ssot_marker.sql").exists());
}

#[test]
fn spec027_tenant_isolation_ssot_phase35() {
    let isolation = read_crate_src("src/services/tenant_isolation.rs");
    assert!(isolation.contains("PgIsolationScope"));
    assert!(isolation.contains("PostgreSQL is identity SSOT"));
    assert!(isolation.contains("with_acquired_tenant_context"));
    let rls = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../edgequake-storage/src/adapters/postgres/rls.rs"),
    )
    .expect("rls.rs");
    assert!(rls.contains("with_acquired_tenant_context"));
    assert!(rls.contains("set_tenant_context_on_conn"));
    let security = read_crate_src("src/state/security_config.rs");
    assert!(security.contains("pg_rls_enabled"));
    assert!(security.contains("EDGEQUAKE_PG_RLS_ENABLED"));
    assert!(security.contains("pg_identity_ssot"));
    assert!(security.contains("EDGEQUAKE_PG_IDENTITY_SSOT"));
    assert!(security.contains("pg_rls_enabled: true"));
    let middleware = read_crate_src("src/middleware.rs");
    assert!(middleware.contains("attach_pg_isolation_scope"));
}

#[test]
fn spec027_pg_identity_ssot_phase38() {
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(identity.contains("IdentityPolicy"));
    assert!(identity.contains("pg_primary"));
    assert!(identity.contains("find_user_record_by_login_pg"));
    assert!(identity.contains("list_user_records_pg"));
    let security = read_crate_src("src/state/security_config.rs");
    assert!(security.contains("pg_identity_ssot: true"));
    assert!(security.contains("pg_rls_enabled: true"));
    assert!(security.contains("EDGEQUAKE_KV_IDENTITY_MIRROR"));
}

#[test]
fn spec027_migration_051_pg_identity_primary_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_051"));
    assert!(bootstrap.contains("migration_051"));
    assert!(bootstrap.contains("MIGRATION_051_VERSION"));
    assert!(bootstrap.contains("SQL_051_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m051.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/051_pg_identity_ssot_primary_marker.sql").exists()
    );
}

#[test]
fn spec027_session_storage_pg_phase39() {
    let session = read_crate_src("src/services/session_storage.rs");
    assert!(session.contains("persist_refresh_token"));
    assert!(session.contains("load_refresh_token"));
    assert!(session.contains("persist_api_key"));
    assert!(session.contains("find_active_api_keys_by_prefix"));
    assert!(session.contains("refresh_token_lookup_hash"));
    let session_handler = read_crate_src("src/handlers/auth/session.rs");
    assert!(session_handler.contains("session_storage::persist_refresh_token"));
    assert!(session_handler.contains("session_storage::load_refresh_token"));
    let api_keys = read_crate_src("src/handlers/auth/api_keys.rs");
    assert!(api_keys.contains("session_storage::persist_api_key"));
    let validation = read_crate_src("src/services/auth_validation.rs");
    assert!(validation.contains("session_storage::find_active_api_keys_by_prefix"));
}

#[test]
fn spec027_migration_052_session_artifacts_ssot_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_052"));
    assert!(bootstrap.contains("migration_052"));
    assert!(bootstrap.contains("MIGRATION_052_VERSION"));
    assert!(bootstrap.contains("SQL_052_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m052.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/052_pg_session_artifacts_ssot_marker.sql").exists()
    );
}

#[test]
fn spec027_pg_auth_no_kv_read_fallback_phase40() {
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(identity.contains("kv_auth_reads_enabled"));
    assert!(identity.contains("kv_auth_writes_enabled"));
    assert!(
        identity.contains("if policy.pg_primary"),
        "identity must branch on pg_primary — no silent KV fallback when pool exists"
    );
    let session = read_crate_src("src/services/session_storage.rs");
    assert!(
        session.contains("if policy.pg_primary"),
        "session must branch on pg_primary"
    );
}

#[test]
fn spec027_pg_auth_test_harness_phase41() {
    let memory = read_crate_src("src/state/memory.rs");
    assert!(memory.contains("test_state_with_pg_pool"));
    let isolation = read_crate_src("src/services/tenant_isolation.rs");
    assert!(isolation.contains("with_optional_pg_rls"));
    assert!(std::path::Path::new("tests/spec027_pg_auth_e2e.rs").exists());
}

#[test]
fn spec027_handler_rls_wiring_phase42() {
    let pdf_lineage = read_crate_src("src/services/pdf_lineage.rs");
    assert!(pdf_lineage.contains("acquire_optional_pg_connection"));
    assert!(pdf_lineage.contains("pdf_documents"));
    let detail = read_crate_src("src/handlers/documents/query/detail.rs");
    assert!(detail.contains("pdf_lineage::fetch_pdf_extraction_metadata"));
    let isolation = read_crate_src("src/services/tenant_isolation.rs");
    assert!(isolation.contains("acquire_optional_pg_connection"));
    assert!(isolation.contains("default_identity"));
}

#[test]
fn spec027_identity_pg_rls_envelope_phase43() {
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(identity.contains("acquire_optional_pg_connection"));
    assert!(identity.contains("ensure_anonymous_user_in_postgres"));
    let session = read_crate_src("src/services/session_storage.rs");
    assert!(session.contains("acquire_optional_pg_connection"));
    let bootstrap = read_crate_src("src/handlers/postgres_user_bootstrap.rs");
    assert!(bootstrap.contains("ensure_anonymous_user_in_postgres"));
}

#[test]
fn spec027_auth_secure_by_default_phase44() {
    let auth_config = read_crate_src("../edgequake-auth/src/config.rs");
    assert!(auth_config.contains("auth_enabled: true"));
    assert!(auth_config.contains("EDGEQUAKE_DEV_MODE"));
    assert!(auth_config.contains("resolve_auth_enabled_from_env"));
    let memory = read_crate_src("src/state/memory.rs");
    assert!(memory.contains("dev_mode: true"));
    let makefile = std::fs::read_to_string("../../../Makefile").expect("Makefile");
    assert!(makefile.contains("DEV_EDGEQUAKE_DEV_MODE"));
}

#[test]
fn spec027_migration_055_auth_secure_default_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_055"));
    assert!(bootstrap.contains("migration_055"));
    assert!(bootstrap.contains("MIGRATION_055_VERSION"));
    assert!(bootstrap.contains("SQL_055_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m055.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/055_auth_secure_by_default_marker.sql").exists()
    );
}

#[test]
fn spec027_auth_memory_store_phase55() {
    assert!(std::path::Path::new("src/services/auth_memory_store.rs").exists());
    assert!(!std::path::Path::new("src/services/auth_kv_store.rs").exists());
    let memory = read_crate_src("src/services/auth_memory_store.rs");
    assert!(memory.contains("AuthMemoryStore"));
    assert!(!memory.contains("kv_storage"));
    let storage_rt = read_crate_src("src/state/storage_runtime.rs");
    assert!(storage_rt.contains("auth_memory"));
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(identity.contains("auth_memory_store::"));
    assert!(identity.contains("\"in-memory\""));
    assert!(!identity.contains("auth_kv_store"));
    let session = read_crate_src("src/services/session_storage.rs");
    assert!(session.contains("auth_memory_store::"));
    assert!(!session.contains("auth_kv_store"));
    let oidc_pending = read_crate_src("src/services/oidc_pending.rs");
    assert!(oidc_pending.contains("auth_memory_store"));
    assert!(!oidc_pending.contains("kv_storage"));
    let services_mod = read_crate_src("src/services/mod.rs");
    assert!(services_mod.contains("pub mod auth_memory_store"));
    assert!(!services_mod.contains("auth_kv_store"));
}

#[test]
fn spec027_auth_kv_store_consolidated_phase45() {
    let memory = read_crate_src("src/services/auth_memory_store.rs");
    assert!(memory.contains("persist_user_record"));
    assert!(memory.contains("persist_refresh_token"));
    assert!(memory.contains("persist_api_key"));
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(identity.contains("identity_backend_label"));
    assert!(identity.contains("auth_memory_store::persist_user_record"));
    let session = read_crate_src("src/services/session_storage.rs");
    assert!(session.contains("auth_memory_store::persist_refresh_token"));
    let health = read_crate_src("src/handlers/health_types.rs");
    assert!(health.contains("auth_identity_ssot"));
    let auth_mod = read_crate_src("src/handlers/auth/mod.rs");
    assert!(
        auth_mod.contains("identity_storage::find_user_record_by_login"),
        "auth helpers must route through identity_storage"
    );
    assert!(
        !auth_mod.contains("auth_kv_store"),
        "handlers/auth must not reference auth_kv_store"
    );
}

#[test]
fn spec027_migration_056_auth_kv_store_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_056"));
    assert!(bootstrap.contains("migration_056"));
    assert!(bootstrap.contains("MIGRATION_056_VERSION"));
    assert!(bootstrap.contains("SQL_056_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m056.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/056_auth_kv_store_consolidated_marker.sql").exists()
    );
}

#[test]
fn spec027_health_schema_ops_phase46() {
    let health_schema = read_crate_src("src/services/health_schema.rs");
    assert!(health_schema.contains("_sqlx_migrations"));
    assert!(health_schema.contains("fetch_sqlx_migration_stats"));
    let health = read_crate_src("src/handlers/health.rs");
    assert!(health.contains("health_schema::fetch_sqlx_migration_stats"));
    let startup = read_crate_src("src/startup_security.rs");
    assert!(startup.contains("kv_identity_mirror"));
    assert!(startup.contains("deprecated"));
}

#[test]
fn spec027_migration_057_kv_mirror_deprecated_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_057"));
    assert!(bootstrap.contains("migration_057"));
    assert!(bootstrap.contains("MIGRATION_057_VERSION"));
    assert!(bootstrap.contains("SQL_057_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m057.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/057_kv_identity_mirror_deprecated_marker.sql")
            .exists()
    );
}

#[test]
fn spec027_identity_policy_ignores_kv_mirror_phase47() {
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(identity.contains("identity_policy_ignores_kv_mirror_when_pool_phase47"));
    assert!(
        identity.contains("kv_mirror: false"),
        "PG-primary must hard-disable KV mirror when pool exists"
    );
}

#[test]
fn spec027_migration_058_kv_mirror_ignored_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_058"));
    assert!(bootstrap.contains("migration_058"));
    assert!(bootstrap.contains("MIGRATION_058_VERSION"));
    assert!(bootstrap.contains("SQL_058_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m058.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/058_kv_mirror_ignored_with_pg_pool_marker.sql")
            .exists()
    );
    let health = read_crate_src("src/handlers/health_types.rs");
    assert!(health.contains("kv_identity_mirror_effective"));
}

#[test]
fn spec027_pg_only_auth_branch_phase48() {
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(
        identity.contains("if policy.pg_primary"),
        "identity uses explicit pg_primary branch"
    );
    let session = read_crate_src("src/services/session_storage.rs");
    assert!(
        session.contains("if policy.pg_primary"),
        "session uses explicit pg_primary branch"
    );
    assert!(
        !session.contains("policy.kv_auth_reads_enabled()"),
        "session should not branch on kv_auth_reads_enabled after phase 48"
    );
}

#[test]
fn spec027_migration_059_pg_only_auth_branch_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_059"));
    assert!(bootstrap.contains("migration_059"));
    assert!(bootstrap.contains("MIGRATION_059_VERSION"));
    assert!(bootstrap.contains("SQL_059_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m059.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/059_pg_only_auth_branch_ssot_marker.sql").exists()
    );
}

#[test]
fn spec027_oauth2_oidc_not_builtin_phase49() {
    let auth_config = read_crate_src("../edgequake-auth/src/config.rs");
    assert!(auth_config.contains("OAUTH2_OIDC_BUILTIN"));
    assert!(auth_config.contains("BUILTIN_AUTH_MECHANISMS"));
    assert!(auth_config.contains("EXTERNAL_SSO_PATTERN"));
    let health_types = read_crate_src("src/handlers/health_types.rs");
    assert!(health_types.contains("oauth2_oidc_builtin"));
    assert!(health_types.contains("auth_mechanisms"));
    assert!(health_types.contains("auth_kv_harness_active"));
    assert!(health_types.contains("external_sso_pattern"));
    let health = read_crate_src("src/handlers/health.rs");
    assert!(health.contains("resolved_auth_mechanisms"));
    assert!(health.contains("is_runtime_builtin"));
    let kv_store = read_crate_src("src/services/auth_memory_store.rs");
    assert!(kv_store.contains("persist_user_record"));
    assert!(
        !kv_store.contains("mirror_user_record"),
        "legacy mirror_user_record name removed"
    );
}

#[test]
fn spec027_migration_060_oauth_oidc_honesty_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_060"));
    assert!(bootstrap.contains("migration_060"));
    assert!(bootstrap.contains("MIGRATION_060_VERSION"));
    assert!(bootstrap.contains("SQL_060_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m060.rs").exists());
    assert!(std::path::Path::new(
        "../../migrations/060_oauth_oidc_honesty_auth_kv_quarantine_marker.sql"
    )
    .exists());
}

#[test]
fn spec027_user_management_isolated_from_auth_kv_phase50() {
    let user_mgmt = read_crate_src("src/handlers/auth/user_management.rs");
    assert!(
        !user_mgmt.contains("auth_kv_store"),
        "handlers must route identity through identity_storage"
    );
    assert!(user_mgmt.contains("identity_storage::list_user_records"));
    assert!(user_mgmt.contains("identity_storage::delete_user_record"));
    let services_mod = read_crate_src("src/services/mod.rs");
    assert!(services_mod.contains("pub mod auth_memory_store"));
    let identity = read_crate_src("src/services/identity_storage.rs");
    assert!(identity.contains("pub(crate) async fn list_user_records"));
    assert!(identity.contains("pub(crate) async fn delete_user_record"));
}

#[test]
fn spec027_migration_061_auth_kv_handler_isolation_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_061"));
    assert!(bootstrap.contains("migration_061"));
    assert!(bootstrap.contains("MIGRATION_061_VERSION"));
    assert!(bootstrap.contains("SQL_061_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m061.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/061_auth_kv_handler_isolation_marker.sql").exists()
    );
}

#[test]
fn spec027_auth_handlers_isolated_from_auth_kv_phase51() {
    let auth_mod = read_crate_src("src/handlers/auth/mod.rs");
    assert!(
        !auth_mod.contains("auth_kv_store"),
        "handlers/auth/mod.rs must not reference auth_kv_store"
    );
    assert!(auth_mod.contains("identity_storage::load_user_record"));
    assert!(auth_mod.contains("identity_storage::persist_user_record"));
    let user_mgmt = read_crate_src("src/handlers/auth/user_management.rs");
    assert!(!user_mgmt.contains("auth_kv_store"));
}

#[test]
fn spec027_migration_062_auth_mod_isolation_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_062"));
    assert!(bootstrap.contains("migration_062"));
    assert!(bootstrap.contains("MIGRATION_062_VERSION"));
    assert!(bootstrap.contains("SQL_062_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m062.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/062_auth_mod_identity_ssot_marker.sql").exists()
    );
}

#[test]
fn spec027_auth_memory_store_callers_only_phase55() {
    fn walk_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_rs(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_root = manifest.join("src");
    let mut rs_files = Vec::new();
    walk_rs(&src_root, &mut rs_files);

    let allowed_callers = [
        "src/services/auth_memory_store.rs",
        "src/services/identity_storage.rs",
        "src/services/session_storage.rs",
        "src/services/oidc_pending.rs",
        "src/services/mod.rs",
        "src/state/memory.rs",
        "src/state/postgres.rs",
        "src/state/storage_runtime.rs",
    ];

    let mut offenders = Vec::new();
    for path in rs_files {
        let rel = path
            .strip_prefix(&manifest)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        let references_memory =
            src.contains("auth_memory_store::") || src.contains("mod auth_memory_store");
        let references_kv_auth =
            src.contains("auth_kv_store::") || src.contains("mod auth_kv_store");
        if references_kv_auth {
            offenders.push(format!("{rel} (legacy auth_kv_store code)"));
        }
        if references_memory && !allowed_callers.iter().any(|a| *a == rel) {
            offenders.push(rel);
        }
    }
    assert!(
        offenders.is_empty(),
        "auth_memory_store must be referenced only from identity/session/oidc_pending (+ mod); found: {offenders:?}"
    );
}

#[test]
#[ignore = "superseded by spec027_auth_memory_store_callers_only_phase55"]
fn spec027_auth_kv_store_two_callers_only_phase52() {
    fn walk_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_rs(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_root = manifest.join("src");
    let mut rs_files = Vec::new();
    walk_rs(&src_root, &mut rs_files);

    let allowed_callers = [
        "src/services/auth_kv_store.rs",
        "src/services/identity_storage.rs",
        "src/services/session_storage.rs",
        "src/services/mod.rs",
    ];

    let mut offenders = Vec::new();
    for path in rs_files {
        let rel = path
            .strip_prefix(&manifest)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        let references_kv = src.contains("auth_kv_store::") || src.contains("mod auth_kv_store");
        if references_kv && !allowed_callers.iter().any(|a| *a == rel) {
            offenders.push(rel);
        }
    }
    assert!(
        offenders.is_empty(),
        "auth_kv_store must be referenced only from identity_storage + session_storage (+ mod); found: {offenders:?}"
    );
}

#[test]
fn spec027_oauth2_oidc_builtin_wiring_phase54() {
    let routes = read_crate_src("src/routes.rs");
    assert!(routes.contains("/auth/oidc/login"));
    assert!(routes.contains("/auth/oidc/callback"));
    assert!(routes.contains("handlers::oidc_login"));
    assert!(routes.contains("handlers::oidc_callback"));
    for line in routes.lines() {
        if line.contains(".route(") {
            let lower = line.to_lowercase();
            if lower.contains("oauth") && !lower.contains("oidc") {
                panic!("unexpected oauth route (non-oidc): {line}");
            }
        }
    }
    let cargo = read_crate_src("Cargo.toml");
    assert!(cargo.contains("openidconnect"));
    let oidc_flow = read_crate_src("src/services/oidc_flow.rs");
    assert!(oidc_flow.contains("PkceCodeChallenge"));
    let oidc_pending = read_crate_src("src/services/oidc_pending.rs");
    assert!(oidc_pending.contains("store_oidc_pending"));
    assert!(!oidc_pending.contains("kv_storage"));
    let oidc_config = read_crate_src("../edgequake-auth/src/oidc_config.rs");
    assert!(oidc_config.contains("EDGEQUAKE_OIDC_ENABLED"));
    assert!(oidc_config.contains("MECHANISM_OIDC"));
    let auth_config = read_crate_src("../edgequake-auth/src/config.rs");
    assert!(auth_config.contains("OAUTH2_OIDC_BUILTIN: bool = false"));
    let health = read_crate_src("src/handlers/health.rs");
    assert!(health.contains("resolved_auth_mechanisms"));
    assert!(health.contains("builtin-oidc"));
    let middleware = read_crate_src("src/middleware.rs");
    assert!(middleware.contains("/auth/oidc/login"));
    assert!(middleware.contains("/auth/oidc/callback"));
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_064"));
    assert!(bootstrap.contains("migration_064"));
    assert!(bootstrap.contains("MIGRATION_064_VERSION"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m064.rs").exists());
    assert!(std::path::Path::new("../../migrations/064_oauth_oidc_builtin_marker.sql").exists());
    let openapi_examples = read_crate_src("src/openapi_examples.rs");
    assert!(openapi_examples.contains("oauth2_oidc_builtin"));
    assert!(openapi_examples.contains("external_sso_pattern"));
    // v2 API uses same protected middleware as v1
    assert!(routes.contains("protected_api_auth"));
    assert!(routes.contains("/api/v2"));
}

#[test]
#[ignore = "superseded by spec027_oauth2_oidc_builtin_wiring_phase54 — OIDC routes are intentional"]
fn spec027_oauth2_oidc_no_protocol_routes_phase53() {
    let routes = read_crate_src("src/routes.rs");
    for line in routes.lines() {
        if line.contains(".route(") {
            let lower = line.to_lowercase();
            assert!(
                !lower.contains("oauth"),
                "no OAuth routes in routes.rs: {line}"
            );
            assert!(
                !lower.contains("oidc"),
                "no OIDC routes in routes.rs: {line}"
            );
            assert!(
                !lower.contains("openid"),
                "no OpenID routes in routes.rs: {line}"
            );
        }
    }
    let auth_config = read_crate_src("../edgequake-auth/src/config.rs");
    let mechanisms_line = auth_config
        .lines()
        .find(|l| l.contains("BUILTIN_AUTH_MECHANISMS") && l.contains("&["))
        .expect("BUILTIN_AUTH_MECHANISMS slice definition");
    assert!(mechanisms_line.contains("jwt_password"));
    assert!(mechanisms_line.contains("api_key"));
    let mech_lower = mechanisms_line.to_lowercase();
    assert!(
        !mech_lower.contains("oauth") && !mech_lower.contains("oidc"),
        "mechanisms slice must not list oauth/oidc: {mechanisms_line}"
    );
    assert!(auth_config.contains("OAUTH2_OIDC_BUILTIN: bool = false"));
    let health = read_crate_src("src/handlers/health.rs");
    assert!(health.contains("BUILTIN_AUTH_MECHANISMS"));
    assert!(health.contains("OAUTH2_OIDC_BUILTIN"));
    assert!(health.contains("EXTERNAL_SSO_PATTERN"));
    let openapi_examples = read_crate_src("src/openapi_examples.rs");
    assert!(openapi_examples.contains("oauth2_oidc_builtin"));
    assert!(openapi_examples.contains("external_sso_pattern"));
}

#[test]
fn spec027_auth_session_api_keys_use_session_storage_phase52() {
    let session = read_crate_src("src/handlers/auth/session.rs");
    assert!(session.contains("session_storage::persist_refresh_token"));
    assert!(session.contains("session_storage::load_refresh_token"));
    assert!(session.contains("identity_storage::access_token_claims"));
    assert!(!session.contains("auth_kv_store"));
    let api_keys = read_crate_src("src/handlers/auth/api_keys.rs");
    assert!(api_keys.contains("session_storage::persist_api_key"));
    assert!(api_keys.contains("session_storage::list_api_keys_for_user"));
    assert!(api_keys.contains("session_storage::revoke_api_key"));
    assert!(!api_keys.contains("auth_kv_store"));
}

#[test]
fn spec027_migration_063_auth_service_layer_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_063"));
    assert!(bootstrap.contains("migration_063"));
    assert!(bootstrap.contains("MIGRATION_063_VERSION"));
    assert!(bootstrap.contains("SQL_063_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m063.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/063_auth_service_layer_ssot_marker.sql").exists()
    );
}

#[test]
fn spec027_migration_054_identity_pg_rls_envelope_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_054"));
    assert!(bootstrap.contains("migration_054"));
    assert!(bootstrap.contains("MIGRATION_054_VERSION"));
    assert!(bootstrap.contains("SQL_054_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m054.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/054_identity_pg_rls_envelope_marker.sql").exists()
    );
}

#[test]
fn spec027_migration_053_pg_auth_kv_reads_removed_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_053"));
    assert!(bootstrap.contains("migration_053"));
    assert!(bootstrap.contains("MIGRATION_053_VERSION"));
    assert!(bootstrap.contains("SQL_053_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m053.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/053_pg_auth_kv_reads_removed_marker.sql").exists()
    );
}

#[test]
fn spec027_migration_049_membership_ssot_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_049"));
    assert!(bootstrap.contains("migration_049"));
    assert!(bootstrap.contains("MIGRATION_049_VERSION"));
    assert!(bootstrap.contains("SQL_049_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m049.rs").exists());
    assert!(
        std::path::Path::new("../../migrations/049_membership_identity_ssot_marker.sql").exists()
    );
}

#[test]
fn spec027_sec010_constant_time_env_api_keys() {
    let validation = read_crate_src("src/services/auth_validation.rs");
    assert!(validation.contains("constant_time_str_eq"));
    let middleware = read_crate_src("src/middleware.rs");
    assert!(middleware.contains("constant_time_str_eq"));
}

#[test]
fn spec027_migration_048_identity_ssot_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_048"));
    assert!(bootstrap.contains("migration_048"));
    assert!(bootstrap.contains("MIGRATION_048_VERSION"));
    assert!(bootstrap.contains("SQL_048_APPLY"));
    assert!(std::path::Path::new("src/state/migration_bootstrap/reconcile/m048.rs").exists());
    assert!(std::path::Path::new("../../migrations/048_auth_identity_ssot_marker.sql").exists());
}

#[test]
fn spec027_openapi_includes_ollama_emulation_paths() {
    let doc = ApiDoc::openapi();
    let paths = doc.paths.paths;
    for path in [
        "/api/version",
        "/api/tags",
        "/api/ps",
        "/api/chat",
        "/api/generate",
    ] {
        assert!(
            paths.contains_key(path),
            "OpenAPI missing Ollama path: {path}"
        );
    }
    let openapi_src = read_crate_src("src/openapi.rs");
    assert!(openapi_src.contains("handlers::ollama_chat"));
}

#[test]
fn spec027_axum_routes_subset_of_openapi() {
    use edgequake_api::services::route_registry::openapi_required_paths;

    let routes_src = read_crate_src("src/routes.rs");
    let required = openapi_required_paths(&routes_src);
    let doc = ApiDoc::openapi();
    let openapi_paths: std::collections::HashSet<_> = doc.paths.paths.keys().cloned().collect();

    let mut missing = Vec::new();
    for path in &required {
        if !openapi_paths.contains(path) {
            missing.push(path.clone());
        }
    }

    assert!(
        missing.is_empty(),
        "routes registered in routes.rs but missing from OpenAPI (IMP-011): {missing:?}"
    );
}

#[test]
fn spec027_openapi_paths_subset_of_axum_routes() {
    use edgequake_api::services::route_registry::openapi_phantom_paths;

    let routes_src = read_crate_src("src/routes.rs");
    let doc = ApiDoc::openapi();
    let openapi_paths: Vec<String> = doc.paths.paths.keys().cloned().collect();
    let phantoms = openapi_phantom_paths(&routes_src, &openapi_paths);

    assert!(
        phantoms.is_empty(),
        "OpenAPI documents paths not registered in routes.rs (phantom spec): {phantoms:?}"
    );
}

#[test]
fn spec027_openapi_servers_version_and_websocket_enrichment() {
    let doc = ApiDoc::openapi();
    let servers = doc.servers.as_ref().expect("servers array configured");
    assert!(
        servers.iter().any(|s| s.url.contains("8080")),
        "local dev server URL missing"
    );
    assert!(
        servers.iter().any(|s| s.url == "/"),
        "relative server URL missing for Try it out"
    );
    assert_eq!(
        doc.info.version,
        env!("CARGO_PKG_VERSION"),
        "info.version must match crate version"
    );
    let ws = doc
        .paths
        .paths
        .get("/ws/pipeline/progress")
        .expect("/ws/pipeline/progress");
    let op = ws.get.as_ref().expect("GET ws");
    let ext = op.extensions.as_ref().expect("websocket x-extension");
    assert_eq!(
        ext.get("x-edgequake-transport"),
        Some(&serde_json::json!("websocket"))
    );
}

#[test]
fn spec027_openapi_bidirectional_parity_summary() {
    use edgequake_api::services::route_registry::{
        all_axum_route_paths, openapi_phantom_paths, openapi_required_paths,
    };

    let routes_src = read_crate_src("src/routes.rs");
    let doc = ApiDoc::openapi();
    let axum_count = all_axum_route_paths(&routes_src).len();
    let openapi_count = doc.paths.paths.len();
    let required = openapi_required_paths(&routes_src);
    let phantoms = openapi_phantom_paths(
        &routes_src,
        &doc.paths.paths.keys().cloned().collect::<Vec<_>>(),
    );

    assert!(
        axum_count >= 100,
        "expected >= 100 axum routes, got {axum_count}"
    );
    assert!(
        openapi_count >= required.len(),
        "openapi paths ({openapi_count}) should cover required routes ({})",
        required.len()
    );
    assert!(phantoms.is_empty());
}

#[test]
fn spec027_openapi_paths_match_handler_utoipa_annotations() {
    use edgequake_api::openapi_annotation_sync::{
        all_handler_utoipa_paths, openapi_paths_missing_annotations,
    };
    use std::path::PathBuf;

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let annotated = all_handler_utoipa_paths(&manifest.join("src/handlers"));
    let doc = ApiDoc::openapi();
    let openapi_paths: Vec<String> = doc.paths.paths.keys().cloned().collect();

    let missing = openapi_paths_missing_annotations(&openapi_paths, &annotated);
    assert!(
        missing.is_empty(),
        "OpenAPI paths without matching #[utoipa::path] in handlers: {missing:?}"
    );
}

#[test]
fn spec027_openapi_asyncapi_sidecar_at_root() {
    let doc = ApiDoc::openapi();
    let ext = doc
        .extensions
        .as_ref()
        .expect("root extensions after enrichment");
    let sidecar = ext
        .get("x-edgequake-asyncapi")
        .expect("x-edgequake-asyncapi sidecar");
    assert_eq!(sidecar["asyncapi"], "2.6.0");
    assert!(sidecar["channels"]["/ws/progress/{track_id}"].is_object());
}

#[test]
fn spec027_swagger_ui_persist_authorization_enabled() {
    let server = read_crate_src("src/server.rs");
    assert!(server.contains("persist_authorization(true)"));
}

#[test]
fn spec027_workspace_delete_uses_metadata_scan_ssot() {
    let crud = read_crate_src("src/handlers/workspaces/workspace_crud.rs");
    assert!(crud.contains("plan_workspace_document_kv_deletion"));
    assert!(!crud.contains("kv_storage.keys()"));
    let scan = read_crate_src("src/services/document_metadata_scan.rs");
    assert!(scan.contains("plan_workspace_document_kv_deletion"));
    assert!(scan.contains("keys_with_prefix"));
}

#[test]
fn spec027_error_responses_use_problem_json_content_type() {
    let err_mod = read_crate_src("src/error.rs");
    assert!(err_mod.contains("into_problem_json_response"));
    let pd = read_crate_src("src/error/problem_details.rs");
    assert!(pd.contains("application/problem+json"));
}

#[test]
fn spec027_bulk_ops_uses_workspace_document_ssot() {
    let bulk = read_crate_src("src/handlers/workspaces/bulk_ops/mod.rs");
    assert!(bulk.contains("load_workspace_documents"));
    let scan = read_crate_src("src/services/document_metadata_scan.rs");
    assert!(scan.contains("WorkspaceDocumentRecord"));
    assert!(scan.contains("load_workspace_metadata_entries_by_index"));
    assert!(scan.contains("wsdoc:"));
    assert!(scan.contains("load_workspace_documents"));
}

#[test]
fn spec027_workspace_document_index_ssot() {
    let idx = read_crate_src("src/services/workspace_document_index.rs");
    assert!(idx.contains("sync_workspace_document_index"));
    assert!(idx.contains("upsert_final_document_metadata"));
    assert!(idx.contains("upsert_metadata_kv_with_index"));
    assert!(idx.contains("wsdoc:"));
    let schema = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../edgequake-storage/src/kv_key_schema.rs"),
    )
    .expect("kv_key_schema.rs");
    assert!(schema.contains("workspace_doc_index"));
    assert!(schema.contains("parse_workspace_doc_index"));
}

#[test]
fn spec027_migration_047_startup_reconcile_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("reconcile_migration_047"));
    assert!(bootstrap.contains("migration_047"));
    assert!(bootstrap.contains("MIGRATION_047_VERSION"));
    let apply = include_str!("../../../migrations/support/047/apply.sql");
    assert!(apply.contains("wsdoc:"));
}

#[test]
fn spec027_metadata_write_paths_use_wsdoc_ssot() {
    let ssot = "upsert_metadata_kv_with_index";
    for (name, rel) in [
        ("ingest_admission", "src/services/ingest_admission.rs"),
        ("text_insert_content", "src/services/text_insert_content.rs"),
        ("scan", "src/handlers/documents/query/scan.rs"),
        ("pdf_upload", "src/handlers/pdf_upload/upload.rs"),
        ("progress_callback", "src/pipeline_progress_callback.rs"),
        ("reprocess", "src/handlers/documents/recovery/reprocess.rs"),
        ("stuck", "src/handlers/documents/recovery/stuck.rs"),
        ("bulk_ops", "src/handlers/workspaces/bulk_ops/mod.rs"),
        ("pdf_processing", "src/processor/pdf_processing.rs"),
        (
            "text_insert_prepare",
            "src/processor/text_insert/prepare.rs",
        ),
        ("multimodal_stage", "src/services/multimodal/stage.rs"),
        ("status_updates", "src/processor/status_updates.rs"),
    ] {
        let src = read_crate_src(rel);
        assert!(
            src.contains(ssot) || src.contains("upsert_metadata_with_wsdoc_index"),
            "{name} must route final metadata writes through wsdoc SSOT"
        );
    }
    // Staging admission writes staging keys only at HTTP admit; promote syncs final meta.
    let staging = read_crate_src("src/handlers/documents/upload/document_admission.rs");
    assert!(
        staging.contains("staging_doc_metadata"),
        "HTTP admit uses staging keys (index sync deferred to promote)"
    );
}

#[test]
fn spec027_document_metadata_key_uses_dry_helper() {
    use std::path::Path;

    fn walk_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_rs_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_root = manifest.join("src");
    let mut files = Vec::new();
    walk_rs_files(&src_root, &mut files);

    let forbidden = r#"format!("{}-metadata""#;
    for path in files {
        let rel = path.strip_prefix(&manifest).unwrap().display();
        let rel_str = rel.to_string();
        // Injection keys use a different namespace (`injection::…-metadata`).
        if rel_str.contains("injection_process.rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        assert!(
            !src.contains(forbidden),
            "{rel_str} must use metadata_key_for_document / kv_keys::doc_metadata, not raw format!"
        );
        if src.contains("metadata_key_for_document") || src.contains("kv_keys::doc_metadata") {
            // exercised paths delegate to SSOT
        }
    }
    let scan = read_crate_src("src/services/document_metadata_scan.rs");
    assert!(scan.contains("pub fn metadata_key_for_document"));
}

#[test]
fn spec027_user_management_no_full_kv_keys_scan() {
    let src = read_crate_src("src/handlers/auth/user_management.rs");
    assert!(
        src.contains("identity_storage::list_user_records"),
        "user list/admin guard must use identity_storage SSOT"
    );
    assert!(
        !src.contains("kv_storage.keys().await"),
        "user_management must not full-scan KV keys()"
    );
    assert!(src.contains("count_other_admin_users"));
}

#[test]
fn spec027_isolation_context_documents_dual_modes() {
    let iso = read_crate_src("src/services/isolation_context.rs");
    assert!(iso.contains("IsolationMode::Strict"));
    assert!(iso.contains("IsolationMode::LegacyDocumentAlias"));
    assert!(iso.contains("strict_mode_differs_from_legacy_for_uuid_stored_properties"));
}

#[test]
fn spec027_entity_name_normalize_ssot() {
    let svc = read_crate_src("src/services/entity_name_normalize.rs");
    assert!(svc.contains("pub fn normalize_entity_name"));
    let entities = read_crate_src("src/handlers/entities/mod.rs");
    assert!(entities.contains("entity_name_normalize"));
    let relationships = read_crate_src("src/handlers/relationships/helpers.rs");
    assert!(relationships.contains("entity_name_normalize"));
    let lineage = read_crate_src("src/handlers/lineage/normalize.rs");
    assert!(lineage.contains("entity_name_normalize"));
}

#[test]
fn spec027_document_task_cleanup_extracted() {
    let svc = read_crate_src("src/services/document_task_cleanup.rs");
    assert!(svc.contains("purge_workspace_tasks"));
    assert!(svc.contains("purge_persisted_tasks_for_document"));
    let storage = read_crate_src("src/handlers/documents/storage_helpers.rs");
    assert!(storage.contains("document_task_cleanup"));
    assert!(!storage.contains("async fn cancel_and_delete_task"));
}

#[test]
fn spec027_migration_bootstrap_ready_gate_wired() {
    let bootstrap = read_crate_src("src/state/migration_bootstrap/mod.rs");
    assert!(bootstrap.contains("is_ready_for_traffic"));
    assert!(bootstrap.contains("run_postgres_migrations"));
}

#[test]
fn spec027_storage_helpers_facade_delegates_to_services() {
    let storage = read_crate_src("src/handlers/documents/storage_helpers.rs");
    assert!(storage.contains("document_vector_storage"));
    assert!(storage.contains("document_reingest"));
    assert!(storage.contains("document_graph_cascade"));
    assert!(!storage.contains("async fn delete_document_for_reingestion"));
    assert!(!storage.contains("async fn get_workspace_vector_storage_strict"));
    let reingest = read_crate_src("src/services/document_reingest.rs");
    assert!(reingest.contains("resolve_workspace_duplicate_for_reingestion"));
    let vector = read_crate_src("src/services/document_vector_storage.rs");
    assert!(vector.contains("get_workspace_vector_storage_strict"));
}

#[test]
fn spec027_injection_handler_module_split() {
    let injection_mod = read_crate_src("src/handlers/injection/mod.rs");
    assert!(injection_mod.contains("pub mod crud"));
    assert!(injection_mod.contains("pub mod injection_file"));
    assert!(injection_mod.contains("mod helpers"));
    let crud = read_crate_src("src/handlers/injection/crud.rs");
    assert!(crud.contains("pub async fn put_injection"));
    assert!(crud.contains("pub async fn delete_injection"));
    let helpers = read_crate_src("src/handlers/injection/helpers.rs");
    assert!(helpers.contains("resolve_injection_context"));
    assert!(
        !std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/handlers/injection.rs")
            .exists(),
        "monolithic injection.rs must be removed after split"
    );
}

#[test]
fn spec027_openapi_path_ssot_build_validation() {
    use edgequake_api::openapi_path_ssot::{
        OPENAPI_GENERATED_HANDLER_COUNT, REGISTERED_HANDLER_COUNT,
    };
    assert_eq!(
        OPENAPI_GENERATED_HANDLER_COUNT, REGISTERED_HANDLER_COUNT,
        "build.rs handler scan must match openapi.rs paths() count"
    );
    let build_rs = read_crate_src("build.rs");
    assert!(build_rs.contains("OpenAPI path SSOT drift"));
    assert!(build_rs.contains("parse_openapi_registered_handlers"));
}

#[test]
fn spec027_openapi_all_schemas_have_examples() {
    use edgequake_api::openapi_enrichment::apply_openapi_enrichment;
    use edgequake_api::openapi_examples::count_schemas_with_examples;

    let mut doc = ApiDoc::openapi();
    apply_openapi_enrichment(&mut doc);
    let (with, total) = count_schemas_with_examples(&doc);
    assert!(total >= 100, "expected >= 100 schemas, got {total}");
    assert_eq!(
        with, total,
        "every OpenAPI schema must have an example (A++): with={with}, total={total}"
    );
}

#[test]
fn spec027_utoipa_version_pinned() {
    assert_eq!(env!("UTOIPA_PIN_VERSION"), "5.4.0");
    let workspace_toml = read_crate_src("../../Cargo.toml");
    assert!(
        workspace_toml.contains("utoipa = { version = \"=5.4.0\""),
        "workspace Cargo.toml must pin utoipa =5.4.0 (OAS-010)"
    );
}

#[test]
fn spec027_patch_user_operation_documented() {
    let doc = ApiDoc::openapi();
    let users = doc
        .paths
        .paths
        .get("/api/v1/users/{user_id}")
        .expect("/api/v1/users/{{user_id}}");
    assert!(
        users.patch.is_some(),
        "OAS-011: PATCH /api/v1/users/{{user_id}} must be documented"
    );
    let user_mgmt = read_crate_src("src/handlers/auth/user_management.rs");
    assert!(user_mgmt.contains("patch,"));
    assert!(user_mgmt.contains("pub async fn update_user"));
}

#[test]
fn spec027_webui_openapi_codegen_script_exists() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let script = repo_root.join("edgequake_webui/scripts/codegen-openapi.sh");
    assert!(
        script.exists(),
        "OAS-009: edgequake_webui/scripts/codegen-openapi.sh must exist"
    );
    let pkg = std::fs::read_to_string(repo_root.join("edgequake_webui/package.json"))
        .expect("package.json");
    assert!(
        pkg.contains("codegen:api"),
        "package.json must define codegen:api script"
    );
}

#[test]
fn spec027_openapi_snapshot_committed_for_codegen() {
    let snapshot = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../edgequake_webui/openapi/openapi.snapshot.json");
    assert!(
        snapshot.exists(),
        "committed openapi snapshot required for offline codegen (OAS-009)"
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&snapshot).expect("read snapshot"))
            .expect("parse snapshot");
    assert_eq!(json["openapi"], "3.1.0");
    assert!(json["paths"].as_object().map(|p| p.len()).unwrap_or(0) >= 100);
}

/// Run with `cargo test -p edgequake-api spec027_write_openapi_snapshot -- --ignored --nocapture`
/// to refresh `edgequake_webui/openapi/openapi.snapshot.json`.
#[test]
#[ignore = "manual snapshot refresh for OAS-009"]
fn spec027_write_openapi_snapshot() {
    use edgequake_api::openapi_enrichment::apply_openapi_enrichment;
    use edgequake_api::openapi_security::apply_path_security;

    let mut doc = ApiDoc::openapi();
    apply_openapi_enrichment(&mut doc);
    apply_path_security(&mut doc);

    let json = serde_json::to_string_pretty(&doc).expect("serialize openapi");
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../edgequake_webui/openapi/openapi.snapshot.json");
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).expect("mkdir openapi");
    }
    std::fs::write(&out, json).expect("write snapshot");
    eprintln!("wrote {}", out.display());
}
