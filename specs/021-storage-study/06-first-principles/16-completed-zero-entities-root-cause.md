# 16 — "Completed / 0 Entities" Root-Cause Assessment (Code Is Law)

> **Spec**: 021-storage-study
> **File**: 06-first-principles/16-completed-zero-entities-root-cause.md
> **Date**: 2026-06-25
> **Trigger**: User screenshot — Documents page shows 7 PDFs, **all** `Status = Completed`,
> **all** `Entities = 0`, **all** `Cost = -`.
> **Method**: First-principles + "Code is Law" — every claim re-verified by reading the
> production source in `edgequake/crates/...` and `edgequake/migrations/...`.
> **Supersedes (in part)**: nothing; **extends** 11-ux-zero-documents-root-cause-assessment.md
> (which addressed the *dashboard* "0 documents" KPI, not the *per-document* "0 entities" cell).

---

## 0. How to read this document

- The earlier file 11 diagnosed "Dashboard shows 0 documents while entities exist."
- This file diagnoses a **different, equally damaging** symptom:
  "Document list shows N documents, each `Completed`, each `Entities = 0`."
- The two share a *family* root cause (cross-store read-authority drift) but
  the *concrete* defect is different and is **not yet fixed** in the codebase.
- Each finding is tagged **VERIFIED** (re-read in source on 2026-06-25),
  **NEW** (gap not previously captured in the spec series), or
  **CONFIRMS** (corroborates an earlier file).

---

## 1. Symptom decomposition

The Documents table cell `Entities` for a row is rendered from
`DocumentSummary.entity_count`. Three distinct upstream feeds can produce that
field, and the table shows `0` only when **all** feeds that the row used resolve
to `0` (or `None`, which the UI renders as `0`).

| Feed | Source | Populated by | Used when |
| ---- | ------ | ------------ | --------- |
| F-KV | KV metadata key `{doc_id}-metadata`, field `entity_count` | `status_updates.rs::update_document_status_with_stats` L333 (`updated.insert("entity_count", json!(stats.entity_count))`) | Row came from KV scan (legacy + async text uploads) |
| F-PG | Relational `documents.entity_count` column | **NOTHING in production code** — see §3 | Row was backfilled by `list_relational_document_summaries` (P5-01) when KV metadata is missing |
| F-Graph | AGE `node_count_by_workspace` | `stats.rs::try_kv_storage_stats` L214–219 | Only the **dashboard** aggregate KPI; never the per-row cell |

The **dashboard** `entity_count` KPI uses F-Graph (AGE) and is correct. The
**per-document** cell uses F-KV or F-PG. **F-PG is the broken feed.**

---

## 2. First-principles trace (UI cell → storage)

```
UI Documents table cell "Entities"
  └─ DocumentsPage renders documents[i].entity_count
       └─ GET /api/v1/documents → list_documents (handlers/documents/query/list.rs)
            ├─ Branch A: KV-scanned rows → meta.entity_count from KV metadata JSON (L220-223)
            └─ Branch B: P5-01 relational backfill → documents.entity_count column
                 (document_read_model.rs::list_relational_document_summaries L132-144)
                      └─ SELECT entity_count FROM documents WHERE workspace_id = $1
                           └─ documents.entity_count column ← WHO WRITES IT?
```

Answer to "who writes it?": **no one, after the initial INSERT.** This is the defect.

---

## 3. Root cause #1 (CRITICAL): relational `documents.entity_count` / `chunk_count` are write-only-dead columns

**VERIFIED + NEW.** This is the highest-impact finding in this assessment.

### 3.1 Evidence

1. **Schema** (`migrations/001_init_database.sql` L191-192, `003_add_document_status_fields.sql` L16-17):
   ```sql
   chunk_count INTEGER DEFAULT 0,
   entity_count INTEGER DEFAULT 0,
   ```
   Both columns exist with default `0`.

