# 11 — UX Shows 0 Documents: Root Cause Assessment (Code Is Law)

Date: 2026-06-25
Scope: Why Dashboard shows 0 uploaded documents while entities/relationships are non-zero.
Method: First-principles trace from UI rendering path to API handlers and storage tables, then live data verification.

---

## Executive finding

The UI is behaving according to current code, but the storage model is split and drifting:

1. Dashboard document KPI prefers workspace stats document_count first.
2. Workspace stats document_count is computed from KV metadata only.
3. Existing documents currently live in PostgreSQL documents table for many workspaces, but corresponding KV metadata rows are mostly scoped to a different (legacy default) workspace.
4. Result: document_count resolves to 0 in UI, even when relational documents rows exist.

This is a source-of-truth divergence, not a rendering bug.

---

## First-principles trace

### A) UI count source precedence

In root dashboard route, document KPI is:

- statsData.document_count first
- then documentsData.total fallback
- then documentsData.items.length fallback

See implementation in app page:
- edgequake_webui/src/app/page.tsx

Because statsData.document_count is returned (0), the fallback to documentsData is never used.

### B) Workspace stats path

Workspace stats handler explicitly uses KV as authoritative source for documents:

- Comment says PostgreSQL documents is "currently empty"
- Code uses try_kv_storage_stats for document_count
- document_count increments only when metadata.workspace_id parses and equals requested workspace_id

See:
- edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs

### C) Documents list path

Documents endpoint is strict:

- If tenant/workspace context missing, returns empty list immediately
- It scans KV metadata/chunk keys and filters by workspace/tenant metadata match

See:
- edgequake/crates/edgequake-api/src/handlers/documents/query/list.rs
- edgequake/crates/edgequake-api/src/workspace_scope.rs

### D) Storage reality check (live data)

Observed from live database:

- documents table rows: 542
- chunks table rows: 0
- entities table rows: 0
- relationships table rows: 0

Yet in the currently selected workspace context (Mistral / Default Workspace):

- workspace stats document_count: 0
- documents list total: 0
- workspace stats entity_count: 412

And for this same workspace there are relational rows:

- workspaces (Mistral / Default Workspace) has 7 rows in documents table

But KV metadata distribution is mostly:

- tenant_id = 00000000-0000-0000-0000-000000000002
- workspace_id = 00000000-0000-0000-0000-000000000003

So metadata in KV does not align with selected modern workspace IDs.

Conclusion: stats/list queries are reading scoped KV metadata that is absent for current workspace IDs, while documents exist in relational table.

---

## Root cause

Primary root cause:

Storage source-of-truth inconsistency for document lifecycle state.

Concretely:

- UX count logic depends on workspace stats.
- Workspace stats computes documents from KV metadata only.
- Existing records are present in relational documents for many workspaces but not represented in KV metadata for those same workspace IDs.
- Therefore UI shows 0.

Secondary contributing factors:

- Strict tenant/workspace filtering on document list endpoint amplifies any metadata scope mismatch.
- Comment assumptions in stats handler are stale (PostgreSQL not empty anymore).

---

## Gap assessment against specs/021-storage-study

### Gap G1: Spec source-of-truth vs runtime implementation mismatch

Spec README states:

- Document metadata primary: documents table
- KV as shadow

Current runtime implementation:

- Workspace stats and documents list read KV as effective source for UI count/list.

Impact:

- UI does not reflect relational document rows when KV shadow is missing or out-of-sync.

### Gap G2: Plan assumptions stale

Current plan text says PostgreSQL docs "currently empty" in stats comments, but live DB has 542 rows.

Impact:

- Operational decisions based on stale assumption lead to wrong fast path/authoritative path.

### Gap G3: Missing cross-store invariant gate in hot path

No enforced invariant before serving dashboard that checks:

For each relational document in workspace W, a KV metadata record exists with workspace_id = W.

Impact:

- Silent divergence reaches end-users as 0 counts.

### Gap G4: CQRS sync status incomplete for documents (not only entities)

Specs/plans emphasize entities dual-write and sync mode, but document metadata synchronization between documents table and KV is not guarded as a first-class invariant in serving paths.

Impact:

- Read model used by UI can be stale or empty despite relational truth.

---

## Code-level evidence map

- UI document KPI precedence:
  - edgequake_webui/src/app/page.tsx
- Dashboard workspace route also reads stats/docs via selected context:
  - edgequake_webui/src/app/(dashboard)/page.tsx
- Workspace stats authoritative KV path:
  - edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs
- Documents list strict context and KV scan/filter:
  - edgequake/crates/edgequake-api/src/handlers/documents/query/list.rs
- Metadata tenant/workspace matcher:
  - edgequake/crates/edgequake-api/src/workspace_scope.rs
- Spec source-of-truth table:
  - specs/021-storage-study/README.md
- Implementation plan status tables:
  - specs/021-storage-study/plan.md

---

## Recommended remediation (ordered)

> **Status (2026-06-25)**: Item 1 Option A implemented as **P5-01** — see `document_read_model.rs`, `stats.rs`, `list.rs`. Regression tests: `e2e_zero_documents_spec021.rs`, `spec021-zero-documents-fix.spec.ts`.

1) Decide and codify one read authority for dashboard document_count.

Option A (aligned with README): use relational documents table as primary for document_count, keep KV as fallback.
Option B: keep KV primary, but implement mandatory relational-to-KV backfill and continuous sync checks before serving.

2) Add hard invariant check to startup and periodic monitor:

- Per workspace mismatch count:
  - relational documents rows
  - KV metadata rows with matching workspace_id
- Expose mismatch in health/admin endpoint and metrics.

3) Add repair job for historical drift:

- Backfill KV metadata for relational documents per workspace/tenant.
- Preserve IDs and timestamps where possible.

4) Update stats handler comments and docs to match current production truth.

5) Add regression tests:

- Given relational docs present and KV missing for workspace, dashboard API should either:
  - return relational count (if relational-first), or
  - fail with explicit consistency warning/status rather than silently returning 0.

---

## Decision note

This is not a frontend display bug. The UI reflects backend counters correctly.
The defect is architectural drift between document storage layers and the read path currently chosen by workspace stats/list endpoints.
