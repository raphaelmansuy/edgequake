# OODA Loop 6 - Orient

## Performance Analysis

Based on the observe phase:

### Enhanced Tokenization Overhead

The stemming and Unicode normalization add processing time:
- Stemming: ~1-2μs per token (Porter2 algorithm)
- Unicode NFKD: ~0.5μs per token
- Stop word filtering: ~0.1μs per token (hash set lookup)

For 1000 documents × 50 tokens avg = 50,000 tokenizations per rerank.
Expected overhead: 50-100ms additional processing.

### DF Map Optimization (Loop 4)

The pre-computed DF map changed IDF from O(k×n) to O(n+k):
- Before: Count term occurrences for each query term across all docs
- After: Single pass to build frequency map, then O(1) lookups

### Testing Strategy

Add three benchmark tests:
1. **Minimal vs Enhanced**: Compare 1000-doc reranking time
2. **Scale behavior**: Verify near-linear scaling with document count
3. **Preset comparison**: Ensure all presets complete in reasonable time

### Acceptance Criteria

- Enhanced tokenization should be < 3x slower than minimal
- 2000 docs should be < 6x slower than 500 docs (accounting for overhead)
- All presets should complete 500-doc reranking in < 500ms
