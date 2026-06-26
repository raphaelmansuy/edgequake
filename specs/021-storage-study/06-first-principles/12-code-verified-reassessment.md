# 12 — Code-Verified Reassessment & Plan Improvement

> **Spec**: 021-storage-study
> **File**: 06-first-principles/12-code-verified-reassessment.md
> **Date**: 2026-06-25
> **Method**: "Code is Law" — every claim below was re-verified by reading the
> actual production source in `edgequake/crates/...` and `edgequake/migrations/...`.
> Stale or inaccurate statements in earlier spec files are corrected here.
> **Supersedes (in part)**: 01-executive-summary.md, 05-risks/01-dry-violations.md,
> 05-risks/02-solid-violations.md, 10-battle-tested-improvement-plan.md, plan.md
> (only where this document explicitly states a correction).

---

## 0. How to read this document

- Each finding is tagged **VERIFIED** (re-read in source on 2026-06-25),
  **CORRECTED** (earlier spec file was inaccurate and is amended here), or
  **NEW** (gap not previously captured).
- File paths are repo-relative and point at the exact code that proves the claim.
- The plan at the end (§7) is the authoritative improvement plan; the older
  `plan.md` status table remains useful for tracking but its wording should be
  reconciled with this file where they diverge.

---

## 1. Source-of-Truth Map — Verified Against Code

| Data Domain | Write Source (Code-Proven) | Read Path Used in Production | Verdict |
| ----------- | -------------------------- | ---------------------------- | ------- |
| Document metadata | `eq_*_kv` key `{doc_id}-metadata` (`processor/status_updates.rs::ensure_document_source_type`, `processor/text_insert.rs` enrich) **AND** `documents` table via `pdf_storage.ensure_document_record` (text_insert.rs L1095-1136) | KV first: `handlers/workspaces/stats.rs::try_kv_storage_stats`, `handlers/documents/query/list.rs` | **VERIFIED + CORRECTED** |
| Chunk content | `eq_*_kv` key `{doc_id}-chunk-{n}` (`text_insert.rs` L586) | KV (`list.rs`, query engine) | VERIFIED |
| Chunk embeddings | `eq_*_vectors` (workspace-scoped) via `workspace_vector_storage.upsert` (`text_insert.rs` L682) | Vector store | VERIFIED |
| Entity (graph traversal) | AGE `Node` via `graph_storage.upsert_nodes_batch` (`text_insert.rs` L898) and `merger/entity.rs::merge_entity` | AGE graph (`query_pipeline.md` Step 3b/3c) | VERIFIED |
| Entity (analytics) | `entities` table **only when `entity_sync_mode != disabled`** (`postgres_entity_sink.rs`, `merger/entity.rs` L52-70, `text_insert.rs` L909-936) | Not yet wired into stats handler | **CORRECTED** — see §2 |
| Relationships (traversal) | AGE `EDGE` via `graph_storage.upsert_edges_batch` (`text_insert.rs` L1009) | AGE graph | VERIFIED |
| Entity embeddings | `eq_*_vectors` key `entity:{name}` (`text_insert.rs` L965) | Vector store (Local mode) | VERIFIED |
| PDF raw bytes | `pdf_documents.pdf_data` (`pdf_storage_impl.rs`) | `pdf_storage` | VERIFIED |

### Correction to README "Sources of Truth" table

The README table row:

> Document metadata | `documents` table | `eq_*_kv` (shadow — R-DRY-04)

is **inverted relative to the running code**. As of 2026-06-25:

- The **read path** for dashboard counts and document lists is **KV-first**
  (`try_kv_storage_stats` ignores `try_postgres_stats` entirely — it is
  `#[allow(dead_code)]` at stats.rs L130).
- The **write path** dual-writes both: KV unconditionally, `documents` table
  only on the async path AND only when `pdf_storage` is present
  (`text_insert.rs` L1095-1139, gated by `#[cfg(feature = "postgres")]`).

So the accurate statement is:

> Document metadata write: KV (always) + `documents` table (best-effort dual-write when Postgres feature active). Read authority for UI: **KV**. The `documents` table is currently a *secondary write* that the read path does not consult.

