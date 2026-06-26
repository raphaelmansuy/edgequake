# 17 — Battle-Tested Improvement Plan (Consolidated, Code-Verified)

> **Spec**: 021-storage-study
> **File**: 06-first-principles/17-battle-tested-improvement-plan-consolidated.md
> **Date**: 2026-06-25
> **Method**: "Code is Law" + First Principles. This plan is the **authoritative**
> improvement plan for the storage, ingestion, and query layers. It supersedes
> `10-battle-tested-improvement-plan.md` and `plan.md` wherever they conflict,
> and integrates all code-verified findings from files 11–16.
> **Principles**: DRY (one authoritative definition per fact), SOLID (SRP per
> module, ISP for storage traits, DIP for sinks), First Principles (storage must
> be a **recoverable, invariant-guarded state machine**, not a best-effort cache).

---

## 0. How to read this plan

- Each work item has: **ID**, **goal**, **files**, **edge cases covered**,
  **risk**, **estimate**, **acceptance test**.
- Items are ordered by **user impact** (fix the screenshot first), then
  **correctness** (sagas/invariants), then **operability** (monitoring/admin),
  then **debt** (refactors).
- Already-completed work from `plan.md` (P0–P5 except the gaps below) is
  **not repeated**. Only **new, corrected, or elevated** work is listed.
- Status legend: ⬜ Not Started | 🔄 In Progress | ✅ Done | 🧪 Tested

---

## 1. Current state, honestly

| Layer | Verdict | Evidence |
|-------|---------|----------|
| Schema (migrations 001–040) | ✅ Solid | CQRS read model in place; embedding dup columns dropped (file 12 §2 R-DRY-02) |
| KV key schema | ✅ Solid | `kv_keys::*` centralized (file 12 §2 R-DRY-05) |
| Vector ID typing (writer) | ✅ Solid | `vector_id.rs` (file 12 §3 R-SOLID-04) |
| Orchestrator ingestion saga | ✅ Solid (vectors) | `ingestion.rs::fail_with_chunk_vector_rollback` (file 12 §4.2) |
| Processor ingestion saga | ❌ **Missing** | `text_insert.rs` has no rollback on graph-batch failure (file 12 §4.2, G-02) |
| Status decision tree | ✅ Solid | `text_insert.rs` L1043-1086 (file 12 §4.5) |
| Dashboard document_count | ✅ Fixed | P5-01 `max(pg, kv)` (file 11, file 12 §7 P5-01) |
| Dashboard entity_count | ✅ Correct | AGE `node_count_by_workspace` (stats.rs L214) |
| **Per-row entity_count cell** | ❌ **Broken** | Relational `documents.entity_count` never updated (file 16 §3) |
| Deletion cross-store cleanup | ❌ **Asymmetric** | Orchestrator delete skips `documents` row + entity vectors on partial update (file 12 §4.4, G-03) |
| StorageInspector invariants | ⚠️ Partial | Startup check exists; hourly monitor + admin endpoint skipped (file 12 §6 G-09); R-DRY-03 invariant trusts a dead column (file 16 §6) |
| Query chunk retrieval contract | ⚠️ Fragile | `source_chunk_ids` ↔ `kv_keys::doc_chunk` ID convention is implicit (file 12 §5.2) |
| Graph materialization capacity | ✅ Fixed | P5-03 raised concurrency + structured SSE retry (file 15) |
| Frontend API layer | ✅ Fixed | P5-02 DRY/SOLID split (file 14) |

**The single biggest residual risk is path asymmetry**: the orchestrator path
and the processor path do not enforce the same invariants, and the read path
was switched (P5-01) to a relational column the write path never refreshes.

---

## 2. First-principles storage model (the target)

A robust storage layer is a **recoverable state machine** with three properties:

1. **Single Source of Truth (SSOT) per fact**: each fact (doc count, entity
   count, chunk content, etc.) has exactly one authoritative store. Other
   stores holding the same fact are **read models** that are (a) derived from
   the SSOT and (b) guarded by an invariant that detects drift.
2. **Write-path closure**: any column a read path consults must be written by
   a code path that runs to completion on every successful ingestion. A column
   that is read but never written is a **latent bug**.
3. **Saga symmetry**: every cross-store write has a compensating action on
   failure, and that compensation is identical across all entry points
   (orchestrator, processor, deletion, re-ingestion).

The current code violates (2) for `documents.entity_count`/`chunk_count` and
violates (3) for the processor path and the deletion path. This plan closes
both.

