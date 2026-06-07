# P0 — `QueryPipeline` single entry path

**Status:** ✅ Proven  
**Date:** 2026-06-03

## Claim

All non-stream SOTA entry points delegate to `run_query_pipeline` in `query_entry/query_pipeline.rs`.

## Commands

```bash
cargo test -p edgequake-query --test spec017_query_pipeline_contract  # 5/5
```

## Evidence

| File | LOC |
|------|-----|
| `query_pipeline.rs` | ~380 |
| `query_basic.rs` | ~96 |
| `query_stream.rs` | ~133 |
| `query_workspace.rs` | ~69 |
