# 008 — System Engineering Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [007 Postgres](./007-postgres-age-pgvector-lens.md) · [012 Plan](./012-improvement-plan.md)

**Findings:** R-01, R-03, R-09, N-03, N-09, N-10, N-12

---

## Operational Maturity (post SPEC-024)

EdgeQuake moved from **"works on my laptop"** to **"operator can diagnose prod"** in one SPEC cycle. This lens grades **runability**, not algorithm beauty.

**Grade: A+**

---

## Health Surface (code-verified)

`GET /health` exposes:

```text
  operational
  ├── task_queue        pending / processing / failed / pressure
  ├── query_engine      default_mode, reranker, fusion env, debounce
  ├── observability     log_format, otel_enabled
  ├── read_model        merge strategy, drift flags
  ├── ingestion         worker_queue, persist_ssot, reingest
  ├── storage           chunk_text_ssot, content_ref, kv_in_persister
  └── migration         version, index readiness, pgvector caps
```

E2E: `e2e_spec024_operational_excellence.rs` — including critical backlog → `status: degraded`.

---

## Backpressure Model

```text
  pending tasks
       │
       v
  assess_queue_pressure()
       │
       ├── normal    (< WARN threshold, default 100)
       ├── elevated  (≥ WARN)
       └── critical  (≥ CRITICAL, default 500)
                │
                └──> /health status = degraded
                └──> Prometheus gauges updated
```

Files: `task_queue_pressure.rs`, `metrics.rs::record_task_queue_stats`.

**Honest gap:** Channel queue depth still approximated (`ChannelTaskQueue::size()` → 0). DB pending count is the real signal — acceptable for now.

---

## Startup Recovery Chain

```text
  main.rs boot
  │
  ├─ recover_orphaned_tasks()      stuck processing → recover
  ├─ recover_orphaned_documents()  KV metadata repair
  ├─ requeue_pending_tasks()         DB → channel
  └─ WorkerPool start
```

Without this, async ingest **will** lose tasks on crash. Code acknowledges failure modes — good engineering.

---

## Observability Stack

| Capability | Status |
|------------|:------:|
| Structured JSON logs | ✓ `EDGEQUAKE_LOG_FORMAT` |
| Prometheus metrics | ✓ queue gauges + existing |
| OTEL full stack | ✗ `otel_enabled: false` default |
| Rerank timing truth | ✓ `rerank_time_ms` engine→API |
| Fusion config in health | ✓ hybrid + mix labels |

See [018-observability](../018-observability/) for OTEL roadmap.

---

## Failure Domains

```text
  ┌─────────────────────────────────────────────────────────┐
  │ Domain          │ Detection           │ Recovery       │
  ├─────────────────┼─────────────────────┼────────────────┤
  │ Worker crash    │ orphan task check   │ requeue        │
  │ LLM timeout     │ chunk partial fail  │ checkpoint     │
  │ Merge fail      │ saga compensate     │ delete vectors │
  │ Queue backlog   │ pressure metrics    │ scale workers  │
  │ KV/PG drift     │ read_model health   │ merge SSOT     │
  │ Admission orphan│ document recovery   │ manual re-upload│
  └─────────────────────────────────────────────────────────┘
```

### N-12 — Admission orphan row

Worker never starts → KV exists, graph empty. Recovery paths exist but **user-visible Failed state** still possible.

---

## Multi-Replica Gaps (honest)

| Component | Single-node | Multi-replica risk |
|-----------|:-----------:|:------------------:|
| Task queue channel | ✓ | Workers compete via Postgres — OK |
| Community debounce timer | ✓ | **Duplicate Louvain** (process-local) |
| In-memory embedding cache | ✓ | Cache miss only — OK |
| Query result cache | ✓ | Per-process LRU — OK |

**P2:** Externalize community refresh lock (Postgres advisory lock or job table).

---

## System Engineering Verdict

SPEC-024 Phase 4 delivered what most RAG startups skip entirely:

- Truthful metrics (no fabricated rerank time)
- Degraded health on backlog
- SRP migration bootstrap split
- Operator-visible fusion config

**Remaining ops work:** N-10 eval CI, OTEL traces, multi-replica community lock, N-03 payload slimming for DB size.

**This is the strongest lens in the audit.** Don't let retrieval marketing outrun this ops foundation.
