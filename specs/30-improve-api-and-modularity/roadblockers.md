# Roadblockers for OODA Loop Mission

## Roadblocker 1: Dead QueryEngine in edgequake-core (Iteration 24)

**Date**: 2026-01-07  
**Severity**: Medium  
**Status**: Documented for future refactor

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
- Add `edgequake-query` as dependency to `edgequake-core`
- Replace `query::QueryEngine` with `SOTAQueryEngine`
- Update initialization in `initialize()` method

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
