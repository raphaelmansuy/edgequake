# OODA Loop 14 - Decide

## Decision: Document Storage-Agnostic Design

### Assessment

BM25 is **already storage-agnostic** by design:
- Takes `Vec<String>` as input
- Returns `Vec<RerankResult>` as output
- No database access whatsoever

### Action Plan

1. Document this design decision in code comments
2. Verify the trait interface is storage-agnostic
3. No additional PostgreSQL-specific tests needed

### Rationale

The `Reranker` trait interface is:

```rust
async fn rerank(
    &self,
    query: &str,
    documents: &[String],
    top_n: Option<usize>,
) -> Result<Vec<RerankResult>>;
```

This interface guarantees:
- No database connection needed
- Pure in-memory operation
- Same behavior regardless of document source

### Expected Outcome

- Documented storage-agnostic design
- Confirmed no PostgreSQL-specific testing needed
