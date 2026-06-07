# E2E Proof — GraphStorage ISP Phase 2b (method-level traits)

**Date:** 2026-06-03  
**Spec:** SPEC-017 P3-19c — Interface segregation for graph storage

## First-principle objective

Call sites that only read the graph must not depend on mutation/analytics surface area. Adapters implement **one `impl` block per sub-trait** (Rust E0119).

## Code

| Piece | Path |
|-------|------|
| Read / mutate / analytics ops | `traits/graph_read_ops.rs`, `graph_mutate_ops.rs`, `graph_analytics_ops.rs` |
| Composite + lifecycle | `traits/graph.rs` |
| Marker traits | `traits/graph_isp.rs` |
| Memory adapter | `adapters/memory/graph.rs` (4 impl blocks) |
| Postgres adapter | `adapters/postgres/graph/graph_storage_impl.rs` (4 impl blocks) |

## Run (code is law)

```bash
cd edgequake
cargo test -p edgequake-storage --test graph_isp_contract
# 6 passed — capability markers + read/mutate/analytics bounds
```

Included in `./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh`.

## Acceptance

- [x] `graph_isp_contract` green on memory backend
- [x] Postgres parity in CI (`postgres-integration.yml` runs `graph_isp_contract --features postgres`)
