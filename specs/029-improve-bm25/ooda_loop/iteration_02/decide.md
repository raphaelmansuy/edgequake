# OODA Loop 2: Decide

## Decision Matrix

| Criterion | Weight | NFKD Only | +Stemming | +Stop Words |
|-----------|--------|-----------|-----------|-------------|
| Recall Improvement | 40% | +5% | +25% | +5% |
| Precision Impact | 20% | 0 | -5% | -2% |
| Performance | 15% | -1% | -5% | +2% |
| Maintenance | 15% | 0 | 0 | 0 |
| Test Breakage Risk | 10% | Low | Low | Low |

## Decisions

### D1: Implement Full TokenizerConfig
- Add `TokenizerConfig` struct with all three features
- Make each feature independently toggleable
- Default to `minimal()` for backward compatibility

### D2: Backward Compatibility Strategy
- `BM25Reranker::new()` → `TokenizerConfig::minimal()` (no stemming)
- `BM25Reranker::new_enhanced()` → `TokenizerConfig::enhanced()` (all features)
- Existing tests use `new()`, so they won't break

### D3: rerank() Method Update
- Check `tokenizer_config` flags to decide which tokenizer to use
- If any enhancement enabled → use `tokenize_with_config()`
- Otherwise → use static `tokenize()` (unchanged behavior)

### D4: Test Strategy
- Add 12 new tests for enhanced tokenizer
- Verify existing 35 BM25 tests still pass
- Add regression test for morphological recall improvement

## Risk Mitigation
- Static `tokenize()` preserved exactly as-is
- Config defaults ensure no behavioral change for existing callers
