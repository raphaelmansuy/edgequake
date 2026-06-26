//! SPEC-021 — test provider override is env-gated (no accidental prod use).

#[test]
fn spec021_test_provider_override_requires_env_gate() {
    let src = include_str!("../src/safety_limits.rs");
    assert!(
        src.contains("EDGEQUAKE_ALLOW_TEST_PROVIDER_OVERRIDE"),
        "test provider override must be env-gated"
    );
    assert!(
        src.contains("test_provider_override()"),
        "create_safe_* must consult override hook"
    );
}
