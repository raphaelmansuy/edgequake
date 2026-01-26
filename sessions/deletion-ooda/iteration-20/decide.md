# OODA-20 Decide: Implement record_metrics_snapshot

## Decision

Implement the `record_metrics_snapshot()` function with:
1. New types in multitenancy.rs
2. Trait extension in workspace_service.rs
3. PostgreSQL implementation in workspace_service_impl.rs
4. In-memory stub for testing

## Implementation Plan

1. [ ] Add `MetricsTriggerType` enum to multitenancy.rs
2. [ ] Add `MetricsSnapshot` struct to multitenancy.rs
3. [ ] Export new types from types/mod.rs
4. [ ] Add `record_metrics_snapshot` to WorkspaceService trait
5. [ ] Implement PostgreSQL version in workspace_service_impl.rs
6. [ ] Add stub implementation to InMemoryWorkspaceService
7. [ ] Run tests to verify no regressions
8. [ ] Commit changes

## Files to Modify

1. `edgequake/crates/edgequake-core/src/types/multitenancy.rs`
2. `edgequake/crates/edgequake-core/src/types/mod.rs`
3. `edgequake/crates/edgequake-core/src/workspace_service.rs`
4. `edgequake/crates/edgequake-core/src/workspace_service_impl.rs`

## Success Criteria

- [ ] Types compile
- [ ] Trait compiles
- [ ] PostgreSQL impl compiles
- [ ] In-memory stub compiles
- [ ] All existing tests pass