This is the root cause already identified in `11-ux-zero-documents-root-cause-assessment.md` — confirmed here at code level.

---

## 2. DRY Violations — Re-Assessed

### R-DRY-01 — Entities/relationships duplicated by AGE graph — **CORRECTED**

- **Earlier spec** (05-risks/01-dry-violations.md): "tables are never written by the active pipeline … drop them."
- **Code truth (2026-06-25)**: The pipeline DOES write to `entities` when dual-write is enabled:
  - `merger/entity.rs` L52-70 calls `self.relational_sink.upsert_entity(...)`.
  - `text_insert.rs` L909-936 calls `self.relational_sink.upsert_entity(...)` for every entity in the batch.
  - The sink is `PostgresEntitySink` when `entity_sync_mode ∈ {dual_write, full}` and `NoopEntitySink` otherwise (`postgres_entity_sink.rs::create_if_enabled`).
- **Re-classification**: This is no longer a DRY violation *by design* — it is an **intentional CQRS read model** (see 07-cqrs-dual-store-design.md). The DRY concern only remains for the *default deployment* where `entity_sync_mode = disabled` and the tables exist but stay empty. With migration 039 + 040 applied and mode = `full`, the duplication is governed and useful.
- **Action**: Keep the 07/08/09 CQRS plan. **Remove** the "drop `entities`/`relationships`" recommendation still present in 01-dry-violations.md and 01-executive-summary.md.

### R-DRY-02 — Duplicate embedding columns — **VERIFIED FIXED**

- Migration 039 STEP 1 drops `chunks.embedding` and `entities.embedding` plus their HNSW indexes (file `migrations/039_cqrs_entities_schema.sql` L65-83). The PRE-STEP drops the `edgequake.chunks`/`edgequake.entities` SELECT-* views first (L55-56) and STEP 7 recreates them with explicit column lists (L200-236). This resolves the RB-03 roadblock documented in `plan.md`.
- **Action**: Mark R-DRY-02 as **resolved** in README critical risk table.

### R-DRY-03 — Document stats computed at write time — **VERIFIED, partially addressed**

- `documents.chunk_count`, `documents.entity_count` are still updated post-pipeline (`text_insert.rs` via `update_document_status_with_stats`).
- The dashboard does NOT consult these columns (KV is read authority), so the denormalization is currently *write-only debt*, not a read-path bug.
- **Action**: Keep as MEDIUM risk; add an invariant in StorageInspector comparing `documents.chunk_count` vs `KV chunk key count per doc_id` (see §7 P4-05).

### R-DRY-04 — `documents` table partially mirrored in `eq_*_kv` — **VERIFIED, reclassified**

- Earlier spec framed this as " KV as shadow." Code shows the opposite: **KV is the read authority**, `documents` is the secondary best-effort write.
- This is the **exact inversion** that produces the "0 documents" UX bug in file 11.
- **Action**: Reclassify as **R-CONS-04 (cross-store consistency)**, not DRY. The fix is a read-path decision (see §7 P6-01).

### R-DRY-05 — KV key patterns — **VERIFIED FIXED**

- `edgequake-storage/src/kv_key_schema.rs` centralizes `doc_metadata`, `doc_chunk`, `doc_chunk_prefix`, `doc_content`, `doc_all_prefix`, `llm_cache`, `keyword_cache` + parsers.
- `text_insert.rs`, `deletion.rs`, `list.rs` all call `kv_keys::doc_metadata(...)` / `kv_keys::doc_chunk_prefix(...)` — confirmed at:
  - `text_insert.rs` L114, L270, L376
  - `deletion.rs` L83, L175, L176
- **Action**: Mark resolved.

---

## 3. SOLID Violations — Re-Assessed

### R-SOLID-01 — GraphStorage ISP — **VERIFIED, partial mitigation exists**

- `traits/graph.rs` L180-194 confirms `GraphStorage` is still a composite supertrait of `GraphStorageReadOps + GraphScanOps + GraphStorageMutateOps + GraphStorageAnalyticsOps`.
- A read-only alias `GraphReadView` is referenced in the trait doc (L173) — that is the ISP escape hatch.
- **Status**: Mitigation exists at the type level but `AppState.storage.graph_storage: Arc<dyn GraphStorage>` (`state/storage_runtime.rs`) still forces the full surface on handlers. The Phase 5 refactor in `10-battle-tested-improvement-plan.md` is still pending.
- **Action**: Keep as MEDIUM; lower from HIGH (the sub-traits already let query code use `GraphReadView`).

