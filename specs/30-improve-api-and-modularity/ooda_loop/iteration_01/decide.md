# OODA Iteration 01 - Decide

**Date**: 2026-01-07
**Focus**: Action plan for first refactoring batch

## Decision: Extract Mode Router from sota_engine.rs

**Rationale:**

1. Highest impact (removes ~500 lines from largest file)
2. Clear boundary (5 independent mode methods)
3. Well-tested (each mode has dedicated tests)
4. Low risk (pure refactoring, no behavior change)

## Extraction Plan

### Step 1: Create `mode_strategies.rs`

Extract these methods from `sota_engine.rs`:

| Method                  | Lines | Description                 |
| ----------------------- | ----- | --------------------------- |
| `query_local()`         | ~150  | Entity-centric search       |
| `query_global()`        | ~200  | Relationship-centric search |
| `query_hybrid()`        | ~80   | Combined local + global     |
| `query_mix()`           | ~80   | Weighted naive + graph      |
| `query_naive()`         | ~60   | Chunk-only search           |
| `fallback_to_popular()` | ~50   | Empty result fallback       |

**Total:** ~620 lines → new module

### Step 2: Create Trait for Mode Execution

```rust
// src/mode_strategies.rs

/// Strategy for executing a specific query mode.
pub trait ModeStrategy: Send + Sync {
    /// Execute the mode-specific retrieval.
    async fn execute(
        &self,
        keywords: &ExtractedKeywords,
        embeddings: &QueryEmbeddings,
        tenant_id: Option<String>,
        workspace_id: Option<String>,
    ) -> Result<QueryContext>;
}
```

### Step 3: Keep sota_engine.rs as Orchestrator

After extraction, `sota_engine.rs` becomes:

- Config + construction (~200 lines)
- Main query orchestration (~300 lines)
- Reranking coordination (~100 lines)
- Prompt building (~200 lines)
- Tests (~600 lines)

**New size:** ~1,400 lines (30% reduction)

## Test Strategy

1. **Before refactoring:** Run full test suite, record results
2. **After extraction:** Run same tests, verify identical results
3. **Add new tests:** Module-level tests for extracted code

## Commit Plan

```
feat(query): Extract mode strategies from sota_engine.rs

- Create mode_strategies.rs with ModeStrategy trait
- Move query_local, query_global, query_hybrid, query_mix, query_naive
- Add fallback_to_popular helper
- Update sota_engine.rs to use extracted module
- All 2,100+ tests pass
```

## Rollback Plan

If tests fail after extraction:

1. Revert to previous commit
2. Analyze which test failed
3. Fix extraction and retry

## Success Criteria

- [ ] All existing tests pass
- [ ] Clippy clean (0 warnings)
- [ ] rustfmt clean
- [ ] sota_engine.rs reduced by ~500 lines
- [ ] New module is self-contained and documented

## Next: Act

→ Implement the extraction
→ Run tests
→ Commit changes
