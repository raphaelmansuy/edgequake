# 021 — EdgeQuake Storage Study

> **Status**: DRAFT 2026-06-25  
> **Scope**: Full cross-reference analysis of every storage layer, its data model,
> the ingestion/embedding/extraction pipeline, the query pipeline, and all risks
> relative to DRY and SOLID principles.  
> **Code is law** — all findings derive directly from source code, not documentation.

---

## Structure

```
021-storage-study/
  README.md                     <- this file
  01-overview/
    01-executive-summary.md     <- TL;DR, key risks, quick action table
    02-storage-landscape.md     <- High-level landscape of all storage systems
  02-schema/
    01-postgresql-tables.md     <- Relational tables (migrations 001-038)
    02-kv-store-model.md        <- Dynamic eq_*_kv tables & key taxonomy
    03-vector-store-model.md    <- eq_*_vectors + per-workspace tables
    04-graph-store-model.md     <- Apache AGE cypher graph (Node/EDGE)
    05-dynamic-tables.md        <- Table naming conventions & lifecycle
  03-pipelines/
    01-ingestion-pipeline.md    <- Ingest → chunk → extract → embed → store
    02-query-pipeline.md        <- Query modes, storage reads, LLM gen
    03-data-flow-diagrams.md    <- ASCII end-to-end flow diagrams
  04-api-storage-usage/
    01-api-endpoints-storage-map.md <- Which endpoint touches which storage
    02-storage-runtime.md       <- AppState / StorageRuntime wiring
  05-risks/
    01-dry-violations.md        <- Concrete DRY violations with code refs
    02-solid-violations.md      <- SOLID principle violations
    03-data-consistency-risks.md <- Saga / cross-store consistency gaps
  06-first-principles/
    01-analysis.md              <- First-principles analysis (original)
    02-improvement-plan.md      <- Initial improvement plan (superseded by 10 and 12)
    07-cqrs-dual-store-design.md  <- REVISED: dual-store advantage, CQRS design
    08-sync-ascending-compat.md   <- Sync strategy + ascending compatibility
    09-drift-detection-autorepair.md <- Schema drift detection + auto-repair
    10-battle-tested-improvement-plan.md <- Battle-tested full plan (superseded by 17)
    11-ux-zero-documents-root-cause-assessment.md <- UX 0-docs root cause
    12-code-verified-reassessment.md <- Code-verified reassessment + improved plan
    13-capacity-system-first-principles-assessment.md <- Capacity system assessment
    14-api-implementation-dry-solid-assessment.md <- Frontend API layer DRY/SOLID
    15-graph-materialization-capacity-assessment.md <- Graph materialization capacity
    16-completed-zero-entities-root-cause.md <- "Completed / 0 entities" screenshot root cause
    17-battle-tested-improvement-plan-consolidated.md <- AUTHORITATIVE (storage-consistency): consolidated, edge-case-aware plan
    18-ingestion-query-deep-audit.md <- AUTHORITATIVE (ingestion/query algorithmic layer): DRY/SOLID/first-principles/perf/O(N) audit, RC-6..RC-17
    19-ingestion-query-improvement-plan.md <- AUTHORITATIVE: Phases G1-G12 closing RC-6..RC-17
```

---

## Quick-Reference: Sources of Truth (Code-Verified 2026-06-25)

> See `06-first-principles/12-code-verified-reassessment.md` for the line-by-line
> code audit that produced this table. Read authority = the store the running
> production code actually consults; Write sources = where the pipeline writes.

| Data Domain               | Write Source (code-proven)                       | Read Authority (production path)            | Status |
| ------------------------- | ------------------------------------------------ | ------------------------------------------- | ------ |
| Document metadata         | `eq_*_kv` `{doc_id}-metadata` (always) **+** `documents` table (best-effort, `#[cfg(feature="postgres")]`) | `eq_*_kv` (`stats.rs::try_kv_storage_stats`, `documents/query/list.rs`) | R-CONS-04 (read authority ≠ write primary) |
| Chunk content             | `eq_*_kv` (key: `{id}-chunk-{n}`)                | `eq_*_kv`                                   | OK |
| Entity (traversal)        | AGE graph `Node`                                 | AGE graph                                   | OK |
| Relationships (traversal) | AGE graph `EDGE`                                 | AGE graph                                   | OK |
| Entity (analytics)        | `entities` table **only when `entity_sync_mode != disabled`** (CQRS dual-write via `PostgresEntitySink`) | Not yet wired into stats handler            | CQRS read model (07/08/09) |
| Relationships (analytics) | `relationships` table (dual-write, planned)      | Not yet wired                               | CQRS read model (planned) |
| Chunk embeddings          | `eq_*_vectors` (workspace-scoped)                | Vector store                                | OK |
| Entity embeddings         | `eq_*_vectors` key `entity:{name}`               | Vector store (Local mode)                   | OK |
| PDF raw bytes             | `pdf_documents.pdf_data`                         | `pdf_storage`                               | OK |
| Task queue                | `edgequake_tasks`                                | In-memory channel                           | OK |
| Conversations             | `conversations` / `messages`                     | `MemoryConversationStorage`                 | OK |
| Workspace registry        | `workspaces` table                               | In-memory cache                             | OK |

