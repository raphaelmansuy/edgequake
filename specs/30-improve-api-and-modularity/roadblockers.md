# Roadblockers for OODA Loop Mission

## Roadblocker 1: Dead QueryEngine in edgequake-core (Iteration 24)

**Date**: 2026-01-07  
**Severity**: Medium  
**Status**: Blocked by cyclic dependency; documented for architectural refactor

### Problem

The `edgequake-core/src/query.rs` file contains a `QueryEngine` (1071 lines) that is **NOT USED** by the production API. The API uses `SOTAQueryEngine` from `edgequake-query` instead.

However, the `EdgeQuake` orchestrator in `edgequake-core/src/orchestrator.rs` still references this old `QueryEngine`:

- Line 266: `query_engine: Option<Arc<crate::query::QueryEngine>>`
- Line 404: `crate::query::QueryEngine::new(...)`

### Evidence

1. **API uses SOTAQueryEngine**:

   ```rust
   // state.rs:110
   pub sota_engine: Arc<SOTAQueryEngine>,
   ```

2. **No external usage of edgequake_core::QueryEngine**:

   - `grep -r "edgequake_core::QueryEngine"` returns 0 matches
   - Only internal usage within orchestrator.rs

3. **Orchestrator still uses old engine**:
   - Examples and tests use `EdgeQuake` orchestrator
   - Orchestrator creates old `QueryEngine` on initialization

### Resolution Plan

**Phase 1**: Update `orchestrator.rs` to use `edgequake_query::SOTAQueryEngine`

Attempted in Iteration 25. Result: Introduced a Cargo cycle (`edgequake-core` ↔ `edgequake-query`). Core cannot depend on query crate because `edgequake-query` already depends on `edgequake-core`.

Blocked actions:
- Adding `edgequake-query` to `edgequake-core` dependencies creates a cyclic dependency.
- Replacing `query::QueryEngine` breaks the workspace graph.

Decision:
- Reverted orchestrator changes; added a guard test (`test_edgequake_query_uses_core_engine`) to assert current behavior.
- Proceed with an architectural extraction before engine swap.

**Phase 2**: Remove dead code

- Delete `edgequake-core/src/query.rs` (1071 lines)
- Remove `pub use query::QueryEngine` from `lib.rs`
- Update examples and tests

**Impact**: ~1071 lines of dead code removal

### Current Workaround

Leave the dead code in place until orchestrator refactor is complete. This is a larger change that requires:

1. Dependency management between crates
2. E2E test validation
3. Breaking API change documentation

### Architectural Refactor Plan (New)

To break the cycle cleanly:
- Extract a minimal `edgequake-query-api` (or `edgequake-common`) crate defining a `QueryService` trait and shared `QueryMode/QueryRequest/QueryResponse` types.
- Make `edgequake-core` depend on this trait crate (no concrete engine).
- Make `edgequake-query` implement `QueryService` and continue to depend on `edgequake-core` types as needed.
- Update `orchestrator.rs` to hold a boxed `dyn QueryService` and inject `SOTAQueryEngine` from `edgequake-api` composition.

This preserves API boundaries and eliminates the cycle while enabling removal of `edgequake-core/src/query.rs`.
