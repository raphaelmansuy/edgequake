# OODA Loop 5: Orient

## Parameter Preset Analysis

### Use Case: Short Documents (Titles, Tweets)

- **Problem**: Default `b=0.75` over-penalizes short docs
- **Solution**: Lower `b` to reduce length normalization
- **Recommended**: `k1=1.2, b=0.3, delta=0`

### Use Case: Long Documents (Academic Papers)

- **Problem**: Long docs unfairly penalized even with relevant content
- **Solution**: Enable BM25+ with `delta=1.0`
- **Recommended**: `k1=1.5, b=0.75, delta=1.0`

### Use Case: Technical Content (Code, APIs)

- **Problem**: Exact term matching important, less stemming needed
- **Solution**: Higher `k1` for term weight, disable stemming
- **Recommended**: `k1=2.0, b=0.5, delta=0`

### Use Case: RAG/Knowledge Graph (EdgeQuake Default)

- **Problem**: Mixed content lengths, semantic matching
- **Solution**: Balanced parameters with enhanced tokenization
- **Recommended**: `k1=1.5, b=0.75, delta=0.5` + stemming

## Implementation Approach

Add named presets as constructor methods:

```rust
BM25Reranker::for_short_docs()
BM25Reranker::for_long_docs()
BM25Reranker::for_technical()
BM25Reranker::for_rag()  // New default
```
