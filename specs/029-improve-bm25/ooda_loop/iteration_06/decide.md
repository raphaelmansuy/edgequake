# OODA Loop 6 - Decide

## Decision: Add Performance Benchmark Tests

Rather than adding criterion benchmarks (which would require new dependencies and 
infrastructure), we'll add performance tests within the existing test framework.

### Chosen Approach

1. **test_performance_minimal_vs_enhanced_1000_docs**
   - Compare minimal (no stemming) vs enhanced (with stemming)
   - Assert enhanced is no more than 3x slower
   - 5 iterations with warmup for stability

2. **test_performance_scale_comparison**
   - Test reranking at 100, 500, 1000, 2000 documents
   - Assert near-linear scaling (6x cap for 4x documents)
   - Uses the `for_rag()` preset

3. **test_performance_presets_comparison**
   - Verify all 6 presets complete in reasonable time
   - Assert < 500ms for 500 documents each
   - Catches any accidentally expensive configurations

### Why Not Criterion?

- Existing test infrastructure is sufficient for regression detection
- These tests run in `cargo test` without extra setup
- Can still add criterion later if micro-benchmarks needed

### Expected Outcome

Automated performance regression detection in CI.
