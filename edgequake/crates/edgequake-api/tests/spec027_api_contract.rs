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
        admin.matches("require_admin_request").count() >= 7,
        "all admin entrypoints must call require_admin_request"
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
    assert!(mat.contains("admit_graph_materialization"));
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
    assert!(jobs_get.get.is_some(), "GET workspace jobs must be registered");
    assert!(jobs_get.post.is_some(), "POST workspace jobs must be registered");
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
    assert!(migration.contains("json_with_v1_rpc_migration"));
    assert!(migration.contains("successor-version"));
    let rebuild = read_crate_src("src/handlers/workspaces/bulk_ops/rebuild_embeddings.rs");
    assert!(rebuild.contains("json_with_v1_rpc_migration"));
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
    let idx = recovery.find("struct ReanalyzeMultimodalResponse").expect("struct");
    let slice = &recovery[idx..recovery.len().min(idx + 400)];
    assert!(slice.contains("v2_migration"));
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

    assert!(axum_count >= 100, "expected >= 100 axum routes, got {axum_count}");
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
        ("text_insert_prepare", "src/processor/text_insert/prepare.rs"),
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
        src.contains("list_user_record_keys"),
        "user list/admin guard must use prefix scan SSOT"
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
