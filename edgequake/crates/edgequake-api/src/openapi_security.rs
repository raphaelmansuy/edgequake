//! SPEC-027 IMP-014: Centralized per-path OpenAPI security requirements (DRY).
//!
//! Global `security(bearer_auth + api_key)` in `openapi.rs` applies by default.
//! This module overrides specific paths so public endpoints show `security: []`
//! and tenant-scoped `/api/v1/*` paths document context headers.

use utoipa::openapi::path::PathItem;
use utoipa::openapi::security::SecurityRequirement;

/// Apply path-specific security overrides after OpenAPI generation.
pub fn apply_path_security(openapi: &mut utoipa::openapi::OpenApi) {
    for (path, path_item) in openapi.paths.paths.iter_mut() {
        let requirements = classify_path_security(path);
        apply_to_path_item(path_item, requirements);
    }
}

fn classify_path_security(path: &str) -> Option<Vec<SecurityRequirement>> {
    if is_public_path(path) {
        return Some(vec![]);
    }

    if path.contains("/admin/") {
        return Some(auth_only());
    }

    if path.starts_with("/api/v1/") {
        return Some(auth_with_tenant_context());
    }

    if path.starts_with("/api/v2/") {
        return Some(auth_with_tenant_context());
    }

    None
}

fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/ready" | "/live" | "/metrics" | "/version"
    ) || matches!(
        path,
        "/api/v1/auth/login"
            | "/api/v1/auth/refresh"
            | "/api/v1/auth/oidc/login"
            | "/api/v1/auth/oidc/callback"
    ) || path.starts_with("/api/v1/shared/")
}

fn auth_only() -> Vec<SecurityRequirement> {
    vec![
        SecurityRequirement::new("bearer_auth", Vec::<String>::new()),
        SecurityRequirement::new("api_key", Vec::<String>::new()),
    ]
}

fn auth_with_tenant_context() -> Vec<SecurityRequirement> {
    vec![
        SecurityRequirement::new("bearer_auth", Vec::<String>::new())
            .add("tenant_id", Vec::<String>::new())
            .add("workspace_id", Vec::<String>::new()),
        SecurityRequirement::new("api_key", Vec::<String>::new())
            .add("tenant_id", Vec::<String>::new())
            .add("workspace_id", Vec::<String>::new()),
    ]
}

fn apply_to_path_item(path_item: &mut PathItem, security: Option<Vec<SecurityRequirement>>) {
    let Some(requirements) = security else {
        return;
    };

    macro_rules! set_op {
        ($field:ident) => {
            if let Some(op) = path_item.$field.as_mut() {
                op.security = Some(requirements.clone());
            }
        };
    }

    set_op!(get);
    set_op!(put);
    set_op!(post);
    set_op!(delete);
    set_op!(options);
    set_op!(head);
    set_op!(patch);
    set_op!(trace);
}

#[cfg(test)]
mod tests {
    use utoipa::OpenApi;

    use crate::openapi::ApiDoc;

    #[test]
    fn public_health_has_empty_security() {
        let doc = ApiDoc::openapi();
        let health = doc.paths.paths.get("/health").expect("/health in spec");
        let op = health.get.as_ref().expect("GET /health");
        assert_eq!(op.security.as_ref().map(|s| s.len()), Some(0));
    }

    #[test]
    fn tenant_scoped_documents_require_context_headers() {
        let doc = ApiDoc::openapi();
        let list = doc
            .paths
            .paths
            .get("/api/v1/documents")
            .expect("/api/v1/documents");
        let get = list.get.as_ref().expect("GET");
        let security = get.security.as_ref().expect("security set");
        assert_eq!(security.len(), 2);
        let first = serde_json::to_value(&security[0]).unwrap();
        let obj = first.as_object().expect("security object");
        assert!(obj.contains_key("bearer_auth"));
        assert!(obj.contains_key("tenant_id"));
        assert!(obj.contains_key("workspace_id"));
    }

    #[test]
    fn admin_paths_use_auth_without_tenant_headers() {
        let doc = ApiDoc::openapi();
        let admin = doc
            .paths
            .paths
            .get("/api/v1/admin/config/defaults")
            .expect("admin path");
        let get = admin.get.as_ref().expect("GET");
        let security = get.security.as_ref().expect("security set");
        assert_eq!(security.len(), 2);
        let first = serde_json::to_value(&security[0]).unwrap();
        let obj = first.as_object().expect("security object");
        assert!(obj.contains_key("bearer_auth"));
        assert!(!obj.contains_key("tenant_id"));
    }
}