2. **Initial row insert** (`pdf_storage_impl.rs::ensure_document_record` L397-436):
   ```sql
   INSERT INTO documents (id, tenant_id, workspace_id, title, content, status, updated_at)
   VALUES ($1, $2, $3, $4, $5, $6, NOW())
   ON CONFLICT (id) DO UPDATE SET
       content = EXCLUDED.content,
       status  = EXCLUDED.status,
       title   = EXCLUDED.title,
       updated_at = NOW()
   ```
   **`chunk_count` and `entity_count` are never in the INSERT column list nor in
   the ON CONFLICT UPDATE list.** They stay at the schema default `0` forever.

3. **Post-pipeline stats update** (`status_updates.rs::update_document_status_with_stats` L293-399):
   writes `chunk_count`, `entity_count`, `relationship_count`, cost, tokens, etc.
   into **KV metadata only** (`self.kv_storage.upsert(&[(metadata_key, json)])`).
   There is **no corresponding SQL `UPDATE documents SET chunk_count = ...,
   entity_count = ...`** anywhere in the crate.

4. **Grep proof** (`UPDATE documents SET chunk_count|entity_count`): zero
   production matches. The only `UPDATE documents SET` in production code is in
   a negative-RLS test (`e2e_postgres_rls.rs` L317) and an injection handler
   (`injection.rs` L1025) that updates a *different* column set.

### 3.2 Why this produces exactly the screenshot

- The 7 PDFs were ingested via the **async processor path**
  (`DocumentTaskProcessor::process_text_insert`), which:
  1. writes chunks → KV + vectors,
  2. writes entities → AGE graph (+ relational `entities` table when sync enabled),
  3. writes `entity_count` into **KV metadata**,
  4. calls `pdf_storage.ensure_document_record(...)` which upserts the
     **relational** `documents` row **without** `entity_count`.
- The Documents page list handler then merges KV rows with relational rows
  (`merge_document_summaries`). For rows where KV metadata exists, the KV
  `entity_count` wins (good). For rows where KV metadata is **missing or scoped
  to a legacy workspace_id** (the exact condition documented in file 11 §D), the
  relational row is used and `entity_count` falls back to the column default `0`.
- The screenshot's "all 7 show 0" means **all 7 rows are being served from the
  relational backfill**, i.e. their KV metadata is not matching the selected
  workspace context. This is the **same workspace-scope drift** as file 11, now
  manifesting on the per-entity cell instead of the document count.

### 3.3 Why "Status = Completed" is not a contradiction

The `final_status` decision tree (`text_insert.rs` L1043-1086) correctly marks a
document `partial_failure` when `entity_count == 0 && chunk_count > 0`. So a
*new* ingestion that extracts 0 entities would show `partial_failure`, not
`completed`. The screenshot's `Completed` status therefore means one of:

- **(a)** Entities **were** extracted into AGE (graph is populated), but the
  per-row `entity_count` cell reads from the broken relational column → 0.
  This is consistent with the dashboard aggregate (F-Graph) showing a non-zero
  entity_count for the same workspace (file 11 §D: `entity_count: 412`).
- **(b)** The documents were ingested **before** the FIX-1/FIX-2 status logic was
  added; their KV metadata still carries `status: "completed"` from the legacy
  path, and the relational row mirrors it via `ensure_document_record`.

**Both (a) and (b) reduce to the same fix**: stop relying on a relational column
that is never refreshed, and make the per-row entity cell authoritative from a
source that is actually updated.

### 3.4 Why "Cost = -" corroborates

`DocumentSummary.cost_usd` is read from the same KV metadata field
(`list.rs` L238) or from the relational `documents` table — but the relational
schema has **no `cost_usd`, `input_tokens`, `output_tokens`, `total_tokens`
columns** (verified against `001_init_database.sql` and all migrations). So the
relational backfill row always returns `cost_usd: None` → UI renders `-`. The
KV row would carry cost. The screenshot showing `-` for **all** rows again
indicates all rows are served from the relational backfill, confirming §3.2.

---

## 4. Root cause #2 (HIGH): per-row entity_count read authority is undefined

