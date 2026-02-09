# OODA Iteration 05 - Observe

**Mission**: Optimize Knowledge Graph Display & UX
**Mission File**: `specs/001-optimize-graph-mission/mission.md`
**Date**: 2026-02-09

---

## Mission Re-read Check

- [x] Node Limit 500 ✅ (Frontend enforces)
- [ ] Node Limit 500 at Backend (Defense in depth)
- [x] Expand neighbors - Fix fallback lookup ✅
- [x] Labels visible ✅
- [x] Search camera focus ✅
- [x] Keyboard navigation ✅
- [x] Screen reader ✅

---

## Backend Node Limit Analysis

### Current State

**File**: `edgequake/crates/edgequake-api/src/handlers/graph_types.rs`

```rust
pub fn default_max_nodes() -> usize {
    100  // Default
}
```

**Documentation** (line 73):
> Maximum nodes to return (default: 100, max: 1000)

### Enforcement Points

| Endpoint | Handler | Limit | Status |
|----------|---------|-------|--------|
| GET /api/v1/graph | get_graph | No clamp | ❌ |
| GET /api/v1/graph/stream | stream_graph | No clamp | ❌ |

### Risk

Server accepts any max_nodes value from client:
- Malicious client could request 100,000 nodes
- Could cause performance issues
- Not a security vulnerability (just performance)

---

## Proposed Fix

Add max clamp in `GraphQueryParams`:

```rust
impl GraphQueryParams {
    /// Apply sanity limits to parameters
    pub fn validate(&mut self) {
        // WHY: Defense in depth - cap max_nodes even if frontend is bypassed
        self.max_nodes = self.max_nodes.clamp(1, 500);
        self.depth = self.depth.clamp(1, 5);
    }
}
```

---

## Alternative: Derive macro with validation

Use serde deserialize_with for automatic validation.

---

## Files to Modify

1. `graph_types.rs` - Add MAX_GRAPH_NODES const
2. `graph.rs` - Clamp in handlers

