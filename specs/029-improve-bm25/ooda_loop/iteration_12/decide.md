# OODA Loop 12 - Decide

## Decision: Add Comprehensive Doc Examples

### Plan

1. **Add struct-level example** to `BM25Reranker` showing basic usage
2. **Add examples to constructors**:
   - `new()` - minimal tokenization
   - `new_enhanced()` - full tokenization
   - `bm25_plus()` - delta parameter
   - `for_rag()` - knowledge graph use case
   - `for_technical()` - code documentation
   - `for_semantic()` - phrase boosting

3. **Add example to `rerank()`** showing full flow

4. **Add builder chain example** to one `with_*` method

### Expected Outcome

- `cargo test --doc` will run all examples
- IDE hover will show usage
- API documentation will be self-contained

### Risk Assessment

- Low risk: doc examples don't affect runtime
- Examples will fail to compile if API changes (good!)

### Effort Estimate

- ~30 minutes to add examples
- ~5 minutes to verify doc tests
