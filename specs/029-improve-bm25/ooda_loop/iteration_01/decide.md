# OODA Loop Iteration 01: Decide

## Date: 2026-01-07

## Decision: Improve BM25 Tokenization Quality

### Decision Summary

**DO NOT** integrate tantivy for the reranking use case.

**DO** improve the existing BM25 implementation with:

1. Enhanced Unicode normalization (beyond French)
2. Optional stemming support (Porter2 algorithm)
3. Configurable stop word filtering
4. Better CJK character handling

### Rationale

1. **Reranking operates on small document sets** (~10-100 docs)
2. **Current BM25 is algorithmically correct** (passes 61 tests)
3. **Tantivy would add complexity** without proportional benefit
4. **Linguistic improvements** have higher impact for relevance

### Implementation Plan

```
Iteration 02-05: Unicode & Tokenization Improvements
  - Add comprehensive Unicode normalization (rust-unicode-normalization crate)
  - Implement Porter2 stemmer (rust-stemmers crate)
  - Add configurable stop word lists
  - Improve CJK handling

Iteration 06-10: BM25F Multi-Field Extension
  - Add field weights for title/body/metadata
  - Implement BM25F scoring formula
  - Add tests for field weighting

Iteration 11-15: Performance Optimization
  - Add query-time IDF caching
  - Pre-compute document statistics
  - Benchmark improvements

Iteration 16-20: Edge Cases & Testing
  - Add more edge case tests
  - Stress test with real documents
  - Validate PostgreSQL backend

Iteration 21-25: Integration & Validation
  - Integrate with sota_engine
  - Run E2E tests
  - Measure retrieval quality improvements

Iteration 26-30: Documentation & Finalization
  - Update API documentation
  - Write migration guide
  - Create summary report
```

### Acceptance Criteria

1. All existing 61 tests continue to pass
2. Stemming improves relevance for morphological variants
3. Unicode normalization handles >90% of Latin scripts
4. No performance regression (maintain <1s for 1000 docs)
5. Backward compatible API

### Dependencies to Add

```toml
# In Cargo.toml for edgequake-llm
rust-stemmers = "1.2"     # Porter2 stemmer
unicode-normalization = "0.1"  # Unicode NFD/NFC
```

### Files to Modify

| File                     | Change                            |
| ------------------------ | --------------------------------- |
| edgequake-llm/Cargo.toml | Add stemmer dependency            |
| reranker.rs:725-765      | Enhance tokenize() function       |
| reranker.rs:658          | Add stemmer field to BM25Reranker |
| reranker.rs tests        | Add stemming tests                |

### Non-Goals

- Full tantivy integration
- Persistent inverted index
- Primary retrieval replacement
- Breaking API changes

## Decision Approved

Proceed with Iteration 02: Implement enhanced tokenization.