### 2.1 Source-of-truth map (target)

| Fact | SSOT | Read models | Invariant |
|------|------|-------------|-----------|
| Document existence + status | relational `documents` table | KV `{doc_id}-metadata` (legacy) | INV-A: every `documents` row has a KV metadata row with matching `workspace_id` (or is backfilled) |
| Chunk content | KV `{doc_id}-chunk-{n}` | — | INV-B: KV chunk count for `doc_id` == `documents.chunk_count` (after P-A2) |
| Entity (traversal) | AGE graph | relational `entities` (CQRS, when sync enabled) | INV-C: AGE node count for `doc_id` chunk-prefix == per-doc entity count served to UI |
| Entity (per-doc count for UI) | AGE graph | KV `metadata.entity_count`, relational `documents.entity_count` | INV-C (above) — both read models must equal AGE within tolerance |
| Embeddings | workspace-scoped vector tables | — | INV-D: every chunk vector has a retrievable KV chunk; every entity vector has an AGE node |
| Cost / tokens | KV `metadata.cost_usd`/`input_tokens`/... | (none — relational has no such columns) | INV-E: relational backfill must not claim cost (UI shows `-` when KV missing — acceptable, documented) |

---

## 3. Phase A — Read Authority & Write-Path Closure (fixes the screenshot)

> **Priority**: P0. This is the direct fix for the "Completed / 0 entities" screenshot.

### P-A1 — Refresh relational `documents.chunk_count`/`entity_count` on every status update ⬜

**Goal**: close the write-path gap (file 16 §3). Make the relational columns
that P5-01 exposed to the read path actually reflect ingestion outcomes.

**Files**:
- `edgequake/crates/edgequake-storage/src/pdf_storage.rs` (trait)
- `edgequake/crates/edgequake-storage/src/adapters/postgres/pdf_storage_impl.rs`
- `edgequake/crates/edgequake-storage/src/adapters/memory/pdf.rs`
- `edgequake/crates/edgequake-api/src/processor/status_updates.rs` (call site)

**Design**:
- Add a new trait method `update_document_stats(&self, id: &Uuid, chunk_count: i32, entity_count: i32, relationship_count: i32, cost_usd: Option<f64>, input_tokens: Option<i64>, output_tokens: Option<i64>, status: &str) -> Result<()>`.
- SQL (postgres):
  ```sql
  UPDATE documents SET
      chunk_count        = $2,
      entity_count       = $3,
      relationship_count = $4,
      cost_usd           = $5,
      input_tokens       = $6,
      output_tokens      = $7,
      status             = $8,
      updated_at         = NOW()
  WHERE id = $1
  ```
  Requires a migration adding `relationship_count`, `cost_usd`, `input_tokens`,
  `output_tokens` columns (all nullable) to `documents`.
- Call it from `update_document_status_with_stats` **in addition to** the KV
  write, best-effort (warn on error, never fail ingestion).

**Edge cases covered**:
- E1: Postgres feature disabled → trait impl is a no-op (memory adapter) or
  `#[cfg(not(feature="postgres"))]` stub returns `Ok(())`.
- E2: `documents` row not yet inserted (race with `ensure_document_record`) →
  `UPDATE ... WHERE id = $1` affects 0 rows; log a warn and rely on the next
  `ensure_document_record` + a follow-up stats update. To make this robust,
  `ensure_document_record` should be called **before** extraction starts (it
  already is, on the async path) and `update_document_stats` should
  upsert-on-conflict the stats columns as a fallback.
- E3: Re-ingestion of an existing doc → `UPDATE` overwrites with the new
  stats (idempotent given the same content).
- E4: Status `failed`/`partial_failure` → still update the stats so the UI can
  show "extracted 3 of 10 chunks" rather than `0`.
- E5: Negative or overflow values → clamp at 0 and at `i32::MAX` in the trait
  impl; never trust the pipeline counter blindly.

**Risk**: LOW (additive column + best-effort write; read path already consults
the columns).
**Estimate**: 0.5 day code + 0.5 day migration + tests.
**Acceptance test**: ingest a doc through the async processor with Postgres
feature on; assert `SELECT entity_count FROM documents WHERE id = $1` returns
the same value as `metadata.entity_count` in KV and as
`graph_storage.node_count_by_chunk_prefix(doc_id)`.

### P-A2 — Make `update_document_stats` idempotent + race-safe ⬜

**Goal**: handle E2 robustly without relying on call ordering.

