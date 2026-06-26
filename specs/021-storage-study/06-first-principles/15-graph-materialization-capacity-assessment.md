# 15 — Graph Materialization Capacity: First-Principles Assessment

**Spec:** `021-storage-study`
**Date:** 2026-06-25
**Trigger:** User screenshot showing `Failed to load graph: Graph materialization capacity reached` toast on the Knowledge Graph page, with the left pane rendering a partial graph and the right pane blank.

---

## 1. Observed symptom

| Aspect | Value |
|--------|-------|
| Error message (toast) | `Failed to load graph: Graph materialization capacity reached` |
| HTTP status produced by backend | `503 Service Unavailable` (`retry_after_secs: 5`) |
| Where it is raised | `edgequake-api/src/services/graph_materialization.rs::admit_graph_materialization` |
| Where it is *surfaced to the user* | `graph_stream.rs` line 76–79 — sent as an SSE `error` event (not an HTTP 503, because the guard runs inside a `tokio::spawn` task *after* the SSE handshake already returned 200) |
| Frontend handling | `use-graph-stream.ts` `onError` → `graph-viewer.tsx` `toast.error(...)` — **no retry, no backoff** |
| Concurrency cap (default) | `DEFAULT_GRAPH_MATERIALIZE_CONCURRENT = 1` (`budget.rs:30`) |

## 2. Root cause (code-verified)

The graph endpoints (`/api/v1/graph`, `/api/v1/graph/stream`, `/graph/traversal`, `/graph/search`) all call `admit_graph_materialization(&state)` which `try_acquire_owned()` a single global semaphore. **Only one graph materialization may run at a time across the entire process.** Any concurrent request — a second browser tab, a StrictMode double-mount, a refetch racing the stream, or a tenant/workspace switch that restarts the stream — is rejected immediately with 503.

Two compounding design problems:

1. **The cap is a count, not a capacity.** The semaphore size (`graph_materialize_concurrent`) is the *number of in-flight materializations*, not a measure of memory/throughput headroom. It is decoupled from `graph_scan_threshold_nodes` (50 000), `max_graph_nodes` (500), available RAM, or DB pool size. A single small graph and a single huge graph both consume exactly one permit.

2. **The failure mode is opaque and unactionable to the client.** The SSE path delivers the error as a stream `error` event with a bare string. The client has no `retry_after_secs`, no hint that the failure is transient (it is — a permit will free up in seconds), and no automatic retry. The user sees a red toast and a half-drawn graph, and must manually reload.

## 3. First-principles analysis

A capacity system for graph materialization must answer four questions unambiguously. Today each is answered weakly or not at all.

### P1 — What is the scarce resource being protected?
**Should be:** RAM + DB connection time, bounded by the *data volume* of the materialization (node/edge count), not by a request counter.
**Today:** A request counter. One permit == one request regardless of whether it materializes 5 nodes or 500. A 5-node request can block a 500-node request that would have fit comfortably in memory.

### P2 — How is the limit expressed and enforced?
**Should be:** A *cost function* on the materialization (e.g. `cost ≈ node_count × k_nodes + edge_count × k_edges`) admitted against a memory budget, with a cheap pre-count (`node_count_fast`) used as a fast-path estimate. Hard ceiling only as a backstop.
**Today:** A binary `try_acquire` on a count-1 semaphore. No cost, no estimate, no backstop — just "yes" or "no, try later".

### P3 — What does the client see when the limit is hit, and what can it do?
**Should be:** A structured error with `retry_after_secs` and a `reason` (`transient_congestion` vs `graph_too_large`), plus client-side exponential backoff with jitter so concurrent clients don't thunder-herd the single slot.
**Today:** SSE `error` event with the literal string `"Graph materialization capacity reached"`. No retry-after, no reason code, no client retry. The HTTP path returns a proper 503 with `retry_after_secs: 5`, but the streaming path (the one the UI actually uses) discards that structure.

### P4 — Is the default sized for the workload?
**Should be:** The default must comfortably absorb the *expected concurrent interactive load* (typically 1 user × 2–3 racing requests from StrictMode/tenant-switch/refetch). A default of 1 fails this trivially — any two overlapping requests collide.
**Today:** Default `1`, clamp `[1, 16]` via `EDGEQUAKE_GRAPH_MATERIALIZE_CONCURRENT`. 1 is the floor of the clamp and is too low for even a single interactive user.

## 4. Assessment against best-practice capacity design

| Best practice | Status | Gap |
|---------------|--------|-----|
| Limit protects the *real* scarce resource (memory/DB), not a proxy counter | ❌ | Semaphore is a request counter, memory-unaware |
| Limit is derived from a cost function, not a boolean | ❌ | `try_acquire` only |
| Client receives structured, actionable feedback (reason + retry-after) | ⚠️ partial | HTTP path yes, SSE path no |
| Client retries with backoff + jitter for transient congestion | ❌ | No retry; user must reload |
| Default is sized for realistic concurrent interactive load | ❌ | 1 is below the minimum useful (2–3) |
| Limit is observable (metric exposed) | ⚠️ | `edgequake_graph_materialize_active` gauge exists but is not surfaced to the UI |
| Limit is a single source of truth | ✅ | `budget.rs` is the SSOT; env override exists |