### R-SOLID-02 — AppState SRP — **VERIFIED**

- `state/mod.rs` L128-178 lists 16 fields spanning storage, query, auth, tasks, workspace, conversation, config, cache, rate limiter, pg_pool, audit, resource guard, graph materialize, migration bootstrap.
- **Action**: Accept as known debt; the recommendation in 02-solid-violations.md (domain accessors) is sound. Lower priority than the ingestion/query reliability gaps.

### R-SOLID-03 — KVStorage::ping() — **VERIFIED FIXED**

- `traits/kv.rs` L157-161: default `ping()` is now `Ok(())` (true O(1) no-op) with a contract doc requiring O(1) and an override recommendation. The previous O(N) `count()`-based default is gone.
- **Action**: Mark resolved.

### R-SOLID-04 — VectorId implicit contract — **VERIFIED FIXED**

- `edgequake-storage/src/vector_id.rs` exists. (The query path still uses `metadata.entity_name` decoding — see `query_pipeline.md` Step 3a — but the typed constructors are now the canonical writer side.)
- **Action**: Mark resolved at the writer level; the reader-side migration remains a follow-up (§7 P5-02).

---

## 4. Reliability at Ingestion — Re-Assessed (Code-Verified)

### 4.1 Two ingestion paths — both wired for dual-write — **VERIFIED**

The plan's "Two Ingestion Paths" diagram (`plan.md` L136-149) is accurate:

1. **Orchestrator path** (`EdgeQuake::insert`, `orchestrator/ingestion.rs`)
   - L353-363: `KnowledgeGraphMerger::new(...).with_tenant_context(...).with_relational_sink(self.relational_sink.clone())` — VERIFIED wiring.
   - Inside `merger/entity.rs::merge_entity` L52-70: best-effort `relational_sink.upsert_entity(...)` after the graph upsert succeeds.

2. **Direct processor path** (`DocumentTaskProcessor::process_text_insert`, `processor/text_insert.rs`)
   - L909-936: after `graph_storage.upsert_nodes_batch(...)` succeeds, iterates entities and calls `self.relational_sink.upsert_entity(...)` best-effort.

3. **main.rs wiring** (L586-593):
   ```rust
   #[cfg(feature = "postgres")]
   if let Some(ref pool) = state.pg_pool {
       let entity_sink = PostgresEntitySink::create_if_enabled(Arc::new(pool.clone())).await;
       processor = processor.with_relational_sink(entity_sink);
   }
   ```
   VERIFIED. The previous "P3-02/03/04 gaps" documented in plan.md are now closed in code.

### 4.2 Cross-store saga compensation — **VERIFIED, but vector-only**

`ingestion.rs::fail_with_chunk_vector_rollback` + `compensate_orphan_chunk_vectors` (L447-505):

- On graph-merge failure (`Err` or `errors > 0`), chunk vectors written in Stage 2 are deleted by exact chunk ID list.
- **Gap (NEW)**: The processor path (`text_insert.rs`) does **not** implement the same saga. It collects `storage_errors` and marks the document `partial_failure` or `failed`, but it does **not** roll back the chunk vectors already written at L682-690 nor the entity embeddings written at L966-981. This means a graph-batch failure on the processor path **orphans chunk + entity vectors**.
- **Action (NEW P3-06)**: Add an equivalent compensation in `process_text_insert`: if `upsert_nodes_batch` fails, delete the chunk IDs and `entity:{name}` IDs just written. Track them in a `Vec<String>` captured before the graph call.

### 4.3 CQRS dual-write is best-effort by design — **VERIFIED, with one caveat**