**Design**: use `INSERT ... ON CONFLICT (id) DO UPDATE SET chunk_count = ...,
entity_count = ..., status = ..., updated_at = NOW()` in the postgres impl,
mirroring `ensure_document_record`'s pattern. This collapses P-A1's UPDATE and
the ensure-record INSERT into a single upsert that always wins. The separate
`ensure_document_record` remains for the pre-extraction "row exists" guarantee;
`update_document_stats` is the post-extraction "row is correct" guarantee.

**Edge cases**:
- E6: Two concurrent ingestions of the same `doc_id` (re-upload) → last writer
  wins; acceptable because the doc content is the same.
- E7: `update_document_stats` runs after `delete_document` → upsert resurrects
  the row. Prevent by checking a tombstone flag (P-C2) or by ordering
  deletion-after-stats within the processor (the processor already clears the
  checkpoint last; deletion is a separate API).

**Risk**: LOW. **Estimate**: 0.25 day. **Acceptance**: E2 unit test.

### P-A3 — Per-row entity_count read authority: AGE fallback ⬜

**Goal**: when both F-KV and F-PG disagree or are missing, fall back to the
authoritative AGE graph (file 16 §4).

**Files**:
- `edgequake/crates/edgequake-storage/src/traits/graph_read_view.rs` (add
  `node_count_by_source_prefix(prefix: &str) -> Result<usize>`)
- `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/analytics_ops.rs`
- `edgequake/crates/edgequake-api/src/document_read_model.rs`
- `edgequake/crates/edgequake-api/src/handlers/documents/query/list.rs`

**Design**:
- Add a `GraphStorageReadOps::node_count_by_source_prefix` that runs a Cypher
  `MATCH (n) WHERE any(s IN n.source_ids WHERE s STARTS WITH $prefix) RETURN
  count(n)`.
- In `list_relational_document_summaries`, after fetching rows, batch-query
  `node_count_by_source_prefix(format!("{}-chunk-", doc_id))` for each doc and
  fill `entity_count = max(relational, age_count)`. Batch to avoid N+1 (one
  Cypher with `UNWIND` over the doc-id prefixes).
- Apply the same `max(kv, pg, age)` resolution in the KV branch of `list.rs`
  when `meta.entity_count` is `None` or `0` but chunks exist.

**Edge cases**:
- E8: AGE query fails → `unwrap_or(0)` keeps the existing value; do **not**
  fail the list request.
- E9: A doc has chunks but 0 entities legitimately (e.g. a pure-data table) →
  AGE returns 0, `max(0,0,0) = 0`; the status would be `partial_failure`
  already (FIX-2). UI should show `0` with the `partial_failure` badge, not
  `0` with `completed`.
- E10: N+1 query blowup for 500 docs → batch via `UNWIND` + cap the list page
  size at `RB-API-001` (100).
- E11: Legacy AGE nodes without `source_ids` (only `source_id`) → the Cypher
  must check both properties (matches `collect_source_references` logic).

**Risk**: MEDIUM (new graph query in the hot list path; must be batched and
timeout-guarded by `RB-DB-002`).
**Estimate**: 1 day.
**Acceptance test**: the screenshot scenario — KV metadata scoped to legacy
workspace, relational row present — produces correct per-row entity_count
matching the dashboard aggregate.

### P-A4 — Legacy workspace-scope backfill ⬜

**Goal**: close the KV-vs-relational workspace_id drift (file 11 §D, file 16 §5)
so the KV branch is used again and P-A3's fallback is a safety net, not the
primary path.

**Files**:
- new `edgequake/crates/edgequake-api/src/storage_reconcile.rs`
- `edgequake/crates/edgequake-api/src/state/migration_bootstrap.rs` (spawn)

**Design**:
- A one-shot reconciliation job (paginated, idempotent) that, for every
  `documents` row, ensures a KV `{doc_id}-metadata` entry exists with the
  correct `workspace_id`/`tenant_id`. If a KV entry exists with a legacy
  workspace_id, **and** the relational row has the modern workspace_id,
  update the KV entry's `workspace_id`/`tenant_id` to match the relational
  truth (with a backup of the old value under `legacy_workspace_id`).
- Gate behind a config flag `reconcile_legacy_workspace_scope = true` (default
  false); run once at startup if enabled; log every changed key.

**Edge cases**:
- E12: A doc genuinely belongs to the legacy workspace (not drift) → skip; the
  job must only rewrite when the relational `documents.workspace_id` differs
  **and** the relational row is the newer one (heuristic: created_at newer than
  the KV entry). Conservative: require an explicit admin invocation, not
  auto-run.
