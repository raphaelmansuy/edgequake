# OODA Loop 13 - Decide

## Decision: Add Enhanced Preset Integration Test

### Gap Identified

The e2e tests all use `BM25Reranker::new()` (minimal). None test the enhanced presets:

- `for_rag()`
- `for_semantic()`
- `new_enhanced()`

### Plan

Add a test that verifies enhanced presets work correctly with the query engine:

1. Create test using `for_rag()` preset
2. Verify stemming improves matching (query "running" matches doc "run")
3. Verify phrase boosting works

### Test Location

Add to `e2e_sota_engine.rs` in the `reranker_integration_tests` module.

### Expected Outcome

Test proves enhanced features work end-to-end.