- `postgres_entity_sink.rs` L114-125: on SQL error the sink logs a warning and returns `Ok(())` — ingestion never fails. Good for availability.
- **Caveat (NEW)**: Because the sink swallows all errors (including "schema column missing"), a deployment that has not applied migration 039 will silently never sync. The `entity_sync_mode` flag will read `dual_write` but no rows will land. There is no health signal for this.
- **Action (NEW P4-06)**: Extend `StorageInspector` Layer 2 with invariant INV-04b: "if `entity_sync_mode ∈ {dual_write, full}`, then `entities.sync_status='synced'` count must be > 0 after the first successful ingestion post-enable." Surface as a startup WARNING when violated.

### 4.4 Deletion path — **VERIFIED, with cross-store gap**

`orchestrator/deletion.rs::delete_document`:

- L84-89: full `kv_storage.keys()` scan to find chunk IDs for the doc. This is the O(N_kv) scan called out in 10-battle-tested-improvement-plan.md Finding F4. The fix (GIN on `entities.source_chunk_ids`) only helps the *entities* table lookup, not this KV scan.
- L117-141: `relational_sink.remove_entity_sources(...)` is called for both fully-removed and partially-updated cases — VERIFIED dual-write delete.
- **Gap (NEW)**: The orchestrator `delete_document` does **not** call `pdf_storage.delete_document_record(&doc_uuid)`. Only the API handler `handlers/documents/delete/single.rs` L372 does. If a caller uses `EdgeQuake::delete_document` directly (e.g. programmatic re-ingestion via `delete_document_for_reingestion` in `storage_helpers.rs` L573), the `documents` table row is left behind while KV/graph/vectors are cleaned.
- **Gap (NEW)**: The deletion path also does not delete entity embeddings from the vector store when an entity is *partially* updated (only `vector_storage.delete_entity` on full removal, L115). A partial update leaves stale `entity:{name}` vector metadata pointing at the removed chunks.
- **Action (NEW P3-07)**: Centralize deletion in a `DocumentDeletionCoordinator` that calls (a) KV cleanup, (b) graph cleanup + relational sync, (c) vector cleanup (chunk + entity), (d) `documents` table cleanup — in that order, best-effort, with a quarantine log on each step's failure. Both the API handler and `delete_document_for_reingestion` should call it.

### 4.5 Status-state integrity — **VERIFIED, robust**

`text_insert.rs` L1043-1086 implements a rigorous final-status decision tree:

- `failed` if all chunks failed or 0 chunks produced
- `partial_failure` if 0 entities extracted OR `storage_errors` non-empty
- `completed` only when extraction + storage both clean

This is the FIX-1/FIX-2 logic and it is correctly wired through `update_document_status_with_stats`. **No change needed.**

### 4.6 Stats cache invalidation — **VERIFIED**

`text_insert.rs` L1200-1204 calls `invalidate_workspace_stats_cache(workspace_uuid)` after processing. This prevents the stale-cache "0 entities" symptom documented in the comment. Good.

---

## 5. Reliability at Query — Re-Assessed

### 5.1 Workspace-scoped vector routing — **VERIFIED, strict mode is the safety net**

- `text_insert.rs` L620-639: `get_workspace_vector_storage_strict` fails the task loudly if workspace storage cannot be obtained, preventing embeddings from landing in the global table. This is OODA-223 and is correctly enforced.
- Query side (`query_pipeline.md` §"Workspace Vector Routing") mirrors this with `WorkspaceVectorRegistry::get_or_create_workspace_storage`.
- **Concern (NEW)**: If a workspace is deleted between ingestion and query, its workspace-scoped vector table `eq_{ns}_ws_{uuid8}_vectors` is orphaned. No GC exists.
- **Action (NEW P4-07)**: Add invariant INV-06 to StorageInspector: "for every `eq_*_ws_*_vectors` table, the corresponding `workspace_id` exists in `workspaces`." Auto-repair = table drop (CAUTION tier, requires admin approval).

### 5.2 Chunk retrieval by source_id — **VERIFIED, fragile contract**

- Query engine reads `source_chunk_ids` from node/edge properties and calls `kv_storage.get_by_ids(chunk_ids)`.
- The chunk IDs are produced by the pipeline as `{doc_id}-chunk-{n}`. Any future change to the chunker that alters the ID scheme silently breaks retrieval.
- **Action (NEW P5-03)**: Add a contract test that ingests a tiny document through the real pipeline and asserts that the IDs in `entity.source_chunk_ids` are retrievable via `kv_storage.get_by_ids`. This is a single Rust integration test that catches the entire class of writer/reader ID drift.

