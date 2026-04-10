# Orient — Iteration 07

Date: 2026-04-10  

## First Principles

- `f32::partial_cmp().unwrap()` panics on NaN. Scores can be NaN from division by zero or invalid embeddings.
- Replace with `f32::total_cmp()` (stable since Rust 1.62) which defines a total order including NaN.
- For reranker unwrap, add a guard returning empty results when no reranker configured.
