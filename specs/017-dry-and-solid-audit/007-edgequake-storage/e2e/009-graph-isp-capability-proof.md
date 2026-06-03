# P3 — GraphStorage ISP capability contract

**Date:** 2026-06-03  
**Status:** ✅ Proven (Phase 1 — marker traits + runtime bounds)

## Design (first principles)

Full method-level trait split was **evaluated and deferred** — see `traits/graph_isp.rs`:

- Call sites use `Arc<dyn GraphStorage>`; Rust cannot compose multiple non-auto trait objects.
- Marker capability traits document intent with zero adapter churn via blanket impls.

Phase 1 delivers ISP **at the type-boundary level**: callers can accept `&dyn GraphStorageReader` for read-only paths.

## Proof

```bash
cargo test -p edgequake-storage --test graph_isp_contract
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh
```

Tests:

| Test | Proves |
|------|--------|
| `memory_graph_storage_satisfies_isp_contract` | Compile-time: all caps on memory backend |
| `postgres_graph_storage_satisfies_isp_contract` | Same on postgres (when feature) |
| `read_cap_can_query_nodes_without_mutation_api` | `dyn GraphStorageReader` runtime bound |
| `analytics_cap_can_count_without_mutation_api` | `dyn GraphStorageAnalyticsCap` |
| `mutator_cap_can_upsert_via_narrow_bound` | `dyn GraphStorageMutator` |
| `graph_read_view_delegates_without_mutation_surface` | Phase 2a read facade |

## Phase 2a (2026-06-03)

See `e2e/012-graph-read-view-isp-phase2a-proof.md` — `GraphReadView` + query read-path migration.

## Gap (honest)

Methods still live on monolithic `GraphStorage`. **Phase 2b** (method migration + generic bounds) requires cross-crate refactor of remaining `Arc<dyn GraphStorage>` call sites (api/core/pipeline/reranking).
