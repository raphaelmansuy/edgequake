# 008 — System Engineering Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [007 Postgres](./007-postgres-age-pgvector-lens.md) · [F-01,F-09,F-10](./README.md#cross-reference-matrix)

---

## Production Architecture

```text
                    ┌─────────────────┐
                    │   Axum API      │
                    │  (edgequake-api)│
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         v                   v                   v
  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
  │ TaskRuntime │    │ QueryEngine │    │  AppState   │
  │ + WorkerPool│    │ (engine_impl)│   │ 25+ deps    │
  └──────┬──────┘    └─────────────┘    └─────────────┘
         │
         v
  ┌─────────────┐         ┌─────────────┐
  │ PostgreSQL  │         │ LLM Provider│
  │ tasks table │         │ Ollama/OpenAI│
  └─────────────┘         └─────────────┘
```

**Mandatory:** `DATABASE_URL` — server exits without DB.

---

## Reliability Mechanisms (what works)

| Mechanism | Location | Assessment |
|-----------|----------|------------|
| Task retry + backoff | `worker.rs` | Good |
| Heartbeat / stale detection | `postgres.rs` touch_task | Good |
| Circuit breaker on timeouts | `worker.rs` | Good |
| Tenant concurrency limiter | `worker.rs` | Good — fair queuing |
| Cooperative cancellation | `CancellationRegistry` | Good |
| Pipeline checkpoints | `text_insert.rs` | Good — async path only |
| Saga compensation | `compensation.rs` | Good — vector/graph only |
| PDF single-flight admission | `ingest_admission.rs` | Good |

---

## Reliability Gaps

### G1 — Split brain ingestion (F-01, P0)

Sync HTTP upload bypasses queue → no checkpoint, no cancel, blocks connection, different timeout semantics.

**Blast radius:** Gateway timeout kills ingest mid-pipeline; client retries → duplicate work unless hash dedup catches it.

### G2 — Global cache invalidation (F-09, P1)

Every ingest bumps query result cache epoch globally.

```text
  ingest doc in workspace A  ──> invalidate ALL cached context_only queries
                                      in workspaces B, C, ...
```

Under ingest load, query cache **never hits**.

### G3 — Injection fire-and-forget (P1)

`tokio::spawn` without task tracking — no queue visibility, no retry, no backpressure.

### G4 — Best-effort dual-write (P1)

KV metadata vs `documents` table — failures logged, not reconciled.

Dashboard KPIs (`document_read_model.rs`) can show stale counts.

### G5 — Strict workspace mode not enforced everywhere (P0)

Documented silent fallback when `strict_workspace_mode=false`. Production should **hard-fail** if workspace providers missing.

---

## Backpressure & Capacity

```text
  Request rate
       │
       v
  ┌─────────────────────────────────────────┐
  │ Admission: safety_limits, rate limiter    │
  │ Tenant limiter: requeue @ 500ms         │
  │ Extraction semaphore: max concurrent LLM  │
  └─────────────────────────────────────────┘
       │
       v
  Worker pool (fixed size)
       │
       v
  LLM + Postgres (unbounded queue depth?)
```

**Gap:** Task queue depth monitoring not evident in hot path code. Risk of unbounded `tasks` table growth under sustained overload.

---

## Failure Mode Matrix

| Failure | Worker path | Sync upload | Injection |
|---------|:-----------:|:-----------:|:---------:|
| LLM timeout | partial_failure | HTTP 500 | metadata failed |
| Persist fail | retry task | orphan KV | spawn error swallowed |
| Server crash | checkpoint resume | lost in-flight | lost in-flight |
| Duplicate upload | hash dedup | hash dedup | N/A |
| Cancel mid-flight | cooperative | client disconnect | no cancel |

---

## Observability (code-visible only)

- `tracing` crate used (not println) ✓
- Stage metadata on documents for UI polling ✓
- WebSocket events in processor ✓
- `rerank_time_ms` not exposed separately at API

Full observability spec in 018 — not re-audited here.

---

## Deployment & Migrations

- Postgres extensions verified via `verify-postgres-extensions.sh`
- Migration bootstrap runs support SQL for extension upgrades
- Pre-commit warns if migration without checksum update

**Ops risk:** Long-running Louvain on ingest extends task duration → worker heartbeat stress, tenant slot hogging.

---

## System Engineering Verdict

**Grade: B- (worker) / D (overall consistency)**

Individual components (worker, postgres adapters, admission) are **production-grade**. System-level weaknesses come from **multiple ingestion paths** and **global cache invalidation** — classic integration debt after feature accretion.

**Top 3 system fixes:**
1. Enqueue all ingests through `TaskRuntime`
2. Workspace-scoped cache epoch
3. Track injection as queued tasks, not spawn

See [012-improvement-plan.md](./012-improvement-plan.md) Phase 1.
