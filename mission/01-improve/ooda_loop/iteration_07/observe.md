# Observe — Iteration 07

Date: 2026-04-10  
Mission file re-read: `mission/01-improve.md`

## Findings

`crates/edgequake-query/src/chunk_retrieval.rs:202` uses `partial_cmp().unwrap()` for f32 score sorting. If any score is NaN, this panics at runtime.

Additionally, `crates/edgequake-query/src/sota_engine/reranking.rs:22` uses `self.reranker.as_ref().unwrap()` which panics when no reranker is configured.
