# OODA Iteration 05 - Decide

**Mission**: Optimize Knowledge Graph Display & UX
**Mission File**: `specs/001-optimize-graph-mission/mission.md`
**Date**: 2026-02-09

---

## Decision: Implement Backend Parameter Validation

### Changes

1. **graph_types.rs**: Add constants and validated() methods
2. **graph.rs**: Call validated() in handlers

### Specific Code Changes

**graph_types.rs**:
- Add `MAX_GRAPH_NODES = 500`
- Add `MAX_GRAPH_DEPTH = 5`
- Add `GraphQueryParams::validated()`
- Add `GraphStreamQueryParams::validated()`

**graph.rs**:
- Call `params.validated()` in `get_graph()`
- Call `params.validated()` in `stream_graph()`

