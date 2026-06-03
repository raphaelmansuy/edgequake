# P1 — QUERY-DRY-004 retrieval unification

**Status:** ✅ Proven  
**Date:** 2026-06-03

## Claim

`query_modes.rs` delegates to `vector_queries::*_with_vector_storage`; default and workspace paths share hybrid semantics (local + global + naive).

## Evidence

- `query_modes.rs` ~246 LOC (was ~755)
- `spec017_workspace_vector_storage_matches_default`
- `test_default_and_workspace_hybrid_use_same_retrieval`
