//! Canonical HTTP route paths for OpenAPI coverage checks (SPEC-027 IMP-011).
//!
//! Parses `routes.rs` at runtime so the Axum router remains the single source of truth
//! while contract tests can assert `routes ⊆ openapi`.

use std::collections::HashSet;

/// Paths intentionally excluded from OpenAPI (infra / non-REST).
const OPENAPI_EXEMPT: &[&str] = &[];

/// Extract `.route("...")` path literals from a routes.rs function body.
fn extract_route_literals(body: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = body[search_from..].find(".route(") {
        let start = search_from + rel + ".route(".len();
        let rest = &body[start..];
        let Some(open) = rest.find('"') else {
            break;
        };
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('"') else {
            break;
        };
        paths.push(after_open[..close].to_string());
        search_from = start + open + 1 + close + 1;
    }
    paths
}

/// Slice a function body from `routes.rs` source between `fn name` and the next top-level `fn`.
fn function_body<'a>(src: &'a str, fn_name: &str) -> Option<&'a str> {
    let needle = format!("fn {fn_name}");
    let start = src.find(&needle)?;
    let after = &src[start..];
    let open = after.find('{')? + 1;
    let body_start = start + open;
    let mut depth = 1usize;
    let mut i = body_start;
    let bytes = src.as_bytes();
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    if depth == 0 {
        Some(&src[body_start..i - 1])
    } else {
        None
    }
}

fn prefixed_paths(body: &str, prefix: &str) -> Vec<String> {
    extract_route_literals(body)
        .into_iter()
        .map(|path| format!("{prefix}{path}"))
        .collect()
}

/// All HTTP paths registered in `create_router` (deduplicated).
pub fn all_axum_route_paths(routes_rs: &str) -> Vec<String> {
    let mut paths = HashSet::new();

    if let Some(body) = function_body(routes_rs, "create_router") {
        for path in extract_route_literals(body) {
            paths.insert(path);
        }
    }
    if let Some(body) = function_body(routes_rs, "ollama_api_routes") {
        for path in prefixed_paths(body, "/api") {
            paths.insert(path);
        }
    }
    if let Some(body) = function_body(routes_rs, "api_v1_routes") {
        for path in prefixed_paths(body, "/api/v1") {
            paths.insert(path);
        }
    }
    if let Some(body) = function_body(routes_rs, "api_v2_routes") {
        for path in prefixed_paths(body, "/api/v2") {
            paths.insert(path);
        }
    }

    let mut sorted: Vec<_> = paths.into_iter().collect();
    sorted.sort();
    sorted
}

/// Paths that must appear in the OpenAPI document (routes minus exempt infra).
pub fn openapi_required_paths(routes_rs: &str) -> Vec<String> {
    let exempt: HashSet<&str> = OPENAPI_EXEMPT.iter().copied().collect();
    all_axum_route_paths(routes_rs)
        .into_iter()
        .filter(|path| !exempt.contains(path.as_str()))
        .collect()
}

/// OpenAPI paths with no matching Axum route (phantom spec entries — IMP-011 reverse).
pub fn openapi_phantom_paths(routes_rs: &str, openapi_paths: &[String]) -> Vec<String> {
    let router: HashSet<String> = all_axum_route_paths(routes_rs).into_iter().collect();
    let mut phantoms: Vec<String> = openapi_paths
        .iter()
        .filter(|path| !router.contains(*path))
        .cloned()
        .collect();
    phantoms.sort();
    phantoms
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn route_registry_parses_v1_prefix() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let src = std::fs::read_to_string(manifest.join("src/routes.rs")).unwrap();
        let paths = all_axum_route_paths(&src);
        assert!(paths.contains(&"/api/v1/documents".to_string()));
        assert!(paths.contains(&"/health".to_string()));
        assert!(paths.contains(&"/api/version".to_string()));
    }
}
