# SPEC-018 — Query propagation proof

**Status:** ✅ Proven  
**Date:** 2026-06-05

## Claim

`execute_query` merges `PropagationHeaders` from middleware with body `extra_headers` for LLM outbound calls.

## Evidence

```bash
cargo test -p edgequake-api --lib handlers::query::tests::test_query_success
rg 'merge_with' edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs
```
