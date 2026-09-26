//! SPEC-149 architecture gate for provider-driver imports in policy crates.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
struct ArchitectureAllowlist {
    allowlist: Vec<AllowedPath>,
}

#[derive(Deserialize)]
struct AllowedPath {
    path: String,
}

#[test]
fn policy_crates_have_no_new_unallowlisted_postgres_imports() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("storage crate must live under edgequake/crates");
    let manifest_path = repo_root.join("scripts/provider-access/architecture-allowlist.json");
    let manifest: ArchitectureAllowlist =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();

    let policy_roots = [
        "edgequake/crates/edgequake-storage-contracts/src",
        "edgequake/crates/edgequake-query/src",
        "edgequake/crates/edgequake-core/src",
    ];
    let mut violations = Vec::new();
    for root in policy_roots {
        collect_rust_files(&repo_root.join(root), &mut |file| {
            let relative = file.strip_prefix(repo_root).unwrap();
            let relative = relative.to_string_lossy().replace('\\', "/");
            let source = fs::read_to_string(file).unwrap();
            let imports_postgres_driver = source.contains("sqlx::") || source.contains("PgPool");
            if imports_postgres_driver && !is_allowlisted(&relative, &manifest.allowlist) {
                violations.push(relative);
            }
        });
    }

    assert!(
        violations.is_empty(),
        "new PostgreSQL driver imports in policy crates require an explicit \
         architecture allowlist entry with a removal step: {violations:#?}"
    );
}

fn is_allowlisted(path: &str, allowlist: &[AllowedPath]) -> bool {
    allowlist.iter().any(|allowed| {
        let prefix = allowed.path.trim_end_matches('/');
        path == prefix || path.starts_with(&format!("{prefix}/"))
    })
}

fn collect_rust_files(directory: &Path, visitor: &mut impl FnMut(&PathBuf)) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_files(&path, visitor);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            visitor(&path);
        }
    }
}