**VERIFIED + NEW.** Extends R-CONS-04 (file 12 §2) from the *document count* to
the *per-document entity count*.

The P5-01 fix (`document_read_model.rs`) resolved the **dashboard document
count** by taking `max(postgresql, kv)`. It deliberately left
`entity_count`/`relationship_count` to AGE (correct for the aggregate KPI).
But the **per-row** `entity_count` in the documents list was left splitting
between two unreliable sources:

- F-KV: only correct if KV metadata matches the selected workspace scope.
- F-PG: never correct (§3).

There is no third option wired in: the list handler does **not** query
`graph_storage.node_count_by_document(doc_id)` or
`entities WHERE source_chunk_ids && ARRAY[...]` to compute a per-doc entity
count from the authoritative graph. So when both F-KV and F-PG fail, the cell is
`0` with no fallback to the truth (AGE).

---

## 5. Root cause #3 (MEDIUM): workspace-scope drift is still unmitigated for legacy uploads

**CONFIRMS file 11 §D.** The P5-01 hybrid read model patched the *count* but not
the *per-row scope match*. KV metadata for the 7 PDFs carries
`tenant_id = 00000000-0000-0000-0000-000000000002`,
`workspace_id = 00000000-0000-0000-0000-000000000003` (legacy), while the UI is
scoped to the modern "Mistral / Default Workspace" UUID. The list handler's
`matches_tenant_context` filter (L306-312) drops these KV rows, leaving only the
relational backfill — which has `entity_count = 0` (§3) and `cost_usd = None`
(§3.4). The user perceives a fully broken ingestion when in fact the graph is
populated.

---

## 6. Root cause #4 (MEDIUM): no per-document storage invariant gate

**NEW.** The `StorageInspector` (file 09, file 12 §7 P4-05) plans an invariant
comparing `documents.chunk_count` vs KV chunk-key count per `document_id`. But:

- It compares two **unreliable** numbers (the relational column that is never
  updated vs the KV count). Since the relational column is always 0, the
  invariant would fire **for every document** — noise, not signal.
- It does **not** compare against the authoritative AGE `node_count` per
  `source_ids` prefix, which is the only source that reflects reality.

The invariant as designed cannot detect this bug because it trusts the
relational column. The fix is to make the invariant compare **KV-stored
entity_count** vs **AGE node count for the doc's chunk-id prefix**, and to
surface a per-doc mismatch in the admin endpoint.

---

## 7. Root cause #5 (LOW): status normalization hides partial_failure from the UI

**VERIFIED.** `document_read_model.rs::normalize_relational_status` (L65-70)
maps `"indexed" → "completed"` and passes everything else through. So a
relational row with `status = "partial_failure"` is shown correctly. However,
the `StatusCounts` aggregator in `list.rs` (L469-476) treats `None`,
`"completed"`, and `"indexed"` as `completed`. A relational row backfilled with
`status = NULL` (possible if `ensure_document_record` was called before status
was finalized) is silently counted as completed. This is a minor display
integrity issue but contributes to the "looks completed, isn't" perception.

---

## 8. Evidence map (file:line)

| Claim | Evidence |
| ----- | -------- |
| `documents.entity_count` column exists, default 0 | `migrations/001_init_database.sql:191-192`, `003_add_document_status_fields.sql:16-17` |
| `ensure_document_record` never sets `entity_count`/`chunk_count` | `edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs:397-436` |
| `update_document_status_with_stats` writes stats only to KV | `edgequake-api/src/processor/status_updates.rs:293-399` (esp. L332-337) |
| No production `UPDATE documents SET chunk_count/entity_count` exists | grep over `edgequake/crates/**` — only test + unrelated matches |
| List handler reads `entity_count` from KV metadata | `edgequake-api/src/handlers/documents/query/list.rs:220-223` |
| Relational backfill reads `entity_count` from the dead column | `edgequake-api/src/document_read_model.rs:107,132-144` |
| Relational schema has no cost/token columns | `migrations/001_init_database.sql` (documents table DDL) |
| Dashboard aggregate entity_count uses AGE (correct) | `edgequake-api/src/handlers/workspaces/stats.rs:214-226` |
| `final_status` correctly flags 0-entities as `partial_failure` | `edgequake-api/src/processor/text_insert.rs:1062-1071` |
| Workspace-scope drift drops KV rows for legacy uploads | `edgequake-api/src/handlers/documents/query/list.rs:306-312` + file 11 §D |