- E13: Race with ongoing ingestion → skip docs whose KV metadata
  `status == "processing"`.
- E14: Partial run crash → idempotent (re-running is safe; each step is an
  upsert keyed by doc_id).

**Risk**: MEDIUM (rewrites KV metadata; requires admin approval). **Estimate**:
1.5 days. **Acceptance**: after backfill, the KV branch of `list.rs` returns
the 7 PDFs with correct `entity_count` and `cost_usd` for the modern workspace.

---

## 4. Phase B — Invariants & UI Integrity

### P-B1 — Rewrite the R-DRY-03 invariant to compare against AGE ⬜

**Goal**: file 16 §6 — the inspector must not trust the dead relational column.

**Files**: `edgequake/crates/edgequake-api/src/storage_inspector.rs`

**Design**: invariant INV-C (per §2.1 table):
- For a sample of N documents (configurable, default 50, paginated):
  - compute `kv_entity_count = metadata.entity_count`,
  - compute `pg_entity_count = documents.entity_count` (after P-A1, this is
    meaningful),
  - compute `age_entity_count = node_count_by_source_prefix("{doc_id}-chunk-")`,
  - flag CRITICAL if `kv != age` or `pg != age` (tolerance 0; any drift is a
    bug after P-A1).
- Surface in the admin endpoint (P-D2) and as a startup WARNING when > 5% drift.

**Edge cases**:
- E15: Doc with 0 entities legitimately → all three equal 0; no false positive.
- E16: Doc mid-ingestion → skip docs with `status IN ('processing','pending')`.
- E17: Sample bias → rotate the sample window each run; record the sampled
  `doc_id`s in the report.

**Risk**: LOW. **Estimate**: 0.5 day. **Acceptance**: invariant fires on a
synthetic drift fixture; passes after P-A1.

### P-B2 — `StatusCounts` must not treat NULL status as completed ⬜

**Goal**: file 16 §7.

**Files**: `edgequake/crates/edgequake-api/src/handlers/documents/query/list.rs`

**Design**: change the `completed` filter (L469-476) to require
`status.as_deref() == Some("completed") || status.as_deref() == Some("indexed")`.
Add a new `unknown` bucket for `None`. UI shows `unknown` as a neutral grey
badge (not green checkmark).

**Edge cases**:
- E18: Relational backfill row with `status = NULL` → counted as `unknown`,
  not `completed`. UI badge reflects the ambiguity.

**Risk**: LOW. **Estimate**: 0.25 day. **Acceptance**: unit test on
`StatusCounts` with a NULL-status row.

### P-B3 — Add INV-D (orphan vector detection) ⬜

**Goal**: file 12 §5.1 — detect orphaned chunk/entity vectors and orphaned
workspace-scoped vector tables.

**Files**: `edgequake/crates/edgequake-api/src/storage_inspector.rs`

**Design**:
- INV-D1: sample N chunk vectors; for each, assert the KV chunk key exists.
  Flag CRITICAL on miss (orphan embedding).
- INV-D2: sample N entity vectors (`metadata.type == "entity"`); assert an AGE
  node with `id == metadata.entity_name` exists.
- INV-D3: enumerate `eq_*_ws_*_vectors` tables; for each, assert the workspace
  exists in `workspaces`. CAUTION-tier repair = table drop (admin approval).

**Edge cases**:
- E19: Vector stored under a workspace that was just deleted → INV-D3 fires;
  repair is a drop (destructive, requires admin).
- E20: Sampling misses the orphans → pair sampling with a count check
  (`COUNT(*)` of vectors minus `COUNT(DISTINCT document_id)` of KV chunks →
  if ratio > 1.1, fall back to full scan).

**Risk**: LOW (read-only). **Estimate**: 0.5 day.

---

## 5. Phase C — Deletion & Saga Symmetry

### P-C1 — Processor-path saga compensation ⬜

**Goal**: file 12 §4.2 (G-02, P3-06) — the processor path must roll back
chunk + entity vectors on graph-batch failure, exactly as the orchestrator
path does.

**Files**:
- new `edgequake/crates/edgequake-api/src/processor/compensation.rs`
- `edgequake/crates/edgequake-api/src/processor/text_insert.rs`

**Design**:
- Extract `compensate_orphan_chunk_vectors` from `ingestion.rs` into a shared
  `compensation` module (DRY: one implementation, two callers).
- In `process_text_insert`, capture `written_chunk_vector_ids` and
  `written_entity_vector_ids` in `Vec<String>` as they are written (L682, L966).
