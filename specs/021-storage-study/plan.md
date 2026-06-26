# Implementation Plan — specs/021-storage-study

> **Updated**: 2026-06-25 (honest audit against actual code)
> **AUTHORITATIVE PLAN**: `06-first-principles/17-battle-tested-improvement-plan-consolidated.md`
> supersedes this file's action items wherever they conflict. This file remains
> the **status tracker** for completed work (P0–P5). New work (Phase A–F from
> file 17) is tracked below in §"Phase A–F (from file 17)".
> **Status Legend**: ✅ Done | 🔄 In Progress | ⬜ Not Started | ❌ Blocked | 🧪 Tested | ⚠️ Needs Test | 🚫 Skipped

---

## Audit Methodology

Each item verified by:
1. Checking the source file exists and contains the described code
2. Checking the code is wired into the production execution path (not just defined)
3. Running `cargo build --features postgres` to confirm compilation
4. Verifying the migration applied cleanly against the live database

---

## Phase 0 — Zero-Code Fixes

| ID    | Task                                                                           | Status | Files                        | Notes                                                       |
| ----- | ------------------------------------------------------------------------------ | ------ | ---------------------------- | ----------------------------------------------------------- |
| P0-01 | Document authoritative stores — NOTES.md | ✅ Done | `migrations/NOTES.md`        | Migrations are SHA-384 checksum-locked; docs go in NOTES.md |
| P0-02 | AGE expression index verification script  | ✅ Done | `e2e/verify_age_indexes.sql` | EXPLAIN ANALYZE query for manual review                     |

---

## Phase 1 — Schema Foundation

