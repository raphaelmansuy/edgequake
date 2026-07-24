# `GH-317` — Selected bulk document deletion fails

> **Priority**: P0  
> **Audit status**: FIXED  
> **Sprint**: 1  
> **Laws**: LAW-12, LAW-9, LAW-8  
> **GitHub**: https://github.com/raphaelmansuy/edgequake/issues/317  
> **Verified against**: v0.21.0 / `19477c2d`

---

## 1. WHY

Users select many documents → Delete → Confirm, and deletion does not complete reliably. At ~199 docs (reporter env), this recreates the same AGE/pool failure class as the old Clear All bug. Users cannot manage corpora.

**Critical distinction:** [#309](https://github.com/raphaelmansuy/edgequake/issues/309) Clear All / wipe-all is **FIXED** in v0.20.1+ via `WorkspaceWipe`. That does **not** close #317.

---

## 2. Audit (code is law)

| Field | Value |
|-------|-------|
| FE confirm | N× `handleDeleteDocument` fire-and-forget ([`document-manager.tsx`](../../../edgequake_webui/src/components/documents/document-manager.tsx) ~545-552) |
| API selected | Only `DELETE /documents/{id}` per doc |
| API wipe-all | `DELETE /documents` → `WorkspaceWipe` ([`bulk.rs`](../../../edgequake/crates/edgequake-api/src/handlers/documents/delete/bulk.rs)) — **different path** |
| Fairness | Lifecycle lane cap (`MAX_LIFECYCLE_TASKS_PER_TENANT` ≈ 4) parks extras |
| Tests | Playwright/e2e cover wipe-all (#309), **not** selected multi-delete |
| Verdict | **CONFIRMED** |

```typescript
// STILL PRESENT — selected bulk delete
for (const doc of bulkDeleteTargets) {
  handleDeleteDocument(doc.id); // parallel storm, no batch API
}
```

---

## 3. Root cause (first principles)

**LAW-12**: Lifecycle at scale must be O(batches), not O(docs)×expensive AGE discovery. Selected delete fans out N admits × N prefix cascades. Wipe-all fixed this for the full workspace with one graph clear; selected delete never got an equivalent durable job.

---

## 4. Multi-lens analysis

### Product Owner

- Acceptance: Select K docs (K up to hundreds) → one confirm → progress UI → all selected gone; unselected remain.
- Must not require Clear All as a workaround (destroys whole workspace).

### Full Stack

| Layer | Today | Target |
|-------|-------|--------|
| FE | N× DELETE | One `POST/DELETE` with `document_ids[]` + track |
| API | Single deletion task | `TaskType::BatchDeletion` (or similar) |
| Worker | Per-doc cascade | Batched discovery + delete; reuse SPEC-050/071 primitives |
| Progress | Per-toast noise | Single track like wipe |

### AI Engineer

- N/A beyond not leaving orphan embeddings if cascade partial — reuse existing cascade integrity rules.

### O(n) / Systems

- N× `find_*_by_source_prefixes` without batching → pool exhaustion (#331 class).
- Lifecycle fairness=4 → most of 199 deletes park; UI looks “stuck/failed.”
- Fix: one task, chunked ID batches (e.g. 50), shared discovery where prefixes allow.

### Postgres Expert

- Prefer batched child-table GIN discovery (`"Node"` / `"EDGE"`), never parent Seq Scan.
- Op-count proof: selected delete of 200 docs must not issue 200 independent full-graph scans.
- Transaction sizing: chunk deletes to avoid huge AGE transactions / long locks.

---

## 5. ASCII causal diagram

```
  User selects K docs → Confirm
        |
        v
  K parallel DELETE /documents/{id}
        |
        +--> lifecycle fairness cap (4) --> many parked
        |
        +--> K× AGE prefix cascades --> pool / timeout
        |
        v
  Partial deletes / UI "failed" / docs remain
```

---

## 6. Solution (SOLID + DRY)

| Principle | Application |
|-----------|-------------|
| S | `BatchDocumentDeletion` service owns selected-ID lifecycle |
| O | Share cascade discovery with single-delete; differ only in batching |
| L | Same graph invariants as single delete / wipe (no orphans) |
| I | `DeleteDocumentsRequest { document_ids: Vec<Id> }` with max K |
| D | FE depends on track progress DTO (wipe pattern), not N mutations |
| DRY | Reuse WorkspaceWipe progress UX + SPEC-071 discovery helpers |

### Implementation steps

1. Add API: e.g. `POST /api/v1/documents/delete` or `DELETE /api/v1/documents` with body `{ "document_ids": [...] }` — **must not** collide with wipe-all confirm semantics (wipe remains header/empty-body delete-all).
2. Admit one durable task; worker processes IDs in chunks; batched `find_nodes/edges_by_source_prefixes`.
3. FE `handleBulkDeleteConfirmed` → single mutation + progress dialog (mirror `clear-documents-dialog` / wipe WS).
4. Cap `document_ids.len()` (e.g. 500) with clear 400 error; for full workspace tell user to Clear All.
5. E2E op-count + Playwright selected delete (~200).

---

## 7. Edge cases

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | K=1 | Degenerate batch OK or delegate to single delete |
| EC-2 | IDs from other workspace | 404/403; tenant/workspace check |
| EC-3 | Already deleting | Idempotent track / skip |
| EC-4 | Partial cascade failure | Document partial_failure; retry subset |
| EC-5 | Mix processing + completed | Cancel ingest first for those IDs (wipe ordering lesson) |
| EC-6 | Empty selection | No-op UI |
| EC-7 | Concurrent wipe-all | Reject or serialize lifecycle |

---

## 8. E2E / contract tests

| Test | Assertion |
|------|-----------|
| `issue317_batch_delete_200_opcount` | 200 selected deletes → prefix discovery op count ≪ 200 full scans; completes under pool timeout |
| `issue317_unselected_remain` | Only selected IDs removed |
| `issue317_playwright_toolbar_bulk_delete` | Select multi → Delete → Confirm → rows gone; one track |

---

## 9. Cross-refs

- #309 / SPEC-050 WorkspaceWipe  
- SPEC-071 child-table discovery  
- #331 pool exhaustion class  
- `docs/ingestion-cancel-and-fairness.md` lifecycle caps