- After `upsert_nodes_batch` returns `Err` or `errors > 0`, call
  `compensate_orphan_vectors(vector_storage, &written_chunk_vector_ids,
  &written_entity_vector_ids, &cause)`; emit a `quarantine` log on cleanup
  failure; mark the document `partial_failure` (not `failed`, because chunks
  are still in KV and can be re-merged).

**Edge cases**:
- E21: Compensation itself fails → quarantine log; do not mask the original
  error (same principle as `fail_with_chunk_vector_rollback`).
- E22: Partial graph write succeeded before the failure → graph MERGE is
  idempotent and source-tracked; safe to leave for re-run or deletion.
- E23: Cancellation arrives mid-compensation → check the cancel token between
  the two compensation steps; if cancelled, log and exit (cleanup is
  best-effort).
- E24: Re-ingestion of the same doc_id → chunk IDs are deterministic
  (`{doc_id}-chunk-{n}`), so deleting them is idempotent.

**Risk**: MEDIUM. **Estimate**: 1 day. **Acceptance**: inject a graph failure
in a test; assert chunk + entity vectors are deleted and the doc is
`partial_failure`.

### P-C2 — `DocumentDeletionCoordinator` (single deletion entry point) ⬜

**Goal**: file 12 §4.4 (G-03, P3-07) — unify the API delete handler and the
orchestrator `delete_document` so both clean **all** stores in the same order.

**Files**:
- new `edgequake/crates/edgequake-core/src/orchestrator/deletion_coordinator.rs`
- `edgequake/crates/edgequake-core/src/orchestrator/deletion.rs` (delegate)
- `edgequake/crates/edgequake-api/src/handlers/documents/delete/single.rs`
- `edgequake/crates/edgequake-api/src/handlers/documents/storage_helpers.rs`
  (`delete_document_for_reingestion`)

**Design**: the coordinator executes, in order, best-effort with quarantine logs:
1. KV cleanup (chunk keys, metadata, content) — current behavior.
2. Graph cleanup (nodes + edges by source-prefix) — current behavior.
3. Relational `entities` sync (`remove_entity_sources`) — current behavior.
4. **NEW**: vector cleanup — chunk vectors by chunk IDs (from KV scan), entity
   vectors for fully-removed nodes, and **entity vector metadata refresh** for
   partially-updated nodes (P-C3).
5. **NEW**: relational `documents` row delete (`delete_document_record`) —
   currently only the API handler does this; the orchestrator path skips it.
6. Stats cache invalidation.

**Edge cases**:
- E25: Step 1 succeeds, step 2 fails → orphan graph nodes; quarantine log;
  the coordinator returns a partial result with `entities_removed: 0,
  chunks_deleted: N` so the caller knows. Re-running deletion is idempotent
  (step 1 finds 0 chunks; step 2 retries).
- E26: `delete_document_for_reingestion` calls the coordinator → after
  deletion, the re-ingestion path must call `ensure_document_record` (it
  already does via the processor) to resurrect the row before `update_document_stats`
  runs (P-A2 upsert handles this race).
- E27: Deletion of a doc whose KV metadata is missing (legacy drift) → step 1
  finds 0 chunks but step 2 still finds graph nodes by source-prefix; the
  coordinator must not require KV to proceed.
