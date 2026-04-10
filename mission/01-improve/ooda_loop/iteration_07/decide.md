# Decide — Iteration 07

Date: 2026-04-10  

## Scope

1. Replace `partial_cmp().unwrap()` with `total_cmp()` in chunk_retrieval.rs
2. Add guard for missing reranker in reranking.rs
3. Search for all other `partial_cmp().unwrap()` in production code
