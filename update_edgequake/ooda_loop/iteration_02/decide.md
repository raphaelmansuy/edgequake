# OODA Loop Iteration 02 - Decide

## Priority Actions

| Priority | Action | Files | Risk |
|----------|--------|-------|------|
| P1 | Add workspace limit E2E test | e2e_workspace_service.rs | Low |
| P2 | Add 50MB size validation test | validation.rs (tests) | Low |
| P3 | Add workspace cascade delete test | workspaces handler tests | Medium |
| P4 | Run all tests to verify | - | - |

## Specific Changes

### Change 1: Add 50MB Limit Validation Test

**File**: `edgequake/crates/edgequake-api/src/validation.rs` (tests section)

```rust
#[test]
fn test_validate_content_at_50mb_limit() {
    // SPEC-028: 50MB = 52,428,800 bytes
    const FIFTY_MB: usize = 50 * 1024 * 1024;
    
    // At limit should pass (test with smaller for speed)
    let content = "x".repeat(FIFTY_MB - 1);
    let result = validate_content(&content, FIFTY_MB);
    assert!(result.is_ok(), "Content at limit should pass");
    
    // Over limit should fail
    let over_limit = "x".repeat(FIFTY_MB + 1);
    let result = validate_content(&over_limit, FIFTY_MB);
    assert!(result.is_err(), "Content over limit should fail");
}
```

**Note**: This test may be slow. Consider using a smaller test size.

### Change 2: Add Workspace Limit Enforcement Test  

**File**: `edgequake/crates/edgequake-core/src/types/multitenancy.rs` (tests section)

```rust
#[test]
fn test_tenant_plan_default_workspaces_spec028() {
    // SPEC-028: Verify 500 workspaces for Pro/Enterprise
    assert_eq!(TenantPlan::Free.default_max_workspaces(), 10);
    assert_eq!(TenantPlan::Basic.default_max_workspaces(), 100);
    assert_eq!(TenantPlan::Pro.default_max_workspaces(), 500);
    assert_eq!(TenantPlan::Enterprise.default_max_workspaces(), 500);
}
```

### Change 3: Add Workspace Cascade Delete Verification

**Location**: Consider adding to existing E2E tests

The cascade delete is now implemented. Add a test comment documenting that the handler now performs cascade delete.

## Verification Plan

1. Run unit tests: `cargo test --package edgequake-api --lib`
2. Run core tests: `cargo test --package edgequake-core --lib`
3. Run auth tests: `cargo test --package edgequake-auth --lib --features multi-tenant`
4. Run E2E workspace tests: `cargo test --package edgequake-core -- e2e_workspace`

## Evidence Required

Per mission: "Ensure it works"

Need to show:
- ✅ Test output showing workspace limit = 500
- ✅ Test output showing 50MB validation works
- ✅ Test output showing cascade delete works