## 5. Recommendations (ordered by impact / effort)

### R1 — Raise the default and make the clamp pool-aware (low effort, high impact)
- Raise `DEFAULT_GRAPH_MATERIALIZE_CONCURRENT` from `1` to `4`. This absorbs StrictMode double-mounts and tenant switches without collision.
- **Make the env clamp pool-aware** (re-assessed): each materialization holds up to 3 concurrent DB connections (the `tokio::join!` of `node_count_fast` + `edge_count_fast` + `get_popular_nodes_with_degree` in `graph_stream.rs`). The previous clamp `[1, 16]` was unsafe — `16 × 3 = 48` connections would exhaust the default pool of 32. The new clamp derives the upper bound from `DATABASE_POOL_SIZE`: `⌊(pool - 8) / 3⌋` (8 reserved for non-graph traffic), with an absolute cap of 8. With the default pool of 32, the effective ceiling is 8; the default remains 4 to leave headroom for ingestion workers.
- Document the change in `004_resource_budget_catalog.md` (RB-MEM-002) per BR-006-012.

### R2 — Make the SSE error path carry the same structure as the HTTP path (medium effort, high impact)
- In `graph_stream.rs`, when `admit_graph_materialization` fails, send an SSE `error` event that includes `retry_after_secs` and a `reason: "transient_congestion"` code (mirroring the 503 body), instead of the bare string.
- Update `GraphStreamEvent::Error` to carry these fields.

### R3 — Add client-side retry with backoff + jitter for transient congestion (medium effort, high impact)
- In `use-graph-stream.ts`, when an SSE `error` event carries `reason: "transient_congestion"`, automatically retry up to N times with exponential backoff + jitter (e.g. `base × 2^attempt + rand(0..base)`), capped by `retry_after_secs` from the server.
- Keep a hard failure for non-transient reasons (`graph_too_large`, DB errors) so we don't retry hopeless queries.

### R4 — Move from request-count to cost-based admission (larger effort, strategic)
- Add a cheap pre-count admission: use `node_count_fast()` to estimate cost before acquiring the permit; reject early with `reason: "graph_too_large"` above `graph_scan_threshold_nodes`.
- Track bytes-materialized against a memory budget gauge and reject when headroom is exhausted (`DEFAULT_MEM_HEADROOM_RATIO` already exists but is unused for graph admission).
- This makes the limit *honest* — it protects memory instead of pretending one request == one unit of pressure.

### R5 — Surface active materialization count to the UI (low effort, nice-to-have)
- The `edgequake_graph_materialize_active` gauge already exists; include it in `/health` or a `/status` payload so the UI can show "N graph queries running" instead of a silent failure.

## 6. DRY/SOLID implications

- `admit_graph_materialization` is correctly a single entry point (SRP ✅, DRY ✅) — the fix does **not** duplicate the admission call; it enriches the error it returns and the way two *different* transports (HTTP vs SSE) consume that error.
- The SSE `error` event type and the HTTP 503 body should share a single `MaterializationBusy` struct (Open/Closed: extend the enum variant rather than branching on transport). Today the SSE path re-types the string literal — a DRY violation between `error.rs:424` and `graph_stream.rs:77`.
- The client retry policy belongs in `use-graph-stream.ts` (one place), not copied into every graph consumer (DRY).

## 7. Proposed remediation order for SPEC-021

1. **R1** (raise default to 4) + **R2** (structured SSE error) + **R3** (client retry) — these three together eliminate the user-visible toast for the common transient case. This is the P5-03 deliverable.
2. **R4** (cost-based admission) — tracked as a follow-up; requires a memory-pressure gauge and is larger.
3. **R5** (UI observability) — small, fold into R1.

---

## Appendix A — Call graph

```
graph-viewer.tsx
  └─ useGraphStream (use-graph-stream.ts)
       └─ fetch /api/v1/graph/stream (SSE)
            └─ stream_graph (graph_stream.rs)
                 └─ tokio::spawn {
                        admit_graph_materialization(&state)   <-- try_acquire_owned(), count=1
                          └─ Err → send SSE error "Graph materialization capacity reached"
                                  (retry_after_secs LOST here)
                     }
```

## Appendix B — Key files

| File | Role |
|------|------|
| `edgequake-core/src/resource/budget.rs` | SSOT for `graph_materialize_concurrent` (default 1, clamp 1–16) |
| `edgequake-core/src/resource/semaphore.rs` | `GraphMaterializationSemaphore` wrapper |
| `edgequake-api/src/services/graph_materialization.rs` | `admit_graph_materialization` — single admission entry |
| `edgequake-api/src/error.rs:422` | `graph_materialization_busy()` → HTTP 503 with `retry_after_secs: 5` |
| `edgequake-api/src/handlers/graph/graph_stream.rs:72-82` | SSE path — discards 503 structure, sends bare string |
| `edgequake_webui/src/hooks/use-graph-stream.ts:252-263` | Client SSE error handling — no retry |
| `edgequake_webui/src/components/graph/graph-viewer.tsx:225-231` | Toast on error |
