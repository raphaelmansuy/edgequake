# 01 — Executive Summary

> **Spec**: 021-storage-study  
> **File**: 01-overview/01-executive-summary.md  
> **Date**: 2026-06-25  
> **Verdict**: The storage layer is functionally correct but carries significant
> technical debt from an **evolutionary design** that introduced new stores
> (AGE graph, pgvector) without retiring the old relational tables they replaced.

---

## TL;DR (5-point summary)

1. **Three independent, living storage systems** share responsibility for the same
   knowledge graph data: Apache AGE (graph), `eq_*_kv` (KV), and `eq_*_vectors`
   (vector). A fourth set of relational tables (`entities`, `relationships`,
   `chunks`) serves as a **CQRS read model** — populated only when
   `entity_sync_mode != "disabled"` (see SPEC-021 file 07/12). Migration 039
   corrected their schema and dropped the vestigial `embedding` columns.

> **CORRECTION 2026-06-25 (file 12)**: The original text here said the
> `entities`/`relationships` tables were "effectively orphaned" and implied they
> should be dropped. That was wrong. They are a CQRS read model populated by
> `PostgresEntitySink` (`edgequake-api/src/postgres_entity_sink.rs`) when
> `entity_sync_mode ∈ {dual_write, full}`. Dropping them would remove a
> first-class analytics/FTS/JOIN capability. See file 07 and file 12.

2. **The pipeline writes in 3 stages** (KV → vector → graph), protected by a
   manual SAGA pattern. There is no 2-phase-commit and no distributed transaction.
   A crash between stage 2 and stage 3 produces orphaned chunk vectors with no
   corresponding graph data.

3. **`KVStorage` is used as a universal catch-all** for document metadata, chunk
   text, LLM cache entries, and task state. The key taxonomy is opaque
   (`{doc_id}-chunk-{n}`, `{doc_id}-metadata`) and encoded only in string
   literals scattered across the codebase — not in a single schema definition.

4. **`GraphStorage` violates the Interface Segregation Principle**: the trait
   requires implementors to provide 20+ methods across read, write, analytics,
   and scan domains. The ISP-split sub-traits exist but are still bundled into
   the composite `GraphStorage` supertrait, forcing full implementation.

5. **`AppState` violates the Single Responsibility Principle**: it is a
   monolithic struct carrying 15+ fields covering storage, auth, query, tasks,
   rate limiting, observability, and configuration.

---

## Key Questions Answered

> **CORRECTION 2026-06-25 (file 12)**: Two rows below were stale and are corrected
> here. The original answers reflected a pre-migration-039 reality and a misread
> of the read path.

| Question | Answer |
|----------|--------|
| Where is the source of truth for entities? | Apache AGE graph — `Node{node_id}` (traversal). The `entities` table is a CQRS analytics read model, populated when `entity_sync_mode != "disabled"`. |
| Where is the source of truth for relationships? | Apache AGE graph — `EDGE{source_id, target_id}` (traversal). `relationships` table is the CQRS read model. |
| Where is the source of truth for chunk text? | `eq_*_kv` table, key `{doc_id}-chunk-{n}` |
| Where is the source of truth for document metadata? | **Read authority = `eq_*_kv`** (`stats.rs::try_kv_storage_stats`, `documents/query/list.rs`). The `documents` table is a best-effort secondary write (`text_insert.rs` L1095-1139). This inversion is the root cause of the "0 documents" UX bug (file 11). |
| Where are embeddings stored? | `eq_*_vectors` (global) or `eq_{ns}_ws_{id}_vectors` (workspace-scoped) |
| Are `entities` and `relationships` tables used? | **Yes, conditionally** — written by `PostgresEntitySink` when `entity_sync_mode ∈ {dual_write, full}` (`postgres_entity_sink.rs`, `merger/entity.rs` L52-70, `text_insert.rs` L909-936). Default is `disabled` so they appear empty until enabled + backfilled (migration 040). |
| Are `chunks.embedding` / `entities.embedding` columns used? | **No, and now dropped** — migration 039 STEP 1 removes them. |
| What is the SAGA pattern for? | Compensating a failed graph-merge by deleting chunk vectors. **Note**: the orchestrator path (`ingestion.rs`) has the saga; the processor path (`text_insert.rs`) does not yet — see file 12 §4.2, P3-06. |

---

## Risk Heat Map

```
         PROBABILITY
         HIGH   MED   LOW
IMPACT  +------+------+------+
  HIGH  |R-DRY-01    |      |
        |R-DRY-02    |      |
        |R-CONS-01   |      |
        +------+------+------+
  MED   |      |R-DRY-03    |
        |      |R-DRY-04    |
        |      |R-SOLID-01  |
        |      |R-SOLID-02  |
        +------+------+------+
  LOW   |      |      |R-CONS-02|
        |      |      |R-CONS-03|
        +------+------+------+
```

---

## Quick Action Table

| Priority | Action | Risk Addressed |
|----------|--------|----------------|
| P0 | Document which tables are authoritative (add code comments + migration notes) | R-DRY-01, R-DRY-02 |
| P1 | Add a migration to drop `entities.embedding`, `chunks.embedding` columns | R-DRY-02 |
| P1 | Create a `KVKeySchema` module with all key patterns as constants | R-DRY-03 |
| P2 | Decouple `GraphStorage` into `GraphReader + GraphWriter + GraphAnalytics` | R-SOLID-01 |
| P2 | Split `AppState` into domain-specific state bundles | R-SOLID-02 |
| P3 | Write a cross-store invariant checker (CI) for orphan detection | R-CONS-01 |
