# OODA Loop 7 - Orient

## Analysis: Phrase/Proximity Boosting

### How It Works

Standard BM25 treats "knowledge graph" and "graph knowledge" the same.
Phrase boosting adds a bonus when query terms appear in the correct order:

```
Query: "knowledge graph"
Doc A: "...knowledge graph extraction..."  → base + phrase_boost
Doc B: "...graph of knowledge..."          → base only
```

### Implementation Options

1. **Simple Adjacent Pairs**
   - Check if consecutive query terms appear adjacent in document
   - Boost by a fixed factor (e.g., 1.5x)
   - Complexity: O(n × m) where n=query terms, m=doc length

2. **Proximity Window**
   - Score inversely proportional to distance between query terms
   - More nuanced but more complex
   - Complexity: O(n² × m)

3. **N-gram Index**
   - Pre-build bigram/trigram index
   - Fast lookup but increased memory
   - Complexity: O(n) with O(m) preprocessing

### Recommendation

Option 1 (Adjacent Pairs) is best for this iteration:
- Simple to implement
- Low overhead
- Meaningful quality improvement
- Easy to test

### Risk Assessment

- Low: Simple algorithm, easy to verify
- Medium: Slight performance impact (need to track positions)
- Mitigation: Add toggle to disable if needed

### Acceptance Criteria

1. Phrase match should boost ranking of documents with exact phrases
2. Performance should remain within acceptable bounds
3. Configurable boost factor
