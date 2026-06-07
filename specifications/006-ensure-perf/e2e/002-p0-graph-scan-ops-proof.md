# SPEC-006 E2E Proof 002 — GraphScanOps Push-Down

**Covers:** TR-006-001, V-006-001 remediation  
**Tests:** `edgequake-storage/tests/graph_scan_ops_tests.rs`

## Assertions

1. `list_nodes_filtered` returns `total=250`, `items.len()=20` on 500 seeded nodes with `entity_type=PERSON` filter.
2. `find_nodes_by_source_prefixes` returns nodes matching `source_ids` without scanning unrelated tenants.
3. Empty edge list returns `total=0` without error.

## First-principles check

```text
API memory for list page  ∝  page_size  (not total graph nodes)
```

## Run

```bash
cargo test -p edgequake-storage graph_scan_ops
```

## Code is law

- Trait: `edgequake-storage/src/traits/graph_scan_ops.rs`
- Memory: `adapters/memory/graph.rs` (`impl GraphScanOps`)
- Postgres: `adapters/postgres/graph/scan_ops.rs`
