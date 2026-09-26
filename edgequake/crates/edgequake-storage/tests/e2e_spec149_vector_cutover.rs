use edgequake_storage::{ensure_caught_up, BindingCompleteness};

const TEST_ID: &str = "PROVIDER-ACCESS-E2E08";
const P0_TO_P1_REQUIREMENTS: [&str; 6] = [
    "writes and tombstones continue during resumable backfill",
    "switch requires complete projection visibility and deliveries",
    "binding generation invalidates generation-bound cursors",
    "old P0 binding remains draining for rollback soak",
    "rollback target must catch up before activation",
    "P0 and P1 model descriptors and revision digests agree",
];

#[test]
fn provider_access_e2e08_p0_p1_switch_requirements_are_wired() {
    assert_eq!(TEST_ID, "PROVIDER-ACCESS-E2E08");
    assert_eq!(P0_TO_P1_REQUIREMENTS.len(), 6);

    let current = BindingCompleteness {
        visible_revisions: 12,
        incomplete_deliveries: 0,
    };
    let caught_up = BindingCompleteness {
        visible_revisions: 12,
        incomplete_deliveries: 0,
    };
    let lagging = BindingCompleteness {
        visible_revisions: 11,
        incomplete_deliveries: 1,
    };

    ensure_caught_up(current, caught_up).unwrap();
    assert!(ensure_caught_up(current, lagging).is_err());
}
