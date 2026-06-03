# P3 — GraphStorage ISP Phase 2a (`GraphReadView`)

**Date:** 2026-06-03  
**Status:** ✅ Proven (read-only facade + query migration)

## Design (first principles)

Rust cannot express `Arc<dyn GraphStorageReader + GraphStorageAnalyticsCap>`. Full method-trait split would force every wiring site off `Arc<dyn GraphStorage>` — high churn, low immediate value.

**Phase 2a:** `GraphReadView<'a>` wraps `&dyn GraphStorage` and exposes **only read/analytics methods**. Mutation APIs are absent at compile time for query paths while keeping trait objects at boundaries.

## Implementation

| Artifact | Role |
|----------|------|
| `traits/graph_read_view.rs` | Read-only facade (delegates to inner storage) |
| `edgequake-query/sota_engine/mod.rs` | `graph_read()` helper on `SOTAQueryEngine` |
| `query_modes.rs`, `vector_queries.rs` | Batch fetch / popular / edge reads via `GraphReadView` |

## Proof

```bash
cargo test -p edgequake-storage --test graph_isp_contract
# → graph_read_view_delegates_without_mutation_surface

cargo check --workspace
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh
```

## Gap (honest)

- **Phase 2b:** Split method traits on `GraphStorage` itself; migrate api/core/pipeline off monolithic trait object.
- **Query:** `reranking.rs` still clones `Arc<dyn GraphStorage>` for async tasks (mutation not needed there either — future cleanup).
