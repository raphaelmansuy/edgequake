# P1 — Triple batch embed when keywords disabled

**Status:** ✅ Proven  
**Date:** 2026-06-03

## Problem

`compute_with_query_vec` reused `embed_one` for query/high/low when keyword extraction was off. Local mode chunk ranking used `[0.1; 1536]` for all levels instead of queued directional `[1,0,...]` vectors — **2 e2e_sota_engine tests failed**.

## Fix (first principles)

When high/low texts equal the query, call `embed(&[q,q,q])` once and skip parallel `embed_one` in `pipeline_prepare`. Restores MockProvider queue contract without extra latency when keywords are enabled.

## Evidence

```bash
cargo test -p edgequake-query --test e2e_sota_engine chunk_ranking
# 10 passed
```

Files: `sota_engine/mod.rs` (`compute_with_query_vec`), `query_entry/query_pipeline.rs` (`pipeline_prepare`).