### 5.3 Local/Global mode dependency on entity vectors — **VERIFIED**

- `query_pipeline.md` Step 3a (Local): vector search filtered by `type=entity`.
- `text_insert.rs` L965: entity vectors stored with `metadata.type = "entity"` and `metadata.entity_name`.
- **VERIFIED consistent** — but only because both sides agree on the `"entity"` literal. This is the same implicit-contract risk as R-SOLID-04 and reinforces the need for P5-03.

---

## 6. Gaps in the Existing Plan (plan.md)

| ID | Gap | Evidence | Severity |
| -- | --- | -------- | -------- |
| G-01 | plan.md says "PostgreSQL docs currently empty" in stats comments — code comment at `stats.rs` L49 still says "Fast but currently empty" but live DB has 542 rows (per file 11). | `stats.rs` L49 | LOW (cosmetic) |
| G-02 | No saga compensation on the processor path (`text_insert.rs`) — chunk/entity vectors orphaned on graph-batch failure. | `text_insert.rs` L898-1020 vs `ingestion.rs` L400-423 | HIGH |
| G-03 | Orchestrator `delete_document` does not delete `documents` table row nor entity vectors on partial update. | `deletion.rs` L99-144 | HIGH |
| G-04 | No health signal when `entity_sync_mode = dual_write` but sink silently no-ops (e.g. migration 039 missing). | `postgres_entity_sink.rs` L114-125 | MEDIUM |
| G-05 | No GC for orphaned workspace-scoped vector tables after workspace deletion. | (absent) | MEDIUM |
| G-06 | No contract test bridging writer/reader ID conventions for `source_chunk_ids`. | (absent) | MEDIUM |
| G-07 | 01-executive-summary.md and 01-dry-violations.md still recommend dropping `entities`/`relationships` — contradicts 07-cqrs-dual-store-design.md and current code. | `01-executive-summary.md` L17, `01-dry-violations.md` L30 | LOW (doc drift) |
| G-08 | R-DRY-04 misclassified as DRY; it is a consistency/read-authority problem (now R-CONS-04). | README L73 | LOW (taxonomy) |
| G-09 | P4-03 (hourly monitor) and P4-04 (admin endpoint) marked 🚫 Skipped in plan.md but the `StorageInspector` module doc and 09-drift-detection-autorepair.md still describe them as integrated. Doc/code drift. | `storage_inspector.rs` L17-19 vs `plan.md` L100-101 | LOW |

---

## 7. Improved, Code-Verified Action Plan

> This plan supersedes the action items in `10-battle-tested-improvement-plan.md`
> wherever they conflict. Already-completed items from `plan.md` (P0, P1, P2,
> P3-01..05, E2E-01..09) are not repeated; only **new or corrected** work is listed.

### Phase 3b — Ingestion Reliability Hardening (NEW — highest priority)

| ID | Task | Files | Risk | Estimate |
| -- | ---- | ----- | ---- | -------- |
| P3-06 | Add saga compensation to `process_text_insert`: on `upsert_nodes_batch` failure, delete the chunk IDs + `entity:{name}` IDs written in Stage 2/3. Reuse `compensate_orphan_chunk_vectors` pattern from `ingestion.rs`. | `edgequake-api/src/processor/text_insert.rs`, `edgequake-api/src/processor/compensation.rs` (new) | MEDIUM | 1 day |
| P3-07 | Introduce `DocumentDeletionCoordinator` that performs KV → graph → relational → vector → `documents` table cleanup in order, best-effort with quarantine logs. Refactor API handler `delete/single.rs` and `storage_helpers::delete_document_for_reingestion` to call it. | `edgequake-core/src/orchestrator/deletion.rs`, `edgequake-api/src/handlers/documents/delete/single.rs`, `edgequake-api/src/handlers/documents/storage_helpers.rs` | MEDIUM | 1.5 days |
| P3-08 | On partial entity update in deletion, also refresh the `entity:{name}` vector metadata (or delete + re-upsert) so it no longer references removed chunks. | `edgequake-core/src/orchestrator/deletion.rs` | LOW | 0.5 day |

