//! SPEC-027 IMP-011 phase 2: OpenAPI post-processing (servers, version, WebSocket hints).
//!
//! Keeps `openapi.rs` focused on path registration; enrichment is applied in `SecurityAddon`.

use crate::openapi_asyncapi;

/// Apply non-security OpenAPI enrichments after generation.
pub fn apply_openapi_enrichment(openapi: &mut utoipa::openapi::OpenApi) {
    sync_crate_version(openapi);
    apply_servers(openapi);
    annotate_websocket_paths(openapi);
    annotate_v1_rpc_v2_migration_paths(openapi);
    apply_asyncapi_sidecar(openapi);
    crate::openapi_examples::apply_schema_examples(openapi);
}

fn sync_crate_version(openapi: &mut utoipa::openapi::OpenApi) {
    openapi.info.version = env!("CARGO_PKG_VERSION").to_string();
}

fn apply_servers(openapi: &mut utoipa::openapi::OpenApi) {
    use utoipa::openapi::ServerBuilder;
    openapi.servers = Some(vec![
        ServerBuilder::new()
            .url("http://localhost:8080")
            .description(Some(
                "Local development (default EdgeQuake backend port)".to_string(),
            ))
            .build(),
        ServerBuilder::new()
            .url("/")
            .description(Some(
                "Same host as Swagger UI (Try it out relative URL)".to_string(),
            ))
            .build(),
    ]);
}

/// Mark WebSocket upgrade paths for integrators (OAS 3.0 has no native WS type).
fn annotate_websocket_paths(openapi: &mut utoipa::openapi::OpenApi) {
    for (path, item) in openapi.paths.paths.iter_mut() {
        if !path.starts_with("/ws/") {
            continue;
        }
        macro_rules! tag_ws {
            ($op:ident) => {
                if let Some(op) = item.$op.as_mut() {
                    op.extensions = Some(websocket_extensions());
                }
            };
        }
        tag_ws!(get);
        tag_ws!(post);
    }
}

/// Embed minimal AsyncAPI 2.x channel map at document root (OAS-008 A++).
fn apply_asyncapi_sidecar(openapi: &mut utoipa::openapi::OpenApi) {
    let sidecar = openapi_asyncapi::asyncapi_sidecar();

    let mut ext = openapi.extensions.take().unwrap_or_default();
    ext.insert("x-edgequake-asyncapi".to_string(), sidecar);
    openapi.extensions = Some(ext);
}

fn websocket_extensions() -> utoipa::openapi::extensions::Extensions {
    use utoipa::openapi::extensions::Extensions;
    let mut ext = Extensions::default();
    ext.insert(
        "x-edgequake-transport".to_string(),
        serde_json::json!("websocket"),
    );
    ext.insert(
        "x-edgequake-upgrade".to_string(),
        serde_json::json!("RFC6455"),
    );
    ext
}

/// Tag v1 RPC operations with v2 Level 4 job equivalents (ascending-compat discovery).
fn annotate_v1_rpc_v2_migration_paths(openapi: &mut utoipa::openapi::OpenApi) {
    use crate::services::job_registry::V1_RPC_V2_JOB_TYPES;

    for (path, job_type) in V1_RPC_V2_JOB_TYPES {
        let Some(item) = openapi.paths.paths.get_mut(*path) else {
            continue;
        };
        let Some(op) = item.post.as_mut() else {
            continue;
        };
        let mut ext = op.extensions.take().unwrap_or_default();
        ext.insert(
            "x-edgequake-v2-job-type".to_string(),
            serde_json::json!(job_type),
        );
        ext.insert(
            "x-edgequake-v2-catalog".to_string(),
            serde_json::json!("/api/v2/workspaces/{workspace_id}/jobs/catalog"),
        );
        op.extensions = Some(ext);
    }
}

#[cfg(test)]
mod tests {
    use utoipa::OpenApi;

    use crate::openapi::ApiDoc;

    #[test]
    fn enrichment_sets_servers_and_crate_version() {
        let mut doc = ApiDoc::openapi();
        super::apply_openapi_enrichment(&mut doc);
        let servers = doc.servers.expect("servers");
        assert!(servers.iter().any(|s| s.url.contains("8080")));
        assert_eq!(doc.info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn websocket_paths_get_transport_extension() {
        let mut doc = ApiDoc::openapi();
        super::apply_openapi_enrichment(&mut doc);
        let ws = doc
            .paths
            .paths
            .get("/ws/pipeline/progress")
            .expect("/ws/pipeline/progress");
        let op = ws.get.as_ref().expect("GET ws");
        let ext = op.extensions.as_ref().expect("extensions");
        assert_eq!(
            ext.get("x-edgequake-transport"),
            Some(&serde_json::json!("websocket"))
        );
    }

    #[test]
    fn asyncapi_sidecar_embedded_at_root() {
        let mut doc = ApiDoc::openapi();
        super::apply_openapi_enrichment(&mut doc);
        let ext = doc.extensions.as_ref().expect("root extensions");
        let sidecar = ext
            .get("x-edgequake-asyncapi")
            .expect("x-edgequake-asyncapi");
        assert_eq!(sidecar["asyncapi"], "2.6.0");
        assert!(sidecar["channels"]["/ws/pipeline/progress"].is_object());
    }
}
