//! Utoipa annotation ↔ OpenAPI document parity (SPEC-027 IMP-011 A+).
//!
//! Handlers declare canonical paths via `#[utoipa::path(path = "...")]`.
//! Generated `ApiDoc` paths must match that annotation set exactly.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Extract `path = "..."` literals from utoipa handler annotations in source text.
pub fn extract_utoipa_path_literals(src: &str) -> Vec<String> {
    const NEEDLE: &str = "path = \"";
    let mut paths = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = src[search_from..].find(NEEDLE) {
        let start = search_from + rel + NEEDLE.len();
        let rest = &src[start..];
        let Some(end) = rest.find('"') else {
            break;
        };
        paths.push(rest[..end].to_string());
        search_from = start + end + 1;
    }
    paths
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// All utoipa path literals under `src/handlers/` (deduplicated).
pub fn all_handler_utoipa_paths(handlers_root: &Path) -> HashSet<String> {
    let mut files = Vec::new();
    collect_rs_files(handlers_root, &mut files);

    let mut paths = HashSet::new();
    for file in files {
        let Ok(src) = std::fs::read_to_string(&file) else {
            continue;
        };
        for path in extract_utoipa_path_literals(&src) {
            paths.insert(path);
        }
    }
    paths
}

/// OpenAPI document paths missing a matching handler `#[utoipa::path]` annotation.
pub fn openapi_paths_missing_annotations(
    openapi_paths: &[String],
    annotated: &HashSet<String>,
) -> Vec<String> {
    let mut missing: Vec<String> = openapi_paths
        .iter()
        .filter(|path| !annotated.contains(*path))
        .cloned()
        .collect();
    missing.sort();
    missing
}

/// Handler annotations not present in the generated OpenAPI document (orphans).
pub fn orphan_handler_annotations(
    openapi_paths: &HashSet<String>,
    annotated: &HashSet<String>,
) -> Vec<String> {
    let mut orphans: Vec<String> = annotated
        .iter()
        .filter(|path| !openapi_paths.contains(*path))
        .cloned()
        .collect();
    orphans.sort();
    orphans
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extract_utoipa_path_literals_finds_paths() {
        let src = r#"
        #[utoipa::path(get, path = "/api/v1/foo", tag = "Test")]
        pub async fn foo() {}
        "#;
        let paths = extract_utoipa_path_literals(src);
        assert_eq!(paths, vec!["/api/v1/foo"]);
    }

    #[test]
    fn handler_tree_includes_health_paths() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let handlers = manifest.join("src/handlers");
        let paths = all_handler_utoipa_paths(&handlers);
        assert!(paths.contains("/health"));
        assert!(paths.contains("/api/v1/documents"));
    }
}