> **Correction note**: an earlier version of this table listed `documents` as the
> primary write source and KV as the shadow. The code shows the inverse: KV is the
> read authority, `documents` is a best-effort secondary write. This inversion is
> the root cause of the "0 documents" UX symptom documented in file 11.

---

## Critical Risk Summary (Code-Verified 2026-06-25)

> Status legend: ✅ Resolved in code | ⚠️ Mitigated/partial | ⛔ Open

| Risk ID    | Severity | Status | Description                                                                        |
| ---------- | -------- | ------ | ---------------------------------------------------------------------------------- |
| R-DRY-01   | HIGH     | ✅ Resolved-by-design | `entities`/`relationships` are a CQRS read model (07), populated by dual-write when `entity_sync_mode != disabled` — not orphaned, do not drop |
| R-DRY-02   | HIGH     | ✅ Resolved | Migration 039 drops `chunks.embedding`/`entities.embedding` + HNSW indexes         |
| R-DRY-03   | ~~MEDIUM~~ → **CRITICAL** | ⛔ Re-elevated | `documents.entity_count`/`chunk_count` are write-only-dead columns that P5-01 promoted to a read-path input → "Completed / 0 entities" screenshot (file 16). Fix: plan-17 P-A1 |
| R-DRY-04   | ~~MEDIUM~~ | ⛔ Reclassified → **R-CONS-04** | Document metadata read authority (KV) ≠ primary write (`documents` table) — root cause of "0 documents" UX bug (file 11) |
| R-DRY-05   | —        | ✅ Resolved | `KVKeySchema` module centralizes all KV key patterns (`kv_key_schema.rs`)         |
| R-SOLID-01 | HIGH→MED | ⚠️ Mitigated | `GraphStorage` ISP composite trait; sub-traits + `GraphReadView` exist, `AppState` still uses full trait |
| R-SOLID-02 | MEDIUM   | ⛔ Open  | `AppState` SRP — 16 fields, god object (acceptable debt)                          |
| R-SOLID-03 | MEDIUM   | ✅ Resolved | `KVStorage::ping()` default is O(1) `Ok(())` (kv.rs L157-161)                     |
| R-SOLID-04 | MEDIUM   | ⚠️ Mitigated | `VectorId` typed module exists on writer side; reader migration pending (P5-02)    |
| R-CONS-01  | HIGH     | ⚠️ Partial | No 2PC vector↔graph; orchestrator path has saga (`ingestion.rs`), **processor path lacks it** → P3-06 |
| R-CONS-02  | MEDIUM   | ⛔ Open  | Saga compensation only covers vector→graph direction                              |
| R-CONS-03  | LOW      | ⛔ Open  | `pdf_documents.document_id` FK nullable during async processing                   |
| R-CONS-04  | HIGH (NEW) | ⛔ Open  | Document metadata read/write authority inversion (was R-DRY-04) — fix via P5-01   |
| R-CONS-05  | HIGH (NEW) | ⛔ Open  | Orchestrator `delete_document` does not clean `documents` table row nor entity vectors on partial update — P3-07 |
| R-ID-01    | **CRITICAL (NEW)** | ⛔ Open  | Entity identity SSOT broken — async processor writes raw `entity.name` as graph node id and `entity:{raw}` as vector id, while merger/sync-upload normalize. Duplicate nodes + invisible vectors. See file 18 §1, plan-19 P-G1 |
| R-DRY-06   | **CRITICAL (NEW)** | ⛔ Open  | Three ingestion persistence paths with inverted batching; production (async processor) is least correct and bypasses the merger. See file 18 §2, plan-19 P-G2 |
| R-PERF-01  | HIGH (NEW) | ⛔ Open  | Processor per-chunk + per-entity vector writes are O(C)/O(E) round-trips; merger is O(E+R) sequential. See file 18 §4.1, plan-19 P-G4 |
| R-PERF-02  | HIGH (NEW) | ⛔ Open  | Query Global mode N+1 `node_degree` (Local was fixed, Global not). See file 18 §4.2, plan-19 P-G3 |
| R-CONS-06  | HIGH (NEW) | ⛔ Open  | Processor compensation wired only for node-batch failure; edge/entity-vector/sync-upload failures orphan data. See file 18 §7, plan-19 P-G5 |
| R-DRY-07   | HIGH (NEW) | ⛔ Open  | Three query engines (legacy `QueryEngine`, `strategies/*`, `SOTAQueryEngine`) + dead `chunk_retrieval.rs`; API sync handler fake rerank contradicts engine BM25. See file 18 §5.2-5.3, plan-19 P-G6 |
| R-PERF-03  | MEDIUM (NEW) | ⛔ Open  | O(W) KV `keys()` scans on reprocess + PDF resume. See file 18 §4.3, plan-19 P-G7 |
| R-CONS-07  | MEDIUM (NEW) | ⛔ Open  | Bypass mode broken at HTTP (returns apology, not direct LLM); Mix mode == Hybrid (docs lie). See file 18 §8.2, plan-19 P-G8 |
| R-PERF-04  | MEDIUM (NEW) | ⛔ Open  | No query-result / query-embedding cache; every request re-embeds. See file 18 §9, plan-19 P-G9 |
| R-SOLID-05 | MEDIUM (NEW) | ⛔ Open  | LSP batch-default trap: memory adapter inherits N+1 `upsert_nodes_batch`; perf tests can lie. See file 18 §6.3, plan-19 P-G10 |
| R-CONS-08  | LOW (NEW)  | ⛔ Open  | `GraphStorageAnalyticsOps` default impls ignore workspace scoping (cross-workspace count leak). See file 18 §6.4, plan-19 P-G12 |

