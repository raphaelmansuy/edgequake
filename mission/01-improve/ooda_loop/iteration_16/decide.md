# OODA-16: Decide — Extract Row Types

## Plan

1. Create `crates/edgequake-core/src/workspace_row_types.rs` with:
   - `TenantRow`, `WorkspaceRow`, `MembershipRow` structs
   - Their `into_tenant()`, `into_workspace()`, `into_membership()` methods
   - `normalize_entity_types()` function
2. Update `workspace_service_impl.rs` to `use` the new module
3. Register the new module in `lib.rs`
4. Verify compilation + all 1147 tests pass
