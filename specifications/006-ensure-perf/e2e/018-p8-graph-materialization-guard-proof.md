# E2E Proof 018 — P8 Graph Materialization Guard (DRY/SOLID)

**Requirement:** NFR-006-001, TR-006-019  
**Status:** ✅ Verified 2026-06-06

---

## Claim

All graph materialization endpoints share one admission service (`graph_materialization.rs`):
`try_acquire_owned` (503 immediately, never queues) + query timeout. Server upload limit reads `AppState.resource_budget()`.

---

## Covered endpoints

| Handler | Guard |
|---------|-------|
| `GET /api/v1/graph` (BFS + popular) | `admit_graph_materialization` + `run_timed_graph_query` |
| `GET /api/v1/graph/labels/popular` | same |
| `GET /api/v1/graph/nodes/search` | same |
| `GET /api/v1/graph/stream` | same (SSE) |

---

## Evidence

### Static gates

```bash
./scripts/spec006_no_adhoc_resource_budget.sh
```

### Unit / integration tests

```bash
cargo test -p edgequake-api graph_materialization --lib
cargo test -p edgequake-api resource_safety_graph_materialization_busy_response
cargo test -p edgequake-api resource_safety_popular_labels_503_when_materialize_full
```

---

## Regression

Included in `make resource-proof` (P0–P8).
