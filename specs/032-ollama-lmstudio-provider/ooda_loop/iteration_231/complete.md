# OODA Loop Iteration 231 - COMPLETE

## Summary

Applied the same tenant isolation fix from chat.rs (OODA-231) to query.rs handlers.

## Changes Made

### 1. Added `get_workspace` helper function

Location: `query.rs` line 493

```rust
async fn get_workspace(
    state: &AppState,
    workspace_id: &str,
) -> Result<Option<edgequake_core::Workspace>, ApiError>
```

### 2. Fixed `execute_query` handler

Location: `query.rs` lines 129-145

- Fetch workspace before setting tenant_id
- Use `workspace.tenant_id` for data queries
- Fall back to header tenant_id if no workspace

### 3. Fixed `stream_query` handler

Location: `query.rs` lines 451-469

- Same pattern as execute_query
- Fetch workspace first, use workspace's tenant_id

## Before vs After

### Before (Bug)
```
Frontend → X-Tenant-ID: random-uuid → Query Handler → Graph Query
                                         ↓
                              Data ingested with workspace.tenant_id
                                         ↓
                                   0 RESULTS ❌
```

### After (Fixed)
```
Frontend → X-Tenant-ID: random-uuid → Query Handler → Fetch Workspace
                                                           ↓
                                    Use workspace.tenant_id for Graph Query
                                                           ↓
                                                    CORRECT RESULTS ✅
```

## Verification

```bash
$ cargo check --package edgequake-api
# ✅ Compiles clean

$ cargo test --package edgequake-api
# ✅ 30 passed; 0 failed

$ ./scripts/check_security_invariants.sh
# ✅ All security invariants passed
```

## Security Invariant Enforcement

Updated `check_security_invariants.sh` to detect unsafe tenant_id usage:
- Checks for `with_tenant_id(tenant_ctx.tenant_id)` pattern
- Passes when `data_tenant_id` (derived from workspace) is used

## Files Modified

1. `query.rs` - Added `get_workspace()` helper and fixed both handlers
2. `check_security_invariants.sh` - Updated tenant isolation check
