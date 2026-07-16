# SPEC-054 — Code Is Law Assessment

**Date:** 2026-07-16 (stampede-guard pass)  
**Standard:** Code is law. Shipped artifacts + falsifying tests required to close issues.

---

## Executive verdict

| Issue | Grade | Status |
| --- | --- | --- |
| **#300** | **B−** | Commit-ready; 4/4 postgres e2e falsified earlier |
| **#298** | **B+** | Commit-ready + **stampede guard** (no more 10k enqueue on `make dev`) |

**Still not law:** uncommitted, unpublished. GitHub #300/#298 **OPEN**.

---

## What “huge activity on make dev” was

Startup previously:

1. Rewrote **all** non-terminal docs (including already-`pending`) → `pending`
2. Reconciled with **`max_documents = 10_000`** → flooded Mistral with PDF extraction

**Fixed:**

- Recover skips already-`pending`/`queued` (waiting ≠ mid-flight orphan)
- Recover returns IDs; reconcile **priority path** uses those IDs only
- Backlog drain capped by `EDGEQUAKE_STARTUP_RECONCILE_MAX` (**default 32**)
- Existing Pending tasks in Postgres still hydrate after workers (expected resume)

To raise the budget intentionally:

```bash
export EDGEQUAKE_STARTUP_RECONCILE_MAX=100
make dev
```

---

## Test evidence

| Suite | Result |
| --- | --- |
| `e2e_spec054_pending_task_reconcile` | **8/8** (+ stampede max guard) |
| `spec045_ingestion_reliability` (SPEC-054 filters) | **pass** |
| `pending_doc_task_reconcile` unit | **5/5** |
| `startup_task_hydrate` soak | **1/1** (prior) |
| WebUI vitest upload | **9/9** |

---

## Checklist

```text
[x] DRY SSOT modules (progress_identity, pending_doc_task_reconcile, startup_task_hydrate)
[x] N>100 hydrate soak
[x] Stampede guard (skip already-pending recover + cap reconcile)
[x] Full local e2e matrix green
[ ] Commit
[ ] Patch release
[ ] specs/056 on published digests
[ ] Post GITHUB_COMMENTS; CLOSE issues
```

**Lawful status:** **READY TO COMMIT** — restart `make dev` to pick up stampede guard.

---

## Pointers

| Module | Role |
| --- | --- |
| `main.rs` recover | Skip waiting pending; return recovered IDs |
| `pending_doc_task_reconcile.rs` | `by_ids` + capped scan; `EDGEQUAKE_STARTUP_RECONCILE_MAX` |
| `startup_task_hydrate.rs` | Background channel hydrate |
| `progress_identity.rs` | #300 progress key SSOT |
