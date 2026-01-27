# OODA Iteration 02 - Orient

**Date**: 2026-01-07
**Focus**: Strategic assessment of helper integration opportunities

## Analysis

### Pattern Replacement Strategy

| Pattern Type          | Instances | Lines per Instance | Total Lines | Replacement                      |
| --------------------- | --------- | ------------------ | ----------- | -------------------------------- |
| Chunk building        | 5         | ~15                | ~75         | `build_chunk_from_result()`      |
| Entity building       | 4         | ~35                | ~140        | `build_entity_from_node()`       |
| Relationship (graph)  | 3         | ~25                | ~75         | `build_relationship_from_edge()` |
| Relationship (vector) | 1         | ~25                | ~25         | Keep as-is (different semantics) |

**Expected Reduction**: ~290 lines (additional reduction from import cleanup)

### Risk Assessment

1. **Low Risk**: Chunk and entity patterns are identical - direct substitution
2. **Medium Risk**: Relationship patterns differ slightly (graph vs vector)
   - Graph-based: Uses `edge.properties`
   - Vector-based: Uses `result.metadata` with additional fields

### Decision Matrix

- ✅ Replace all chunk patterns (5/5)
- ✅ Replace all entity patterns (4/4)
- ✅ Replace graph-based relationship patterns (3/4)
- ❌ Keep vector-based relationship pattern (1/4) - semantically different

## Priority

High - This directly reduces sota_engine.rs from 2,004 to ~1,640 lines (18% reduction)
