# SPEC-006 E2E Proof 001 — ResourceBudget SSOT

**Covers:** BR-006-012, RB-MEM-* catalog  
**Test:** `edgequake-core/src/resource/budget.rs::resource_budget_defaults_match_catalog`

## Assertion

`ResourceBudgetConfig::default()` matches [004_resource_budget_catalog.md](../004_resource_budget_catalog.md):

| Field | Expected |
|-------|----------|
| `max_graph_nodes` | 500 |
| `max_upload_bytes` | 52_428_800 (50 MiB) |
| `graph_scan_threshold_nodes` | 50_000 |
| `graph_query_timeout_secs` | 15 |

## Run

```bash
cargo test -p edgequake-core resource_budget_defaults_match_catalog
```

## Code is law

- `edgequake-core/src/resource/budget.rs`
- Re-exported via `edgequake_core::ResourceBudgetConfig`
