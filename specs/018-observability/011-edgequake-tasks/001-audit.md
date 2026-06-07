# edgequake-tasks — Observability Audit

**Path:** `edgequake/crates/edgequake-tasks`  
**Tracing macros (src):** ~10 (concentrated in `worker.rs`)  
**Role:** Background document processing queue (Postgres-backed)

---

## Executive Summary

Task queue is **operationally critical** but **under-instrumented relative to API handlers**. `worker.rs` imports `info/warn/error/debug` but queue depth and stall detection lack metrics — aligns with known “Processing…” UI stalls (AGENTS.md).

API layer (`handlers/tasks.rs`) has more visibility than the crate itself.

---

## Findings

| ID | P | Finding | Evidence | Remediation |
|----|---|---------|----------|-------------|
| TASK-OBS-001 | P1 | Worker lifecycle not logged | `worker.rs` | `info!(task_id, status)` on state transitions |
| TASK-OBS-002 | P1 | Queue depth invisible | `queue.rs` | Gauge `edgequake_task_queue_size` |
| TASK-OBS-003 | P2 | Orphan recovery only in main | `main.rs` periodic check | Span per recovery batch |
| TASK-OBS-004 | P2 | Failed tasks | Processor in API | `error!(task_id, error)` in worker |

---

## Target Flow

```
  enqueue ──▶ tasks_pending gauge++
       │
       ▼
  worker pick ──▶ info!(task_id, document_id, attempt)
       │
       ├── success ──▶ info!(duration_ms)
       └── fail    ──▶ error!(error) + audit event
```

---

## Verify

```bash
rg 'tracing::' edgequake/crates/edgequake-tasks/src
rg 'tracing::' edgequake/crates/edgequake-api/src/handlers/tasks.rs
```
