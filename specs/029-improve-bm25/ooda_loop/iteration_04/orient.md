# OODA Loop 4: Orient

## Performance Analysis

### Original Algorithm Complexity
For each query term, `compute_idf` scans all documents:
- k query terms
- n documents  
- m average tokens per document

**Complexity**: O(k × n) per rerank call

### Optimized Algorithm
1. Build document frequency map once: O(n × m)
2. IDF lookup per term: O(1)
3. Total: O(n × m + k)

### Expected Improvement

| Corpus Size | Query Terms | Before | After | Speedup |
|-------------|-------------|--------|-------|---------|
| 100 docs | 5 terms | 500 ops | 105 ops | 4.8x |
| 1000 docs | 5 terms | 5000 ops | 1005 ops | 5.0x |
| 1000 docs | 10 terms | 10000 ops | 1010 ops | 9.9x |

## Technical Approach

1. **Precompute DF Map**: Single pass through corpus building `HashMap<term, doc_count>`
2. **HashSet for Dedup**: Use `HashSet<&String>` to count each term once per document
3. **Inline IDF Formula**: Mark `compute_idf_from_df` as `#[inline]` for zero-cost abstraction
