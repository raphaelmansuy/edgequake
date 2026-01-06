# OODA Loop 6 - Observe: Similarity Thresholds

## Current Configuration

- `min_score`: 0.1 (default)
- Applied to vector search results in SOTA engine

## Score Distribution Analysis

Query: "prix" (local mode)

| Source Type | Count | Avg Score | Notes |
|-------------|-------|-----------|-------|
| Chunk | 4 | 1.000 | Vector search, high relevance |
| Entity | 20 | 0.164 | Mixed: some from vector, some from graph |
| Relationship | 18 | 0.000 | All from graph traversal |

### Observations

1. **Chunks**: Excellent precision with avg score 1.0
2. **Entities**: Low avg due to graph-traversed entities (score=0)
3. **Relationships**: All score=0 from graph traversal

### Why Relationships Have score=0

Relationships come from two sources:
1. **Vector search**: Would have similarity scores
2. **Graph traversal**: Discovered by following entity connections, no vector score

The current implementation uses graph traversal for relationships in local/global modes, which doesn't compute vector similarity.

## Decision

The current threshold (0.1) is appropriate:
- Chunks are well-filtered
- Low-score entities/relationships provide valuable graph context
- LLM synthesizes answer effectively from mixed sources

**No threshold changes needed.**

## Potential Future Improvement

Could add `source_score_origin` field to distinguish:
- "vector" - score from vector similarity
- "graph" - score from graph connectivity/degree
- "combined" - weighted combination
