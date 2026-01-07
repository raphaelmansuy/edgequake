# OODA Iteration 02 - Observe

**Date**: 2026-01-07
**Focus**: Use helpers in sota_engine.rs to reduce duplication

## Observations

### Current State After Iteration 01

- New `helpers.rs` module created with 6 helper functions and tests
- `sota_engine.rs` still contains duplicated patterns that can use these helpers
- The helpers are not yet being used - they're ready for integration

### Duplication Patterns in sota_engine.rs

1. **`extract_document_id`** - Called 6 times inline

   - Lines: 1164, 1474, 1543, 1681, 1730
   - Can use: `helpers::extract_document_id`

2. **Entity source tracking extraction** - Repeated 4 times (~30 lines each)

   - Lines: 1030-1052, 1328-1347, 1428-1447, 1777-1797
   - Can use: `helpers::build_entity_from_node`

3. **Relationship source tracking extraction** - Repeated 4 times (~20 lines each)

   - Lines: 1078-1109, 1085-1110, 1268-1299
   - Can use: `helpers::build_relationship_from_edge`

4. **Chunk building from vector results** - Repeated 5 times (~15 lines each)
   - Multiple locations in query_naive, query_global, query_mix
   - Can use: `helpers::build_chunk_from_result`

### Estimated Impact

| Pattern               | Occurrences | Lines/Each | Total Lines Saved |
| --------------------- | ----------- | ---------- | ----------------- |
| Entity building       | 4           | ~30        | ~120              |
| Relationship building | 4           | ~20        | ~80               |
| Chunk building        | 5           | ~15        | ~75               |
| **Total**             |             |            | **~275 lines**    |

## Next: Orient

→ Identify safe replacement points
→ Plan incremental integration
