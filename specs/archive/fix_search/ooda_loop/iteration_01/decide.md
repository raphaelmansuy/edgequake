# OODA Loop 1 - Decide

## Decision: Fix source_chunk_ids linkage in Pipeline

### What We Will Do

1. **Modify `pipeline.rs`** after extraction to populate `source_chunk_ids` on entities and relationships
2. **Location**: After `extract_parallel()` call, before embedding generation
3. **Changes**:
   - Add chunk_id to each entity's `source_chunk_ids`
   - Set `source_chunk_id` on each relationship

### Expected Outcome

- Local mode will retrieve chunks linked to entities
- Global mode will retrieve chunks linked to relationships
- Hybrid mode will benefit from both
- LLM will receive actual content context → better answers

### Risk Assessment

| Risk                | Mitigation                           |
| ------------------- | ------------------------------------ |
| Regression in tests | Run full test suite after change     |
| Performance impact  | Negligible - simple field assignment |
| API compatibility   | No API changes, internal only        |

### Success Metrics

| Metric            | Before  | Target    |
| ----------------- | ------- | --------- |
| Local mode chunks | 0       | > 0       |
| Answer quality    | 1/11 OK | > 6/11 OK |
| Empty answers     | Many    | Few       |

### Implementation Plan

1. Edit `edgequake/crates/edgequake-pipeline/src/pipeline.rs`
2. After line ~293 (after extraction loop), add chunk_id population
3. Run tests to verify no regression
4. Re-run search tests to measure improvement
