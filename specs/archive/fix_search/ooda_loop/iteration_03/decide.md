# OODA Loop 3 - Decide

## Decision

Implement keyword boost in reranking to improve precision for queries with specific model names.

### Implementation Plan

1. **Modify chunk reranking to include keyword matching**
   - Extract important keywords from query (numbers, model names)
   - Boost chunks that contain exact keyword matches
   - Formula: `final_score = 0.7 * vector_similarity + 0.3 * keyword_match_score`

2. **Where to implement**
   - File: `edgequake-query/src/chunk_retrieval.rs`
   - Function: `rerank_chunks_by_similarity`
   - Add keyword extraction and matching logic

### Expected Outcome
- Query "2008 ENVY" will boost chunks containing "2008" and "ENVY"
- Precision should improve for model-specific queries
- Recall should remain high (no filtering, just reranking)

### Alternative Approaches Considered

1. **BM25 hybrid search** - Too complex for this iteration
2. **Document-level filtering** - May hurt recall
3. **Entity disambiguation** - Requires reprocessing

### Chosen: Keyword-boosted reranking (simplest, lowest risk)

