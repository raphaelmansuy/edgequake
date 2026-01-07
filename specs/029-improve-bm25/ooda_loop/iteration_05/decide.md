# OODA Loop 5 - Decide

## Decision: Implement Domain-Specific Parameter Presets

Based on the observation that BM25F is too invasive (requires structured input) and the
orient finding that parameter presets can provide similar benefits with minimal API changes,
we will:

### Chosen Approach

1. **Add 4 domain-specific preset constructors**:

   - `for_short_docs()`: k1=1.2, b=0.3 - optimized for tweets, titles
   - `for_long_docs()`: k1=1.5, b=0.75, delta=1.0 - BM25+ for papers
   - `for_technical()`: k1=2.0, b=0.5, no stemming - code/API docs
   - `for_rag()`: k1=1.5, b=0.75, delta=0.5 - balanced for RAG

2. **Add comprehensive tests** for each preset

### Why Not BM25F?

- Would require splitting input into fields (title, content, etc.)
- SOTA engine passes only `chunk.content` - no field separation
- Changes would ripple through API, pipeline, storage
- Parameter tuning gives 90% of benefit with 10% of effort

### Expected Outcome

Users can select appropriate BM25 configuration based on their use case without
understanding the underlying math. The presets encapsulate expert knowledge about
optimal parameters for different document types.
