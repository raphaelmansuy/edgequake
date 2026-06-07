# E2E Proof — P0 Memory Graph Workspace Stats Parity

**Date:** 2026-06-02  
**Spec:** SPEC-017 / `007-edgequake-storage`  
**Scope:** `node_count_by_workspace`, `edge_count_by_workspace`, `distinct_node_type_count_by_workspace` on memory backend must match postgres semantics.

## Problem (pre-remediation)

`MemoryGraphStorage` relied on `GraphStorage` trait defaults that scanned **all** nodes/edges globally. Dashboard stats for workspace A included workspace B data when running in memory/test mode.

## Fix

Overrides in `edgequake/crates/edgequake-storage/src/adapters/memory/graph.rs` filter by `workspace_id` property on nodes and edges.

## Contract test

Shared fixture in `tests/support/graph_workspace_contract.rs` seeds:

| Workspace | Nodes | Edges | Distinct types |
|-----------|-------|-------|----------------|
| ws-a | 2 (person, org) | 1 | 2 |
| ws-b | 1 (person) | 0 | 1 |

Run:

```bash
cargo test -p edgequake-storage --test storage_backend_contract
cargo test -p edgequake-storage --test memory_graph_workspace_parity
```

With postgres (requires `POSTGRES_PASSWORD`):

```bash
cargo test -p edgequake-storage --test storage_backend_contract --features postgres
```

## Expected output

```
test memory_backend_workspace_graph_stats_contract ... ok
test memory_graph_workspace_scoped_counts ... ok
```

## Acceptance

`node_count_by_workspace(ws-a) == 2` and `edge_count_by_workspace(ws-b) == 0` on memory backend for identical fixture data — **verified**.