| ID     | Task                                                  | Status     | Files                                         | Notes                                |
| ------ | ----------------------------------------------------- | ---------- | --------------------------------------------- | ------------------------------------ |
| P1-01  | `KVKeySchema` module — centralize all KV key patterns | ✅ Done 🧪 | `edgequake-storage/src/kv_key_schema.rs`      | Replaces ~30 scattered format! calls |
| P1-01b | Replace all format! callers with `kv_keys::*`         | ✅ Done    | processor/*, deletion.rs, etc.                | All production code updated          |
| P1-02  | Fix `KVStorage::ping()` default — O(1) no-op          | ✅ Done 🧪 | `edgequake-storage/src/traits/kv.rs`          | Was O(N) COUNT(*)                    |
| P1-02b | Override `ping()` in `PostgresKVStorage`              | ✅ Done    | `adapters/postgres/kv.rs`                     | SELECT 1 — O(1) probe                |
| P1-03  | Add `ping()` to `GraphStorage` trait                  | ✅ Done    | `traits/graph.rs`                             | Default: lightweight cypher MATCH    |
| P1-03b | Override `ping()` in `PostgresAGEGraphStorage`        | ✅ Done    | `adapters/postgres/graph/graph_storage_impl.rs`| Direct SQL on AGE vertex table       |
| P1-03c | Override `ping()` in `MemoryGraphStorage`             | ✅ Done    | `adapters/memory/graph.rs`                    | Returns Ok(()) immediately           |
| P1-04  | `VectorId` typed module — typed IDs for vectors       | ✅ Done 🧪 | `edgequake-storage/src/vector_id.rs`          | Prevents silent key mismatch         |

---

## Phase 2 — Schema Repair (Migrations)

| ID     | Task                                             | Status          | Files                                                   | Notes                                                   |
| ------ | ------------------------------------------------ | --------------- | ------------------------------------------------------- | ------------------------------------------------------- |
| P2-01  | Migration 039 — correct entities schema for CQRS | ✅ Done **FIXED** | `migrations/039_cqrs_entities_schema.sql`               | Fixed 2026-06-25: view dependency blocked DROP COLUMN   |
| P2-02  | Migration 040 — backfill marker                  | ✅ Done         | `migrations/040_entity_backfill_marker.sql`             | Marker only; actual backfill in apply.sql               |
| P2-02b | Backfill apply script                            | ✅ Done         | `migrations/support/040/apply.sql`                      | Paginated AGE→relational backfill                       |
| P2-02c | Integration in migration_bootstrap.rs            | ✅ Done         | `crates/edgequake-api/src/state/migration_bootstrap.rs` | Spawns background tokio task after migration 040        |

### Root Cause of Migration 039 Failure (Fixed)

Migration 001 creates `edgequake.*` schema views using `SELECT *` from `public.*` tables.
PostgreSQL bakes column references at view-creation time. When migration 039 tried to
`ALTER TABLE chunks DROP COLUMN IF EXISTS embedding`, PostgreSQL refused because
`edgequake.chunks` still referenced `embedding`.

**Fix**: Added PRE-STEP to drop the two views before column drops; STEP 7 recreates
them with explicit column lists (without `embedding`, with new CQRS columns).

---

## Phase 3 — Dual-Write Integration

| ID     | Task                                              | Status              | Files                                                    | Notes                                                                              |
| ------ | ------------------------------------------------- | ------------------- | -------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| P3-01  | `RelationalEntitySink` trait + `NoopEntitySink`   | ✅ Done 🧪          | `crates/edgequake-pipeline/src/merger/mod.rs`            | DIP trait; NoopEntitySink is backward-compatible default                           |
| P3-01b | Dual-write call in `KnowledgeGraphMerger`         | ✅ Done 🧪          | `crates/edgequake-pipeline/src/merger/entity.rs`         | `merge_entity()` calls `relational_sink.upsert_entity()` (best-effort)            |
| P3-01c | `PostgresEntitySink` concrete implementation      | ✅ Done             | `crates/edgequake-api/src/postgres_entity_sink.rs`       | `create_if_enabled()` checks entity_sync_mode at construction                     |
| P3-01d | Export `RelationalEntitySink`, `NoopEntitySink`   | ✅ Done             | `crates/edgequake-pipeline/src/lib.rs`                   | Public API for consumers                                                           |
| P3-02  | Wire `relational_sink` in `EdgeQuake` orchestrator| ✅ Done **FIXED**   | `crates/edgequake-core/src/orchestrator/ingestion.rs`    | Added `.with_relational_sink(self.relational_sink.clone())` — was missing          |
| P3-03  | Wire `relational_sink` in `DocumentTaskProcessor` | ✅ Done **NEW**     | `crates/edgequake-api/src/processor/mod.rs`+`text_insert.rs` | Added field + builder; dual-write after `upsert_nodes_batch()` (best-effort)  |
| P3-04  | Wire `PostgresEntitySink` from `main.rs`          | ✅ Done **NEW**     | `edgequake/src/main.rs`                                  | Calls `create_if_enabled(pool)` at startup; chains via `with_relational_sink()`   |
| P3-05  | Relational sync in `EdgeQuake::delete_document`   | ✅ Done             | `crates/edgequake-core/src/orchestrator/deletion.rs`     | Calls `remove_entity_sources` after graph node delete (best-effort)               |

### Gaps Found in Audit (all Fixed)

The previous plan claimed P3 was complete but three production paths were NOT wired:

1. **`ingestion.rs` gap**: `KnowledgeGraphMerger` created without `.with_relational_sink()` —
   `EdgeQuake.relational_sink` field existed but was never passed to the merger.

2. **`text_insert.rs` gap**: `DocumentTaskProcessor` had no `relational_sink` field; direct graph
   writes via `upsert_nodes_batch()` never triggered CQRS sync.

3. **`main.rs` gap**: `PostgresEntitySink::create_if_enabled()` was defined but never called —
   processor always used `NoopEntitySink`.

Since `entity_sync_mode = "disabled"` by default, there is **zero regression risk** — dual-write
only activates when explicitly enabled via server_config.

---

## Phase 4 — Storage Inspector

| ID     | Task                                                  | Status      | Files                                              | Notes                                                                     |
| ------ | ----------------------------------------------------- | ----------- | -------------------------------------------------- | ------------------------------------------------------------------------- |
| P4-01  | `StorageInspector` Rust module                        | ✅ Done     | `crates/edgequake-api/src/storage_inspector.rs`    | 3-layer: schema drift + data invariants + auto-repair                     |
| P4-02  | Startup invariant check in `AppState::new_postgres`   | ✅ Done     | `crates/edgequake-api/src/state/postgres.rs`       | Lines 382–415; logs critical/warning; auto-repairs SAFE-tier              |
| P4-03  | Background hourly monitor task                        | 🚫 Skipped  | (not implemented)                                  | `task_runtime.rs` has no storage monitor. Startup check is sufficient.    |
| P4-04  | `/api/v1/admin/storage/inspect` endpoint              | 🚫 Skipped  | (not implemented)                                  | No route registered, no handler file. Future sprint item.                 |

> **Honesty note**: A previous version of this plan had a **duplicate Phase 4 table** that
> falsely claimed P4-03 and P4-04 were "Done". This was inaccurate. They are deliberately
> skipped — the startup check (P4-02) provides sufficient observability for now.

---

## E2E Tests

| ID     | Test                                     | Status     | File                                                      |
| ------ | ---------------------------------------- | ---------- | --------------------------------------------------------- |
| E2E-01 | KVKeySchema contract tests (Rust)        | ✅ Done 🧪 | `edgequake-storage/src/kv_key_schema.rs` (13 inline)      |
| E2E-02 | VectorId roundtrip tests (Rust)          | ✅ Done 🧪 | `edgequake-storage/src/vector_id.rs` (9 inline)           |
| E2E-03 | NoopEntitySink contract (Rust)           | ✅ Done 🧪 | `merger/mod.rs` (2 inline tests)                          |
| E2E-04 | SpySink — merge_entity calls sink (Rust) | ✅ Done 🧪 | `merger/mod.rs` (tokio async test)                        |
| E2E-05 | KV key pattern SQL verification          | ✅ Done    | `specs/021-storage-study/e2e/test_kv_key_schema.sql`      |
| E2E-06 | Migration 039 idempotency SQL test       | ✅ Done    | `specs/021-storage-study/e2e/test_migration_039.sql`      |
| E2E-07 | Storage invariant SQL checks             | ✅ Done    | `specs/021-storage-study/e2e/test_invariants.sql`         |
| E2E-08 | AGE index verification                   | ✅ Done    | `specs/021-storage-study/e2e/verify_age_indexes.sql`      |
| E2E-09 | Playwright: Document upload + query (Mistral) | ✅ Done | `specs/021-storage-study/e2e/` + `screenshots/`          |
| E2E-10 | Rust: hybrid read model — relational docs when KV missing (P5-01) | ✅ Done 🧪 | `edgequake-api/tests/e2e_zero_documents_spec021.rs` |
| E2E-11 | Playwright: zero-documents dashboard fix (P5-01) | ✅ Done | `edgequake_webui/e2e/spec021-zero-documents-fix.spec.ts` → `screenshots/11-*.png`, `12-*.png`, `13-*.png` |
| E2E-12 | Vitest: backend-not-ready stabilization (P5-02) | ✅ Done 🧪 | `edgequake_webui/src/lib/api/__tests__/backend-readiness.test.ts`, `query-params.test.ts`, `api-error-boundary.test.tsx` |
| E2E-13 | Vitest: silent network logging + upload-limit SSOT (P5-02) | ✅ Done 🧪 | `edgequake_webui/src/lib/api/__tests__/observability-client.test.ts` (updated) |
| E2E-14 | Rust: graph materialization capacity (P5-03) | ✅ Done 🧪 | `edgequake-api/src/handlers/graph_types.rs`, `graph_stream.rs`, `error.rs`; `edgequake-core/src/resource/budget.rs` |
| E2E-15 | Vitest: graph stream transient-congestion retry (P5-03) | ✅ Done 🧪 | `edgequake_webui/src/lib/api/__tests__/graph-stream-retry.test.ts`, `app-version.test.ts` |

---

## Phase 5 — Read Model / UX Fixes

| ID     | Task                                              | Status          | Files                                                    | Notes                                                                              |
| ------ | ------------------------------------------------- | --------------- | -------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| P5-01  | Hybrid document read model (`max(pg, kv)`)        | ✅ Done 🧪       | `document_read_model.rs`, `stats.rs`, `list.rs`          | Fixes dashboard "0 documents" when relational rows exist; KV fallback preserved    |
| P5-01b | Fix `pg_get_workspace_stats` embedding_count SQL   | ✅ Done         | `workspace_ops.rs`                                       | Removed dropped `chunks.embedding` column reference (migration 039)                  |
| P5-02  | Frontend stabilization + capacity/API DRY fixes    | ✅ Done 🧪       | `client-context.ts`, `stream-client.ts`, `backend-readiness.ts`, `query-params.ts`, `upload-limits.ts`, `api-error-boundary.tsx`, `backend-status-banner.tsx`, `app/error.tsx`, `app/global-error.tsx` | Backend-not-ready edge cases; `console.warn` (no dev overlay); NetworkError retry; error boundaries; upload-limit SSOT; `buildQueryString` DRY. See `06-first-principles/13-*.md`, `14-*.md`. |
| P5-03  | Graph materialization capacity + version-label fix | ✅ Done 🧪       | `budget.rs` (default 1→4), `graph_types.rs` (structured SSE error), `error.rs` (`TransientCongestion` SSOT), `graph_stream.rs` (structured error), `graph-stream-retry.ts` (retry helper), `use-graph-stream.ts` (backoff retry), `app-version.ts`/`sidebar.tsx`/`header.tsx` (labeled UI vs API version) | Fixes "Graph materialization capacity reached" toast: raises default concurrency, makes SSE error carry reason+retry_after_secs (was lossy vs HTTP 503), adds client exponential-backoff retry with jitter, labels sidebar "UI vX" vs header "API vX" to resolve version discrepancy. See `06-first-principles/15-*.md`. |

---

## Roadblocks

| ID    | Blocker                                                                                                  | Impact   | Resolution                                                                              |
| ----- | -------------------------------------------------------------------------------------------------------- | -------- | --------------------------------------------------------------------------------------- |
| RB-01 | Migration 039: bare RAISE NOTICE outside PL/pgSQL (previous)                                            | FIXED    | Wrapped in DO block; GENERATED ALWAYS guarded with DO                                   |
| RB-02 | AGE expression index compatibility cannot be auto-verified without live DB                               | P0-02    | Run `e2e/verify_age_indexes.sql` against production DB                                  |
| RB-03 | Migration 039: `edgequake.chunks`/`edgequake.entities` SELECT* views blocked `DROP COLUMN embedding`    | FIXED    | PRE-STEP: `DROP VIEW IF EXISTS edgequake.chunks/entities;` + STEP 7 recreates them      |
| RB-04 | P3 dual-write silent no-op: `relational_sink` field existed but never passed to merger or processor     | FIXED    | Added `.with_relational_sink()` in `ingestion.rs`; wired from `main.rs` via `AppState`  |

---

## Architecture Clarification: Two Ingestion Paths

```
Path 1: Orchestrator path (EdgeQuake::insert_document)
  ingestion.rs → KnowledgeGraphMerger::merge_entity()  ← relational_sink (P3-02)
              → graph_storage.upsert_node()

Path 2: Direct processor path (DocumentTaskProcessor::process_text_insert)
  text_insert.rs → graph_storage.upsert_nodes_batch()  ← relational_sink (P3-03)
                 → entity embeddings to vector_storage
```

Both paths call `relational_sink`. Default is `NoopEntitySink` — zero behavior change
until `entity_sync_mode` is set to `dual_write` or `full` in `server_config`.

---

## Summary Counts

| Phase | Planned | Implemented | Skipped |
| ----- | ------- | ----------- | ------- |
| P0    | 2       | 2           | 0       |
| P1    | 7       | 7           | 0       |
| P2    | 4       | 4           | 0       |
| P3    | 5       | 8           | 0       |
| P4    | 4       | 2           | 2       |
| E2E   | 8       | 11          | 0       |

---

## Phase A–F (from file 17 — AUTHORITATIVE)

> These items supersede the residual gaps in P3/P4 above. See
> `06-first-principles/17-battle-tested-improvement-plan-consolidated.md` for
> full design, edge-case registry, and acceptance tests.

### Phase A — Read Authority & Write-Path Closure (fixes the screenshot)

| ID   | Task                                                                          | Status | Files                                                                | Edge cases |
| ---- | ----------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------- | ---------- |
| P-A1 | Refresh relational `documents.chunk_count`/`entity_count` on every status update (close the dead-column write gap) | ⬜      | `pdf_storage.rs`, `pdf_storage_impl.rs`, `status_updates.rs` + migration | E1–E5 |
| P-A2 | Make `update_document_stats` idempotent + race-safe (upsert)                  | ⬜      | same as P-A1                                                         | E6–E7 |
| P-A3 | Per-row entity_count read authority: AGE fallback (`max(kv, pg, age)`)        | ⬜      | `graph_read_view.rs`, `analytics_ops.rs`, `document_read_model.rs`, `list.rs` | E8–E11 |
| P-A4 | Legacy workspace-scope backfill (admin-gated, idempotent)                     | ⬜      | new `storage_reconcile.rs`, `migration_bootstrap.rs`                | E12–E14 |

### Phase B — Invariants & UI Integrity

| ID   | Task                                                                          | Status | Files                                                                | Edge cases |
| ---- | ----------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------- | ---------- |
| P-B1 | Rewrite R-DRY-03 invariant to compare KV/PG vs AGE (not vs the dead column)   | ⬜      | `storage_inspector.rs`                                               | E15–E17 |
| P-B2 | `StatusCounts` must not treat NULL status as completed                       | ⬜      | `handlers/documents/query/list.rs`                                   | E18 |
| P-B3 | INV-D orphan vector + orphan workspace-table detection                       | ⬜      | `storage_inspector.rs`                                               | E19–E20 |

### Phase C — Deletion & Saga Symmetry

| ID   | Task                                                                          | Status | Files                                                                | Edge cases |
| ---- | ----------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------- | ---------- |
| P-C1 | Processor-path saga compensation (extract shared `compensation` module)      | ⬜      | new `processor/compensation.rs`, `text_insert.rs`                    | E21–E24 |
| P-C2 | `DocumentDeletionCoordinator` (single deletion entry point)                  | ⬜      | new `deletion_coordinator.rs`, `deletion.rs`, `delete/single.rs`, `storage_helpers.rs` | E25–E28 |
| P-C3 | Entity vector metadata refresh on partial deletion                           | ⬜      | `deletion_coordinator.rs`                                            | E29–E30 |

### Phase D — Operability (Inspector + Admin)

| ID   | Task                                                                          | Status | Files                                                                | Edge cases |
| ---- | ----------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------- | ---------- |
| P-D1 | Re-enable hourly invariant monitor in `task_runtime`                          | ⬜      | `state/task_runtime.rs`                                              | E31–E32 |
| P-D2 | Admin `/storage/inspect` + `/storage/repair?dry_run=true` endpoints           | ⬜      | new `handlers/admin/storage.rs`, `lib.rs`                            | E33–E34 |
| P-D3 | Silent no-op detection for `entity_sync_mode` (INV-04b)                       | ⬜      | `storage_inspector.rs`, `state/postgres.rs`                          | E35 |

### Phase E — Query Robustness & Contract Tests

| ID   | Task                                                                          | Status | Files                                                                | Edge cases |
| ---- | ----------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------- | ---------- |
| P-E1 | Migrate query-path entity vector decoding to `VectorId`                       | ⬜      | `edgequake-query/src/strategies/local.rs`, `global.rs`               | — |
| P-E2 | Writer/reader chunk-ID contract test                                          | ⬜      | new `edgequake-core/tests/contract_chunk_id.rs`                      | E36 |
| P-E3 | Ingest → delete → all-stores-empty contract test                              | ⬜      | new `edgequake-core/tests/contract_deletion.rs`                      | E37 |
| P-E4 | Per-row entity_count regression test (the screenshot)                         | ⬜      | new `edgequake-api/tests/regression_zero_entities.rs`                | E38 |

### Phase F — Documentation Reconciliation

| ID   | Task                                                                          | Status | Files |
| ---- | ----------------------------------------------------------------------------- | ------ | ----- |
| P-F1 | Update README source-of-truth table: per-row entity_count SSOT = AGE          | ⬜      | `README.md` |
| P-F2 | Re-elevate R-DRY-03 to CRITICAL with link to file 16                          | ✅ Done | `README.md` (this update) |
| P-F3 | Reference file 17 as the authoritative plan                                   | ✅ Done | `plan.md` (this update) |
| P-F4 | Update code comments (stats.rs, document_read_model.rs) with resolution order | ⬜      | code |
| P-F5 | Document `documents.*` column writers in migration NOTES.md                   | ⬜      | `migrations/NOTES.md` |

### Execution order (from file 17 §9)

1. P-A1 + P-A2 (write-path closure) — 1.25 days
2. P-A3 (AGE fallback, fixes screenshot) — 1 day
3. P-B1 + P-B2 (invariant + status integrity) — 0.75 day
4. P-C1 (processor saga) — 1 day
5. P-C2 + P-C3 (deletion coordinator) — 2 days
6. P-D1 + P-D2 + P-D3 (operability) — 1.5 days
7. P-A4 (legacy backfill) — 1.5 days
8. P-E1..P-E4 (contract tests) — 2.5 days
9. P-F1..P-F5 (docs) — alongside each change

**Total**: ~12 engineering days, user-visible value in days 1–3.
