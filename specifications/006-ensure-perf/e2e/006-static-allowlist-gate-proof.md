# SPEC-006 E2E Proof 006 — Static Allowlist Gate

**Covers:** BR-006-021, G-006-05  
**Script:** `scripts/spec006_no_get_all_api.sh`

## Assertion

Every `get_all_nodes()` / `get_all_edges()` in `edgequake-api/src` must match an entry in:

`specifications/006-ensure-perf/support/get_all_allowlist.txt`

## Shrinking policy

Remove allowlist lines as handlers migrate to `GraphScanOps`:

| Milestone | Remove from allowlist |
|-----------|----------------------|
| P0 done | `entity_crud.rs`, `relationships/list.rs`, `graph_query/traversal.rs` |
| P1 | `documents/delete/*.rs`, `lineage/*.rs` |
| P2 | allowlist empty → gate fails on any `get_all_*` |

## Run

```bash
./scripts/spec006_no_get_all_api.sh
```
