//! SPEC-111 / SPEC-150 — checksum repair wiring must stay coherent.
//!
//! Catches the class of failure where:
//! - repair modules invent private allow helpers that ignore the shared helper
//! - Makefiles reintroduce a parallel integer allowlist (manifest is SSOT)
//! - fossils 001/019 are missing from the manifest

#[test]
fn makefile_must_not_reintroduce_parallel_allowlist() {
    let makefile = std::fs::read_to_string("../../../Makefile").expect("Makefile");
    assert!(
        !makefile
            .lines()
            .any(|l| l.starts_with("KNOWN_CHECKSUM_REPAIR_VERSIONS")),
        "Makefile must not define KNOWN_CHECKSUM_REPAIR_VERSIONS — \
         SPEC-150 SSOT is edgequake/migrations/manifest.toml"
    );
    // VISIBLE_MIGRATE_STEP may still mention the env for emergency docs, but
    // should not hard-code the version list as an assignment.
    let assign_count = makefile
        .lines()
        .filter(|l| {
            l.trim_start().starts_with("KNOWN_CHECKSUM_REPAIR_VERSIONS")
                || l.contains("KNOWN_CHECKSUM_REPAIR_VERSIONS :=")
                || l.contains("KNOWN_CHECKSUM_REPAIR_VERSIONS:=")
        })
        .count();
    assert_eq!(assign_count, 0);
}

#[test]
fn rust_must_not_reintroduce_integer_array_allowlist() {
    let rust_src = include_str!("../src/state/migration_bootstrap/checksum_repair.rs");
    assert!(
        !rust_src.contains("KNOWN_CHECKSUM_REPAIR_VERSIONS: &[i64]"),
        "checksum_repair.rs must not hard-code KNOWN_CHECKSUM_REPAIR_VERSIONS array"
    );
    assert!(
        rust_src.contains("edgequake_migrate_manifest")
            || rust_src.contains("manifest::known_checksum_repair_versions"),
        "checksum_repair must delegate to edgequake-migrate-manifest"
    );
}

#[test]
fn manifest_is_ssot_and_includes_ancient_fossils() {
    let manifest = std::fs::read_to_string("../../../edgequake/migrations/manifest.toml")
        .expect("manifest.toml");
    assert!(
        manifest.contains("irreversible_drop = [125, 126, 131]"),
        "manifest must list irreversible drops"
    );
    assert!(
        manifest.contains("9e44513e1b22ab482a3703f3"),
        "manifest must include 001 v0.11.0 fossil"
    );
    assert!(
        manifest.contains("7b544306c5da16b05ec0607a"),
        "manifest must include 019 v0.10.6 fossil"
    );
    // Production fossils for the former Makefile list.
    for prefix in [
        "fa6cce9c4b088b5dbc850764",
        "d22cc6d8416c6a8ccf28542c",
        "331967467fdbeb58aeeb41ca",
        "da347384f34eb9db99d635f4",
        "67b73fd0f683dd5cae06213a",
        "461fa2a7c560513df711f954",
    ] {
        assert!(
            manifest.contains(prefix),
            "manifest missing fossil prefix {prefix}"
        );
    }
}

#[test]
fn all_repair_modules_call_shared_allow_helper() {
    let modules = [
        ("m071.rs", "MIGRATION_071_VERSION"),
        ("m078.rs", "MIGRATION_078_VERSION"),
        ("m118.rs", "MIGRATION_118_VERSION"),
        ("m121.rs", "MIGRATION_121_VERSION"),
        ("m125.rs", "MIGRATION_125_VERSION"),
        ("m131.rs", "MIGRATION_131_VERSION"),
    ];
    for (file, version_const) in modules {
        let path = format!("src/state/migration_bootstrap/reconcile/{file}");
        let src = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {path}"));
        assert!(
            src.contains(&format!("authorize_checksum_rewrite({version_const}")),
            "{file} must call authorize_checksum_rewrite({version_const}, …)"
        );
        assert!(
            !src.contains("fn allow_checksum_repair()"),
            "{file} must not define a private allow_checksum_repair()"
        );
    }
}

#[test]
fn immutability_spec_exists() {
    assert!(
        std::path::Path::new("../../../specs/111-issues/10-migration-immutability.md").exists(),
        "LAW-MIG doc required"
    );
    assert!(
        std::path::Path::new("../../../specs/150-reliable-migration-system/01-first-principles.md")
            .exists(),
        "SPEC-150 first principles required"
    );
}