---

## 9. Why the existing plan (file 10, file 12) did not catch this

- File 10's `R-DRY-03` flagged `documents.chunk_count`/`entity_count` as
  "write-only debt, not a read-path bug" because at the time the **read path
  was KV**. The P5-01 relational backfill then **made the dead column a
  read-path input** — flipping the risk from LOW to CRITICAL. The plan did not
  revisit R-DRY-03 after P5-01 landed.
- File 12 §4.5 verified the status decision tree but did not trace where the
  per-row `entity_count` *cell* is materialized, so it missed that the cell's
  relational feed is never refreshed.
- File 11 fixed the *count* KPI (`max(pg, kv)`) but did not extend the fix to
  the *per-row* entity/cost cells.

This is a classic **regression introduced by a partial fix**: P5-01 wired the
relational table into the read path for documents, but the relational table's
`entity_count`/`chunk_count`/cost columns were never wired into the write path.

---

## 10. Diagnosis summary

| # | Root cause | Severity | Symptom it explains | Fixed by |
|---|------------|----------|---------------------|----------|
| RC-1 | Relational `documents.entity_count`/`chunk_count` never updated post-ingestion | **CRITICAL** | All rows show 0 entities when served from relational backfill | Plan-17 P-A1, P-A2 |
| RC-2 | Per-row `entity_count` read authority undefined; no AGE fallback | **HIGH** | No recovery when both KV and PG feeds fail | Plan-17 P-A3 |
| RC-3 | Workspace-scope drift unmitigated for legacy KV metadata | **MEDIUM** | KV rows dropped → relational backfill used → 0 | Plan-17 P-A4 (backfill) |
| RC-4 | StorageInvariant trusts the dead relational column | **MEDIUM** | Inspector cannot detect this bug | Plan-17 P-B1 |
| RC-5 | `StatusCounts` treats NULL status as completed | **LOW** | "Looks completed" perception | Plan-17 P-B2 |

---

## 11. What this is NOT

- **Not** an LLM extraction failure. The status decision tree would have marked
  those `partial_failure`, and the dashboard aggregate shows 412 entities for
  the same workspace (file 11 §D).
- **Not** a frontend rendering bug. The UI faithfully renders the JSON the API
  returns.
- **Not** a missing CQRS sync. The `entities` table sync (file 12 §2 R-DRY-01)
  is a separate concern; the per-document `documents.entity_count` column is a
  denormalized cache that was never wired to any writer.

---

## 12. Task logs

Actions: Re-verified the full per-row entity_count read path (list.rs → document_read_model.rs → pdf_storage_impl.rs → status_updates.rs); confirmed via grep that no production code updates `documents.chunk_count`/`entity_count`; cross-checked schema DDL for cost/token columns (absent); confirmed the dashboard aggregate uses AGE (correct) while the per-row cell does not.

Decisions: Classified RC-1 (dead relational columns) as CRITICAL and the direct cause of the screenshot; classified RC-2 (no AGE fallback for per-row) as HIGH; linked RC-3 to file 11's workspace-scope drift; flagged that file 10 R-DRY-03 must be re-elevated after P5-01 made the dead column a read-path input.

Next steps: Author file 17 (battle-tested improvement plan) with Phase A (read-authority + write-path fix), Phase B (invariants + UI integrity), Phase C (deletion + saga symmetry), Phase D (query robustness), and Phase E (legacy backfill).

Lessons/insights: A partial read-model fix (P5-01) that wires a stale write-only column into the read path is a **regression multiplier**: it converts silent write-only debt into a user-visible defect. The lesson is that any read-authority switch must be paired with a write-path audit of every column the new authority exposes.