### Phase 4b — Inspector & Health Hardening (NEW)

| ID | Task | Files | Risk | Estimate |
| -- | ---- | ----- | ---- | -------- |
| P4-05 | Add invariant: `documents.chunk_count` vs KV chunk-key count per `document_id` (R-DRY-03 monitor). | `edgequake-api/src/storage_inspector.rs` | LOW | 0.5 day |
| P4-06 | Add invariant INV-04b: warn at startup if `entity_sync_mode ∈ {dual_write, full}` AND `entities` table has 0 `synced` rows AND AGE has > 0 nodes (silent no-op detection). | `edgequake-api/src/storage_inspector.rs` | LOW | 0.5 day |
| P4-07 | Add invariant INV-06: orphaned workspace vector tables (`eq_*_ws_*_vectors` whose workspace_id no longer exists). CAUTION-tier repair (drop). | `edgequake-api/src/storage_inspector.rs` | LOW (read-only detection) | 0.5 day |
| P4-08 | Re-enable P4-03 (hourly monitor) by spawning the existing `StorageInspector::inspect` loop in `task_runtime.rs`. The inspector module already exists; only the spawn is missing. | `edgequake-api/src/state/task_runtime.rs` | LOW | 0.5 day |
| P4-09 | Re-enable P4-04 admin endpoint `/api/v1/admin/storage/inspect` (read-only) + `/repair?dry_run=true`. Handler is straightforward against the existing inspector. | `edgequake-api/src/handlers/admin/storage.rs` (new), route registration in `edgequake-api/src/lib.rs` | LOW | 0.5 day |

### Phase 5b — Read Authority & Contract Tests (NEW)

| ID | Task | Files | Risk | Estimate |
| -- | ---- | ----- | ---- | -------- |
| P5-01 | Decide dashboard `document_count` read authority (Option A in file 11: relational `documents` table primary, KV fallback). Update `try_kv_storage_stats` to consult `workspace_service.get_workspace_stats` first when Postgres feature active. This closes the "0 documents" UX bug. | `edgequake-api/src/handlers/workspaces/stats.rs` | MEDIUM (behavior change) | 0.5 day + test |
| P5-02 | Migrate query-path entity vector decoding to use `VectorId::from_storage_id` instead of raw `metadata.entity_name` string lookup (R-SOLID-04 reader side). | `edgequake-query/src/strategies/local.rs`, `edgequake-query/src/strategies/global.rs` | LOW | 1 day |
| P5-03 | Add contract test: ingest a tiny doc through the real pipeline, assert `entity.source_chunk_ids` are retrievable via `kv_storage.get_by_ids`. Catches writer/reader ID drift. | `edgequake/crates/edgequake-core/tests/` (new) | LOW | 0.5 day |
| P5-04 | Add contract test: ingest → delete → assert `documents` row, KV metadata, chunk vectors, entity vectors, graph nodes all gone. Uses the new `DocumentDeletionCoordinator` from P3-07. | `edgequake/crates/edgequake-core/tests/` (new) | LOW | 0.5 day |

### Phase 6 — Documentation Reconciliation (NEW — low effort, high clarity value)

| ID | Task | Files |
| -- | ---- | ----- |
| P6-01 | Update README "Sources of Truth" table: document metadata read authority = KV (current truth), with `documents` table as secondary best-effort write. Reclassify R-DRY-04 → R-CONS-04. | `specs/021-storage-study/README.md` |
| P6-02 | Update `01-executive-summary.md`: remove "entities/relationships tables are effectively orphaned" and "drop them" — replace with "CQRS read model, populated when `entity_sync_mode != disabled`." | `specs/021-storage-study/01-overview/01-executive-summary.md` |
| P6-03 | Update `05-risks/01-dry-violations.md` R-DRY-01: mark RESOLVED-BY-DESIGN (CQRS) with link to 07. Update R-DRY-02, R-DRY-05, R-SOLID-03, R-SOLID-04 as RESOLVED with migration/code refs. | `specs/021-storage-study/05-risks/01-dry-violations.md`, `02-solid-violations.md` |
| P6-04 | Update `stats.rs` L49 comment: remove "currently empty" — replace with "consulted when `entity_sync_mode = full`; until then KV is read authority (see SPEC-021 §1)." | `edgequake/crates/edgequake-api/src/handlers/workspaces/stats.rs` |
| P6-05 | Reconcile `storage_inspector.rs` module doc (L17-19) with plan.md: either implement P4-04 admin endpoint (P4-09 here) or remove the doc lines claiming it exists. | `edgequake/crates/edgequake-api/src/storage_inspector.rs` |