---

## How to Navigate

1. Start with [01-overview/01-executive-summary.md](01-overview/01-executive-summary.md) for the TL;DR (note: some claims there are superseded by file 12).
2. Read [02-schema/01-postgresql-tables.md](02-schema/01-postgresql-tables.md) for the full table inventory.
3. Read [03-pipelines/03-data-flow-diagrams.md](03-pipelines/03-data-flow-diagrams.md) for ASCII flow diagrams.
4. Read [06-first-principles/07-cqrs-dual-store-design.md](06-first-principles/07-cqrs-dual-store-design.md) for the CQRS dual-store insight (why entities/relationships tables should be populated, not dropped).
5. Read [06-first-principles/08-sync-ascending-compat.md](06-first-principles/08-sync-ascending-compat.md) for the migration strategy (ascending compatibility, backfill design).
6. Read [06-first-principles/09-drift-detection-autorepair.md](06-first-principles/09-drift-detection-autorepair.md) for schema drift detection and auto-repair SQL.
7. Read [06-first-principles/10-battle-tested-improvement-plan.md](06-first-principles/10-battle-tested-improvement-plan.md) for the battle-tested action plan.
8. Read [06-first-principles/11-ux-zero-documents-root-cause-assessment.md](06-first-principles/11-ux-zero-documents-root-cause-assessment.md) for the "0 documents" UX root cause.
9. **Read [06-first-principles/12-code-verified-reassessment.md](06-first-principles/12-code-verified-reassessment.md) for the authoritative code-verified reassessment and improved plan (2026-06-25).**
10. **Read [06-first-principles/18-ingestion-query-deep-audit.md](06-first-principles/18-ingestion-query-deep-audit.md) for the authoritative ingestion+query algorithmic audit (DRY/SOLID/first-principles/perf/O(N), RC-6..RC-17, 2026-06-26) and [06-first-principles/19-ingestion-query-improvement-plan.md](06-first-principles/19-ingestion-query-improvement-plan.md) for its closure plan (Phases G1-G12).**

> **NOTE**: [06-first-principles/02-improvement-plan.md] is the initial plan (now superseded by 10 and 12).
> It recommended dropping entities/relationships tables — that was wrong (see 07 for correction).
> **NOTE**: Some claims in 01-executive-summary.md and 05-risks/01-dry-violations.md are stale
> (e.g. "entities/relationships orphaned", "drop them", "documents table primary"). File 12
> corrects them with line-level code references; defer to file 12 wherever they conflict.
> **NOTE**: Files 17 and 19 are **complementary authoritative plans**: 17 owns storage-consistency
> (RC-1..5: read-authority, write-path closure, saga symmetry, deletion coordination); 19 owns the
> ingestion/query algorithmic layer (RC-6..17: entity identity SSOT, single persistence path, query
> N+1, dead engines, caching, O(W) scans). They do not conflict; implement both.
