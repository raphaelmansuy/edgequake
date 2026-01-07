# OODA Loop 7 - Decide

## Decision: Implement Phrase Match Boosting

We will add optional phrase boosting to BM25Reranker.

### Implementation Plan

1. **Add phrase_boost field to BM25Reranker**
   - Default: 0.0 (disabled)
   - Recommended: 0.5-1.0 for phrase-sensitive queries

2. **Add compute_phrase_bonus method**
   - Input: query tokens, document tokens
   - Output: bonus score for adjacent pair matches
   - Algorithm: Count consecutive query term pairs found in document

3. **Integrate into score calculation**
   ```rust
   let base_score = compute_bm25_score(...);
   let phrase_bonus = self.compute_phrase_bonus(&query_tokens, &doc_tokens);
   let final_score = base_score + (self.phrase_boost * phrase_bonus);
   ```

4. **Add preset with phrase boosting**
   - `for_semantic()`: Includes phrase_boost = 0.5

5. **Add tests**
   - Verify phrase matching boosts correct documents
   - Verify non-phrase queries are unaffected

### Design Choices

- Additive bonus (not multiplicative) to avoid zero inflation
- Configurable boost factor for tuning
- Disabled by default for backward compatibility