---

## 8. Prioritized Execution Order

1. **P3-06** (processor saga compensation) — closes the highest-severity NEW ingestion gap.
2. **P3-07** (deletion coordinator) — closes the highest-severity NEW query/cleanup gap.
3. **P5-01** (dashboard read authority) — fixes the user-visible "0 documents" bug from file 11.
4. **P4-08 + P4-09** (hourly monitor + admin endpoint) — operationalizes the inspector that already exists.
5. **P4-06** (silent no-op detection) — prevents the next silent-divergence incident.
6. **P5-03 + P5-04** (contract tests) — lock in the fixes above.
7. **P3-08, P4-05, P4-07, P5-02** — secondary hardening.
8. **P6-01..05** — documentation reconciliation (do alongside each code change, not at the end).

---

## 9. Summary of Corrections to Earlier Spec Files

| Earlier claim | Location | Correction |
| ------------- | -------- | ---------- |
| "entities/relationships tables are effectively orphaned" / "drop them" | 01-executive-summary.md L17-18; 01-dry-violations.md L30 | They are a CQRS read model populated when `entity_sync_mode != disabled`. Do not drop. |
| "Document metadata primary: `documents` table; KV as shadow" | README.md L51 | Inverted: KV is the read authority; `documents` table is a best-effort secondary write. |
| R-DRY-04 framed as a DRY violation | README.md L73, 01-dry-violations.md L94 | Reclassify as R-CONS-04 (cross-store consistency / read-authority ambiguity). |
| "PostgreSQL docs currently empty" | stats.rs L49 comment | Stale; live DB has 542 rows (per file 11). Update comment. |
| P3-02/03/04 listed as gaps in plan.md | plan.md L76-91 | Verified CLOSED in code (ingestion.rs L363, text_insert.rs L909-936, main.rs L586-593). |
| P4-03/P4-04 claimed implemented in storage_inspector.rs module doc | storage_inspector.rs L17-19 | Not wired (no spawn in task_runtime, no route in lib.rs). Plan.md correctly marks them Skipped; the module doc is the drift. |
| "No 2PC between vector store and AGE graph — partial write window" (R-CONS-01) | README.md L76 | Still true on the orchestrator path's graph side, but the saga in `ingestion.rs` collapses the orphan window to vectors only. The **processor path** lacks the equivalent saga — see P3-06. |

---

## Task logs

Actions: Re-read all key production files cited by spec 021 (stats.rs, list.rs, ingestion.rs, merger/entity.rs, text_insert.rs, deletion.rs, postgres_entity_sink.rs, storage_inspector.rs, state/mod.rs, state/postgres.rs, kv.rs, graph.rs, migrations 039/040 + apply.sql); cross-checked every spec claim against source.

Decisions: Reclassify R-DRY-01 as resolved-by-design (CQRS); reclassify R-DRY-04 as R-CONS-04; mark R-DRY-02, R-DRY-05, R-SOLID-03, R-SOLID-04 resolved in code; identify three new HIGH/MEDIUM ingestion+query gaps (P3-06, P3-07, P4-06).

Next steps: Implement P3-06 (processor saga compensation) and P3-07 (deletion coordinator) first; then P5-01 to close the UX 0-documents bug; then P4-08/09 to operationalize the inspector. Documentation reconciliation (P6-*) should be done side-by-side with each code change.

Lessons/insights: The spec's biggest residual risk is no longer schema drift (migrations 039/040 handle it) but **path asymmetry**: the orchestrator path has a saga, the processor path does not; the API delete handler cleans the `documents` table, the orchestrator delete does not. Closing those asymmetries is higher value than any remaining schema work.