- E28: Concurrent deletion + ingestion of the same doc_id → coordinator holds
  a per-doc_id mutex (or relies on the task queue's single-writer per doc);
  ingestion after deletion starts a fresh row.

**Risk**: MEDIUM. **Estimate**: 1.5 days. **Acceptance**: contract test
(P-E3) ingest → delete → assert all stores empty.

### P-C3 — Entity vector metadata refresh on partial deletion ⬜

**Goal**: file 12 §4.4 (P3-08) — when an entity is partially updated (some
source chunks removed), refresh its `entity:{name}` vector metadata so it no
longer references the removed chunks.

**Files**: `edgequake/crates/edgequake-core/src/orchestrator/deletion_coordinator.rs`

**Design**: after step 3 (relational sync) for a partially-updated node, call
`vector_storage.upsert_entity(entity_name, embedding, refreshed_metadata)` where
`refreshed_metadata.source_chunk_ids = remaining`. Requires fetching the
existing embedding (or re-embedding if the entity description changed — out of
scope; just refresh metadata).

**Edge cases**:
- E29: Entity has no embedding (never embedded) → skip silently.
- E30: Re-embedding is needed because the description changed → out of scope;
  log a warning; a future "entity re-embedding" job can handle it.

**Risk**: LOW. **Estimate**: 0.5 day.

---

## 6. Phase D — Operability (Inspector + Admin)

### P-D1 — Re-enable the hourly invariant monitor ⬜

**Goal**: file 12 §6 (G-09, P4-08) — the inspector exists but is not spawned.

**Files**: `edgequake/crates/edgequake-api/src/state/task_runtime.rs`

**Design**: spawn a `tokio::time::interval` task that calls
`StorageInspector::inspect` every hour, records metrics, and runs
`auto_repair_safe` for SAFE-tier findings. Honor `RB-WRK-003` processing
timeout; abort the run if it exceeds 60s.

**Edge cases**:
- E31: Inspector run overlaps with itself → guard with an `AtomicBool` running
  flag; skip if already running.
- E32: DB unavailable → log and skip; do not crash the runtime.

**Risk**: LOW. **Estimate**: 0.5 day.

### P-D2 — Admin inspection + repair endpoint ⬜

**Goal**: file 12 §6 (P4-09) — `/api/v1/admin/storage/inspect` (read-only) and
`/api/v1/admin/storage/repair?dry_run=true` (admin-only).

**Files**:
- new `edgequake/crates/edgequake-api/src/handlers/admin/storage.rs`
- `edgequake/crates/edgequake-api/src/lib.rs` (route registration)

**Design**:
- GET `/inspect` returns the full `StorageReport` (JSON) — all invariants,
  per-doc drift sample, orphan counts.
- POST `/repair?dry_run=true|false` — dry_run lists the actions; non-dry-run
  executes `auto_repair_safe` (SAFE tier only); CAUTION tier (e.g. dropping
  orphan vector tables) requires a separate `?confirm=<token>` derived from the
  dry_run report hash.

**Edge cases**:
- E33: Non-admin access → 403 (reuse existing admin middleware).
- E34: Concurrent dry_run and non-dry-run → the confirm token prevents
  accidental execution of a stale plan.

**Risk**: LOW (read-only + gated). **Estimate**: 0.5 day.

### P-D3 — Silent no-op detection for `entity_sync_mode` ⬜

**Goal**: file 12 §4.3 (G-04, P4-06) — warn at startup if sync mode is enabled
but no rows are landing.

**Files**: `edgequake/crates/edgequake-api/src/storage_inspector.rs`,
`edgequake/crates/edgequake-api/src/state/postgres.rs`

**Design**: invariant INV-04b: if `entity_sync_mode ∈ {dual_write, full}` AND
`entities.sync_status='synced'` count == 0 AND AGE node count > 0 → startup
WARNING "CQRS sync enabled but no rows synced; check migration 039/040 and
PostgresEntitySink error logs".

**Edge cases**:
- E35: Fresh deployment with no docs yet → AGE node count == 0; invariant
  does not fire.

**Risk**: LOW. **Estimate**: 0.5 day.

---

## 7. Phase E — Query Robustness & Contract Tests

### P-E1 — Migrate query-path entity vector decoding to `VectorId` ⬜

**Goal**: file 12 §3 (R-SOLID-04 reader side, P5-02) — replace raw
`metadata.entity_name` string lookup with `VectorId::from_storage_id`.

**Files**: `edgequake/crates/edgequake-query/src/strategies/local.rs`,
`edgequake/crates/edgequake-query/src/strategies/global.rs`

**Risk**: LOW. **Estimate**: 1 day.

### P-E2 — Writer/reader chunk-ID contract test ⬜

**Goal**: file 12 §5.2 (P5-03) — catch any change to the chunk-ID scheme that
would break retrieval.

**Files**: new `edgequake/crates/edgequake-core/tests/contract_chunk_id.rs`

**Design**: ingest a tiny doc through the real pipeline; assert every ID in
`entity.source_chunk_ids` is retrievable via `kv_storage.get_by_ids`.

**Edge cases**: E36: chunker changes ID format → test fails loudly.

**Risk**: LOW. **Estimate**: 0.5 day.

### P-E3 — Ingest → delete → all-stores-empty contract test ⬜

**Goal**: lock in P-C2.

**Files**: new `edgequake/crates/edgequake-core/tests/contract_deletion.rs`

**Design**: ingest → delete via coordinator → assert: KV metadata/chunks
gone, AGE nodes by source-prefix gone, chunk vectors gone, entity vectors
gone, `documents` row gone, `entities` rows gone (if sync enabled).

**Edge cases**: E37: partial-update branch → assert entity vector metadata
refreshed (P-C3).

**Risk**: LOW. **Estimate**: 0.5 day.

### P-E4 — Per-row entity_count regression test (the screenshot) ⬜

**Goal**: lock in P-A1 + P-A3.

**Files**: new `edgequake/crates/edgequake-api/tests/regression_zero_entities.rs`

**Design**:
- Setup: ingest a doc; delete its KV metadata (simulate legacy drift) so
  only the relational row remains.
- Assert: `GET /api/v1/documents` returns the doc with `entity_count` equal
  to the AGE node count for the doc (not 0), and `status` not falsely
  `completed` when the AGE count is 0.

**Edge cases**: E38: AGE fallback also fails → the test asserts a structured
`X-Storage-Drift: true` response header so the UI can show a drift badge
(future UX work, not in this plan).

**Risk**: LOW. **Estimate**: 0.5 day.

---

## 8. Phase F — Documentation Reconciliation

> Mirror of file 12 §7 Phase 6, plus the new findings. Do alongside each code
> change, not at the end.

| ID | Task | Files |
|----|------|-------|
| P-F1 | Update README source-of-truth table: per-row entity_count SSOT = AGE; read models = KV + relational (both guarded by INV-C). | `specs/021-storage-study/README.md` |
| P-F2 | Re-elevate R-DRY-03 from "write-only debt" to "CRITICAL read-path bug after P5-01" and link to file 16. | `specs/021-storage-study/05-risks/01-dry-violations.md` |
| P-F3 | Update `plan.md` with the Phase A–F items and mark P-A1..P-A4 as the fix for the screenshot. | `specs/021-storage-study/plan.md` |
| P-F4 | Update `stats.rs` L49 comment + `document_read_model.rs` module doc to state the per-row entity_count resolution order (KV → PG → AGE). | code comments |
| P-F5 | Add `documents.entity_count`/`chunk_count`/cost columns to the migration NOTES.md "Document lifecycle" row with the writer = `update_document_stats` (P-A1). | `edgequake/migrations/NOTES.md` |

---

## 9. Prioritized execution order

1. **P-A1 + P-A2** (write-path closure) — without this, every other fix is a
   read-side workaround. 1.25 days.
2. **P-A3** (AGE fallback) — fixes the screenshot immediately, even before
   backfill. 1 day.
3. **P-B1 + P-B2** (invariant + status integrity) — detect the bug class and
   stop false-green UI. 0.75 day.
4. **P-C1** (processor saga) — closes the highest-severity ingestion asymmetry. 1 day.
5. **P-C2 + P-C3** (deletion coordinator) — closes the highest-severity cleanup asymmetry. 2 days.
6. **P-D1 + P-D2 + P-D3** (operability) — operationalize detection. 1.5 days.
7. **P-A4** (legacy backfill) — permanent fix for the drift, after the above
   make detection safe. 1.5 days.
8. **P-E1..P-E4** (contract tests) — lock in the fixes. 2.5 days.
9. **P-F1..P-F5** (docs) — alongside each change.

**Total**: ~12 engineering days, ordered to ship user-visible value in days 1–3.

---

## 10. Edge-case registry (consolidated)

This registry is the single place every edge case considered in the plan is
listed. Every work item references its edge cases by E-id. New edge cases
discovered during implementation must be added here.

| ID | Edge case | Handled by |
|----|-----------|------------|
| E1 | Postgres feature disabled | P-A1 stub |
| E2 | `documents` row not yet inserted (race) | P-A2 upsert |
| E3 | Re-ingestion of existing doc | P-A1 idempotent UPDATE |
| E4 | `failed`/`partial_failure` status | P-A1 still updates stats |
| E5 | Negative/overflow counter | P-A1 clamp |
| E6 | Concurrent ingestion same doc_id | P-A2 last-writer-wins |
| E7 | Stats update after deletion | P-A2 tombstone/ordering |
| E8 | AGE query fails | P-A3 unwrap_or(0) |
| E9 | Legit 0-entity doc | P-A3 + status badge |
| E10 | N+1 graph query | P-A3 batched UNWIND |
| E11 | Legacy `source_id` vs `source_ids` | P-A3 Cypher checks both |
| E12 | Genuine legacy workspace (not drift) | P-A4 admin-gated |
| E13 | Backfill race with ingestion | P-A4 skip processing docs |
| E14 | Backfill crash | P-A4 idempotent |
| E15 | 0-entity drift false positive | P-B1 equal-0 ok |
| E16 | Mid-ingestion doc | P-B1 skip processing |
| E17 | Sample bias | P-B1 rotate window |
| E18 | NULL status | P-B2 unknown bucket |
| E19 | Workspace deleted | P-B3 INV-D3 drop |
| E20 | Sampling misses orphans | P-B3 count-ratio fallback |
| E21 | Compensation fails | P-C1 quarantine log |
| E22 | Partial graph write | P-C1 idempotent MERGE |
| E23 | Cancellation mid-compensation | P-C1 cancel token |
| E24 | Re-ingestion idempotency | P-C1 deterministic IDs |
| E25 | Multi-step deletion partial | P-C2 partial result |
| E26 | Re-ingestion after deletion | P-C2 + P-A2 upsert |
| E27 | KV missing, graph present | P-C2 no KV requirement |
| E28 | Concurrent delete + ingest | P-C2 per-doc mutex |
| E29 | Entity has no embedding | P-C3 skip |
| E30 | Description changed (re-embed needed) | P-C3 warn, out of scope |
| E31 | Inspector overlap | P-D1 running flag |
| E32 | DB unavailable | P-D1 skip |
| E33 | Non-admin access | P-D2 403 |
| E34 | Concurrent dry_run + real run | P-D2 confirm token |
| E35 | Fresh deployment | P-D3 AGE==0 skip |
| E36 | Chunker changes ID format | P-E2 test fails |
| E37 | Partial-update deletion | P-E3 assert metadata refresh |
| E38 | All feeds fail | P-E4 drift header |

---

## 11. What this plan does NOT do (and why)

| Not doing | Why |
|-----------|-----|
| Drop `documents.entity_count`/`chunk_count` columns | They are the natural per-doc read model once P-A1 wires the writer; dropping would force the list handler to call AGE for every row (costlier). |
| Make AGE the per-row SSOT and remove the relational column | AGE Cypher per-row is slower than a relational SELECT for list views; keep AGE as the invariant reference (P-B1) and the fallback (P-A3). |
| Synchronous 2PC across KV/AGE/vectors/relational | No transactional coordinator; best-effort + saga + invariant is the correct pattern. |
| Auto-run the legacy backfill (P-A4) | Rewriting KV metadata is destructive; require admin invocation. |
| Re-embed entities on partial deletion (P-C3 full) | Out of scope; metadata refresh is enough for retrieval correctness; full re-embedding is a future job. |

---

## 12. Success metrics

| Metric | Before | After Phase A | After Phase B+D |
|--------|--------|---------------|-----------------|
| Per-row `entity_count` accuracy vs AGE | 0% (screenshot) | 100% | 100% + drift-detected |
| `documents.entity_count` column populated | 0% | 100% | 100% |
| Orphan chunk vectors after graph failure (processor path) | possible | 0 (P-C1) | 0 |
| Deletion leaving `documents` rows (orchestrator path) | possible | 0 (P-C2) | 0 |
| Invariant drift detected within | never | never | 1 hour (P-D1) |
| Admin can trigger a dry-run repair | no | no | yes (P-D2) |

---

## 13. Task logs

Actions: Re-verified the full per-row entity_count path; confirmed no production writer updates `documents.entity_count`/`chunk_count`; cross-checked schema for cost/token columns (absent → explains `-` in screenshot); read the orchestrator saga, processor path, deletion path, inspector module, query engine, and budget catalog; integrated findings from files 10–16 into a single ordered plan with an edge-case registry.

Decisions: Made the relational `documents` columns the per-doc read model (wired by P-A1) rather than dropping them, because relational SELECT is cheaper than per-row Cypher for list views; kept AGE as the invariant reference and fallback (P-A3, P-B1); ordered the plan by user impact (P-A1/A2/A3 fix the screenshot in ~3 days) before correctness (P-C) and operability (P-D); made the legacy backfill (P-A4) admin-gated because it rewrites KV metadata.

Next steps: Implement P-A1 (migration + `update_document_stats` trait method + call site) and P-A3 (AGE fallback in the list handler) first; add the regression test P-E4 to lock the fix before declaring victory; then proceed to P-C1 (processor saga) and P-C2 (deletion coordinator).

Lessons/insights: The deepest lesson from this assessment is the **regression-multiplier pattern**: a partial read-model fix (P5-01) that promotes a write-only-dead column to a read-path input converts silent debt into a user-visible defect. Any future read-authority switch must be paired with a write-path audit of every column the new authority exposes. The edge-case registry (§10) is the structural defense against this class — every work item must enumerate the failure modes it handles.
