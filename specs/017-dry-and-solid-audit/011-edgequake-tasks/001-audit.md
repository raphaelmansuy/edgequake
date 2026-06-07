# edgequake-tasks — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-tasks`  
**LOC:** ~5,925 (src) + ~1k tests  
**Role:** Background task queue, worker pool, pipeline state tracking

---

## Executive Summary

**Correct crate boundary** for substantial async job domain. Good trait design (`TaskStorage`, `TaskQueue`, `TaskProcessor`). Expected memory/postgres parallel impls. Minor leakage: task payloads import `PdfParserBackend` from pdf crate.

---

## DRY Violations

| ID | P | Violation | Evidence | Remediation |
|----|---|-----------|----------|-------------|
| TASK-DRY-001 | **P2** | memory/postgres storage parallel impls | `memory.rs` (427) vs `postgres.rs` (616) | Expected; extract shared helpers to reduce drift |
| TASK-DRY-002 | **P3** | PDF type leakage in task payload | `types/data.rs:6` imports `PdfParserBackend` | Opaque string; resolve in API layer |

---

## SOLID Violations

| ID | P | Principle | Violation | Evidence |
|----|---|-----------|-----------|----------|
| TASK-SOLID-S-001 | **P2** | SRP | `pipeline_state/mod.rs` ~647 LOC | PDF tracking + emitters + snapshots |
| TASK-SOLID-S-002 | **P3** | SRP | `worker.rs` ~857 LOC | Large but cohesive |
| TASK-SOLID-D-001 | ✅ | DIP | Good trait boundaries | `storage.rs`, `queue.rs`, `worker.rs` |

---

## Verdict

**Keep as standalone crate.** Queue, worker pool, pipeline state, tenant limiter justify separation.

---

## Remediation Plan

| P | Action |
|---|--------|
| **P2** | Extract shared storage helpers between memory/postgres |
| **P3** | `PdfParserBackend` → opaque string in payload |
| **P3** | Split `pipeline_state/` into `pdf_tracking`, `emitters`, `snapshot` (partial exists) |

---

## Verification

```bash
cargo test -p edgequake-tasks --lib
cargo test -p edgequake-api --test e2e_document_processing_pipeline
```

---

## Positive Patterns

- `TaskProcessor` trait enables API injection of document processor
- Tenant limiter integration for fair queueing
- Postgres + memory backends for CI vs production
