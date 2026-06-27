# 008 — System Engineering Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [007 Postgres](./007-postgres-age-pgvector-lens.md) · [012 Plan](./012-improvement-plan.md)  
**Post-remediation:** SPEC-024 pass 11 (2026-06-27)

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
  │ + WorkerPool│    │ engine_impl │    │ (bootstrap) │
  └──────┬──────┘    └─────────────┘    └─────────────┘
         │
         v
  ┌─────────────┐         ┌─────────────┐
  │ PostgreSQL  │         │ LLM Provider│
  │ tasks + docs│         │ Ollama/OpenAI│
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
| Pipeline checkpoints | `text_insert.rs` | Good — worker path |
| Saga compensation | `compensation.rs` | Good — vector/graph |
| PDF single-flight admission | `ingest_admission.rs` | Good |
| Async file/batch upload | `file_upload.rs`, `batch_upload.rs` | Good — 202 + queue |
| Injection as queued task | `TaskType::KnowledgeInjection` | Good |
| Debounced community index | `community_index_service.rs` | Good |
| Workspace-scoped cache bust | `QueryResultCache::invalidate_workspace` | Good |

---

## Reliability Gaps — Remediation Status (SPEC-024)

| ID | Gap | Status | Evidence |
|----|-----|:------:|----------|
| G1 | Split brain ingestion (F-01) | **Fixed** | Uploads → `TaskRuntime`; `e2e_spec024_async_file_upload.rs` |
| G2 | Global cache invalidation (F-09) | **Fixed** | Workspace-scoped epoch; `contract_workspace_cache_invalidation.rs` |
| G3 | Injection fire-and-forget | **Fixed** | `TaskType::KnowledgeInjection`; worker E2E |
| G4 | Best-effort dual-write KPI drift | **Mitigated** | `document_read_model.rs` merge + `/health` read_model snapshot |
| G5 | Strict workspace not enforced | **Fixed** | Production bootstrap + worker strict mode |

**Remaining (P2):** Sustained overload can still grow the `tasks` table — **fully monitored** (pass 11): `task_queue_pressure` labels, structured warn/error logs, Prometheus gauges, `/health` degraded when critical. Scale workers or reduce ingest rate when alerts fire.

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
  LLM + Postgres
       │
       v
  Queue pressure SSOT (pass 10–11): task_queue_pressure.rs
  - EDGEQUAKE_QUEUE_PENDING_WARN / _CRITICAL
  - /health + queue-metrics: pressure + operator_action
  - /health status degraded when critical
  - publish_queue_observability → Prometheus gauges + structured logs
  - edgequake_task_queue_{pending,processing,failed}
```

**Operator visibility:** `/health` → `operational.task_queue` (counts + **pressure**), `query_engine`, `observability`, `read_model`, `migration` (Postgres).

---

## Failure Mode Matrix (post Phase 1)

| Failure | Worker path | Async upload | Injection |
|---------|:-----------:|:------------:|:---------:|
| LLM timeout | partial_failure | task retry | task retry |
| Persist fail | retry task | task retry | task retry |
| Server crash | checkpoint resume | task resume | task resume |
| Duplicate upload | hash dedup | hash dedup | N/A |
| Cancel mid-flight | cooperative | cooperative | cooperative |

---

## Observability (code-visible)

| Signal | Status |
|--------|:------:|
| `tracing` + `EDGEQUAKE_LOG_FORMAT=json` | ✅ |
| `rerank_time_ms` engine → query API | ✅ |
| `rerank_time_ms` engine → chat API | ✅ (pass 9) |
| Task queue depth in `/health` | ✅ |
| Queue metrics endpoint | ✅ |
| Prometheus task-queue gauges | ✅ (pass 11) |
| Structured queue pressure logs | ✅ warn/error on elevated/critical |
| Fusion config in `/health` | ✅ hybrid_fusion + mix_fusion |

Full OTEL stack: SPEC-018 (opt-in build feature).

---

## System Engineering Verdict

**Grade: A (post SPEC-024 pass 11)** — was **B- / D (consistency)**

Individual components remain production-grade. Phase 1 eliminated split-brain ingestion; Phase 4 closed observability gaps. Queue depth under overload is **monitored end-to-end** (health + Prometheus + logs + degraded status). Remaining P2: **`migration_bootstrap/mod.rs` orchestration** (~565 LOC) — reconcile hooks now per-migration (`reconcile/m038..m045.rs`).

**See:** [012-improvement-plan.md](./012-improvement-plan.md)
