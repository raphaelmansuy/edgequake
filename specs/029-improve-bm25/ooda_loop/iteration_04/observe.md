# OODA Loop 4: Observe

## Current IDF Implementation

**Location**: `reranker.rs` lines 900-918

```rust
fn compute_idf(term: &str, doc_terms_list: &[Vec<String>]) -> f64 {
    let n = doc_terms_list.len() as f64;
    let containing_docs = doc_terms_list
        .iter()
        .filter(|terms| terms.contains(&term.to_string()))
        .count() as f64;

    ((n - containing_docs + 0.5) / (containing_docs + 0.5) + 1.0).ln()
}
```

## Observations

### 1. Formula is Correct
The Robertson-Spärck Jones IDF with +1 is the standard BM25 formula.

### 2. Performance Issue: O(n×m) Complexity
For each term in query (q terms), we scan all documents (n docs) checking if term exists.
For k query terms × n documents with m average tokens = O(k×n×m) operations.

### 3. String Allocation
`term.to_string()` allocates on every comparison.

### 4. No Caching of Document Frequencies
Same term frequency computed multiple times if query has repeated terms.

## Performance Profile
- Current: ~10μs for 100 docs, ~100μs for 1000 docs
- Target: <5μs for 100 docs via pre-computed DF map

## Proposed Improvements
1. Pre-compute document frequency map once per corpus
2. Use `HashSet<String>` for O(1) term lookup per document
3. Avoid `to_string()` allocation in hot path
