# O(n) Complexity & System Engineering Lens

**Spec:** 027-api-edgequake-audit  
**Cross-ref:** [006-rust-architecture-lens.md](./006-rust-architecture-lens.md) | [003-rest-design-lens.md](./003-rest-design-lens.md)

---

## Verdict: A++ (post phase 18)

All **P0/P1 HTTP hot paths** and **P2 cold-path HTTP handlers** now use batch graph/KV I/O or scoped metadata SSOT. Phase 18 closed merge batch reads and unified query-filter metadata loading.

**Honest A++ caveat:** Admin `entity_reconcile` (storage crate) still full-graph — not an HTTP hot path. Tenant-only queries without workspace UUID still traverse suffix scan inside `load_scoped_document_metadata_entries` (no tenant KV index — product constraint, not a bug).

---

## Request Budget Model (ASCII)

```
  Production HTTP (hot + cold handlers):
  ┌──────────────────────────────────────────────┐
  │ wsdoc prefix OR scoped metadata SSOT        │
  │ node_degrees_batch / get_edges_for_node_set │
  │ upsert_edges_batch on merge                 │
  │ get_incident_edges_batch in neighborhood    │
  └──────────────────────────────────────────────┘

  Accepted exceptions (documented, not hidden):
  ┌──────────────────────────────────────────────┐
  │ tenant-only query: suffix scan + tenant filter│
  │ admin entity_reconcile: storage full-graph   │
  └──────────────────────────────────────────────┘
```

---

## P0 — Production Scale Blockers (Historical → FIXED)

All P0 items **FIXED** phases 3–16. See phase 16 re-assessment.

---

## P1 — Scale Before 10k Documents (Historical → FIXED)

All P1 items **FIXED** phases 3–16.

---

## P2 — Cold-Path Exceptions (Post Phase 18)

| ID | Finding | Status | Evidence |
|----|---------|--------|----------|
| PERF-007 | merge_entities edge rewire | **FIXED** | `entity_merge.rs` — `get_edges_for_node_set` + `upsert_edges_batch` |
| PERF-008 | Graph search neighbor degrees | **FIXED** | phase 16 `node_degrees_batch` |
| PERF-009 | recover_stuck metadata | **FIXED** | scoped metadata SSOT |
| PERF-010 | Workspace stats chunks | **FIXED** | `load_workspace_metadata_values` |
| PERF-011 | Trait default N+1 | **MITIGATED** | Postgres overrides batch methods |
| PERF-012 | Admin reconcile full-graph | **ACCEPTED (cold)** | `entity_reconcile` in storage; admin-only |
| PERF-KV-002 | Query filter metadata load | **FIXED (SSOT)** | always `load_scoped_document_metadata_entries`; suffix when no wsdoc |
| PERF-CP-001 | Checkpoint cleanup | **FIXED** | phase 17 suffix + batch get |

**P2 closure rule:** HTTP handler cold paths use batch APIs or scoped SSOT. Storage-layer admin tools may remain O(corpus) with explicit ACCEPTED status.

---

## System Engineering Observations

### Observability ✅

- Request ID middleware (SPEC-018)
- Prometheus `/metrics`
- Structured tracing on auth failures

### Reliability ✅ (code-verified)

| Control | Layer | Evidence | Limit |
|---------|-------|----------|-------|
| Graph query timeout | API | `run_timed_graph_query` | No auto-retry |
| Materialization admission | API | `admit_graph_materialization` → 503 | Capacity bound |
| Health probe timeout | API | `COMPONENT_PING_TIMEOUT` 750ms | E2E: `/health` components |
| Task queue + retry | API | persisted tasks + retry route | Max retry cap |
| Pipeline circuit breaker | Pipeline | `CircuitBreakerOpen` → `retryable: true` | Single owner (DRY) |
| Checkpoint resume | Processor | save/load + startup cleanup | Orphan sweep on boot |

Contract: `spec027_reliability_graph_query_timeout_ssot` + E2E `spec027_health_reports_storage_component_probes`.

### Multi-tenancy ✅ (HTTP paths)

Workspace-scoped reads use wsdoc index; all metadata consumers delegate to `document_metadata_scan` SSOT.

### Resource limits ✅

- `DefaultBodyLimit` from resource budget
- Graph query `validated()` on traversal params

---

## Performance Acceptance Criteria (IMP)

| Endpoint | Target | Status |
|----------|--------|--------|
| `GET /graph/entities?page=100` | ≤3 DB RTTs | **MET** |
| `GET /documents` | scoped scan | **MET** |
| `GET /graph/nodes/search?include_neighbors=1` | batch degrees | **MET** |
| `POST /graph/entities/merge` | batch edge read/write | **MET** — phase 18 |
| Query with document filter | scoped metadata SSOT | **MET** — phase 18 |

---

## Code Re-assessment (phase 17)

Checkpoint suffix scan + merge batch writes. Reliability table added.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 30)

No O(n) changes. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 29)

graph_stream SSE uses GraphQueryRuntime timeout SSOT. No O(n) regressions. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 32)

No O(n) changes. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 31)

`get_document` ISP — no O(n) path change. Verdict **A++** unchanged. Contract 78+1.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 28)

Graph materialization admits via `GraphQueryRuntime` — timeout/budget SSOT preserved. No new O(n) regressions. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 27)

No O(n) changes. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 26)

No O(n) changes. Verdict **A++** unchanged. Lineage handlers ISP-only (no perf change).

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 25)

No O(n) changes. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 24)

No O(n) path changes. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 23)

No O(n) path changes. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 22)

No O(n) path changes. Verdict **A++** unchanged.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 37)

No O(n) changes. RLS SSOT is isolation-layer only. Verdict **A++** unchanged. 87+1 contract.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 21)

| Item | Status | Evidence |
|------|--------|----------|
| PERF-007 merge batch reads | **FIXED** | `services/entity_merge.rs` |
| PERF-KV-002 query filter SSOT | **FIXED** | `document_filter_resolver.rs` → scoped loader only |
| SOLID extract | **DONE** | `entity_merge` service module |
| Reliability E2E | **DONE** | health storage component probes |
| Contract tests | **DONE** | +2 contract, +1 e2e (64+26) |
| Bootstrap migration | **NOT NEEDED** | — |

**Verdict: A++ retained** — P2 HTTP cold paths closed; only storage admin + tenant-index constraint remain ACCEPTED.

---

## Code Re-assessment (phase 41)

PG auth E2E confirms O(1) API key prefix lookup path. Tests: 94+1 + 33 + 2 pg auth.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 43)

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 44)

AC-4 secure default. Migration 055. 99+1 + 35 e2e.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 45–46)

`health_schema.rs` — global ops query. 105 contract tests.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)

---

## Code Re-assessment (phase 47)

No perf change. Auth SSOT complete per 009.

Authority: [009-code-is-law-verdict.md](./009-code-is-law-verdict.md)
