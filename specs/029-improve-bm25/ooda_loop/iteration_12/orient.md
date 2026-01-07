# OODA Loop 12 - Orient

## Analysis: Doc Examples Value

### Why Doc Examples Matter

1. **Tested Documentation**: `cargo test --doc` compiles and runs examples
2. **API Contracts**: If example breaks, API changed unexpectedly
3. **Onboarding**: New developers see usage immediately in IDE hover
4. **Rust Convention**: Standard practice in well-maintained crates

### What Needs Examples

| Item | Priority | Reason |
|------|----------|--------|
| `BM25Reranker::new()` | High | Default constructor |
| `BM25Reranker::new_enhanced()` | High | Main enhanced entry point |
| `BM25Reranker::for_rag()` | High | Most common use case |
| `BM25Reranker::rerank()` | High | Core method |
| `with_k1()`, `with_b()` | Medium | Builder pattern |
| `TokenizerConfig` | Low | Internal type |

### Constraints

1. Doc tests require types to be public
2. Must import from crate root or full path
3. Examples should be minimal but complete

### Decision Framework

Add examples to:
1. All public constructors (6 presets + 2 base)
2. `rerank()` method
3. Builder methods (chained example)
