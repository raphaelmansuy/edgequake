# OODA Loop 12: Decide - Implementation Approach

## Decision: Implement BM25Reranker + RRFReranker

### Rationale

1. **BM25Reranker** - Primary reranker for text relevance

   - Industry standard (used by Elasticsearch, Lucene, etc.)
   - No external dependencies
   - Well-understood parameters
   - Handles the "2008" vs "208" problem via IDF

2. **RRFReranker** - Secondary for combining signals
   - Combines vector similarity + BM25 scores
   - No tuning required
   - Formula: `score = Σ 1/(k + rank)`

### Parameters Chosen

| Parameter | Value | Reason                                        |
| --------- | ----- | --------------------------------------------- |
| k1        | 1.5   | Balance between TF saturation (1.2-2.0 range) |
| b         | 0.75  | Standard length normalization                 |
| RRF k     | 60    | Standard constant from literature             |

### Implementation Order

```
┌─────────────────────────────────────────────────────────────────────┐
│                    IMPLEMENTATION PHASES                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│   Phase 1: BM25Reranker                                             │
│   ├── Add struct BM25Reranker                                       │
│   ├── Implement Reranker trait                                      │
│   ├── Add unit tests                                                │
│   └── Benchmark vs MockReranker                                     │
│                                                                      │
│   Phase 2: RRFReranker                                              │
│   ├── Add struct RRFReranker                                        │
│   ├── Combine multiple ranking sources                              │
│   └── Add integration tests                                         │
│                                                                      │
│   Phase 3: Integration                                              │
│   ├── Update state.rs to use BM25Reranker                          │
│   ├── Run full test suite                                           │
│   └── Commit changes                                                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### File Changes

| File                            | Change                                   |
| ------------------------------- | ---------------------------------------- |
| `edgequake-llm/src/reranker.rs` | Add BM25Reranker, RRFReranker            |
| `edgequake-llm/src/lib.rs`      | Export new rerankers                     |
| `edgequake-api/src/state.rs`    | Switch from MockReranker to BM25Reranker |

### Success Criteria

1. **Precision**: "2008" query returns 2008 doc first (not 208)
2. **IDF Working**: Rare terms like "ENVY" score higher
3. **Tests Pass**: All 18 existing tests still pass
4. **Performance**: Reranking < 10ms for 100 documents

## Next Step

Act - Write the code
