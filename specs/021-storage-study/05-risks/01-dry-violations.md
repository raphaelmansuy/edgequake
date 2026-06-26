# 01 — DRY Violations

> **Spec**: 021-storage-study  
> **File**: 05-risks/01-dry-violations.md  
> **Date**: 2026-06-25

---

## R-DRY-01 — Dual Storage of Entities and Relationships — ✅ RESOLVED-BY-DESIGN (CQRS)

> **Status update 2026-06-25 (file 12)**: This is no longer a DRY violation.
> The `entities`/`relationships` tables are an **intentional CQRS read model**
> populated by `PostgresEntitySink` when `entity_sync_mode != "disabled"`.
> The "drop them" recommendation below is **retracted** — see file 07
> (cqrs-dual-store-design) and file 12 (code-verified reassessment).
> Migration 039 corrected their schema; migration 040 backfills from AGE.

### Description
The same entity and relationship data is represented in **two places**:
1. AGE graph (`Node{node_id}`, `EDGE{source_id, target_id}`) — written by the pipeline (PRIMARY for traversal)
2. PostgreSQL relational tables (`entities`, `relationships`) — CQRS read model for analytics/FTS/JOINs, populated by dual-write when enabled

### Evidence (verified 2026-06-25)
- `edgequake-pipeline/src/merger/entity.rs` L52-70: `self.relational_sink.upsert_entity(...)` is called after every graph upsert.
- `edgequake-api/src/processor/text_insert.rs` L909-936: dual-write loop over batch entities.
- `edgequake-api/src/postgres_entity_sink.rs`: `PostgresEntitySink::create_if_enabled` returns the real sink when `entity_sync_mode ∈ {dual_write, full}`, else `NoopEntitySink`.
- `edgequake/src/main.rs` L586-593: wires the sink into the processor.
- Migrations `039_cqrs_entities_schema.sql` + `040_entity_backfill_marker.sql` + `support/040/apply.sql` implement the ascending-compat backfill.

### Impact (residual)
- When `entity_sync_mode = disabled` (default), the tables exist but stay empty — looks like dead schema to a new developer. Mitigated by migration 039 comments + file 07.
- No residual correctness risk: AGE remains the traversal source of truth; relational is a derived read model.

### Recommendation
- **Do NOT drop** `entities`/`relationships`. Populate them via dual-write (file 07/08/09).
- The original "drop them" recommendation in this section is retracted.

---

## R-DRY-02 — Duplicate Embedding Storage — ✅ RESOLVED (migration 039)

> **Status update 2026-06-25 (file 12)**: Migration 039 STEP 1 drops both
> `chunks.embedding` and `entities.embedding` plus their HNSW indexes, after
> first dropping the `edgequake.chunks`/`edgequake.entities` SELECT-* views
> that pinned the column references (PRE-STEP). STEP 7 recreates the views
> with explicit column lists. This closes RB-03.

### Description
Chunk and entity embeddings were previously stored in two places; the
relational `embedding` columns were always NULL in production. Migration 039
removes them.

### Evidence
- `migrations/039_cqrs_entities_schema.sql` L55-83: PRE-STEP drops views; STEP 1 drops indexes + columns.
- `migrations/039_cqrs_entities_schema.sql` L200-236: STEP 7 recreates views without `embedding`.

### Recommendation
- Complete — no further action. Mark resolved in README.

---

## R-DRY-03 — KV Key Patterns Scattered as String Literals

### Description
The KV store key naming convention (`{doc_id}-metadata`, `{doc_id}-chunk-{n}`, etc.) is
implemented as **string literals scattered** across multiple files rather than centralized constants.

### Evidence — locations where KV keys are constructed

| File                                           | Key Pattern                            |
| ---------------------------------------------- | -------------------------------------- |
| `edgequake-core/src/orchestrator/ingestion.rs` | `format!("{}-metadata", doc_id)`       |
| `edgequake-core/src/orchestrator/ingestion.rs` | `format!("{}-chunk-{}", doc_id, n)`    |
| `edgequake-core/src/orchestrator/deletion.rs`  | `format!("{}-", doc_id)` (prefix scan) |
| `edgequake-pipeline/src/cache.rs`              | `format!("{}-cache", key_hash)`        |
| `edgequake-query/src/keywords/mod.rs`          | `format!("{}-kwcache", query_hash)`    |
| `edgequake-api/src/handlers/`                  | Various patterns                       |

### Impact
- **Refactoring risk**: Changing a key pattern requires a full-codebase text search and simultaneous data migration.
- **Typo bugs**: A single character difference between write and read paths silently loses data.
- **Operability**: No way to discover all key patterns from documentation alone.

### Recommendation
Create a single module `edgequake-storage/src/kv_key_schema.rs`:
```rust
pub mod kv_keys {
    pub fn doc_metadata(doc_id: &str) -> String { format!("{doc_id}-metadata") }
    pub fn doc_chunk(doc_id: &str, n: usize) -> String { format!("{doc_id}-chunk-{n}") }
    pub fn doc_prefix(doc_id: &str) -> String { format!("{doc_id}-") }
    pub fn cache_entry(hash: &str) -> String { format!("{hash}-cache") }
    pub fn keyword_cache(hash: &str) -> String { format!("{hash}-kwcache") }
}
```

---

## R-DRY-04 — Document Metadata in Both `documents` Table and KV Store — ⛔ RECLASSIFIED → R-CONS-04

> **Status update 2026-06-25 (file 12)**: This is not a DRY violation. It is a
> **cross-store consistency / read-authority** problem. The running code reads
> from KV (`stats.rs::try_kv_storage_stats`, `documents/query/list.rs`) but the
> `documents` table is a best-effort secondary write. The inversion is the root
> cause of the "0 documents" UX symptom (file 11). See R-CONS-04 in README and
> P5-01 in file 12 §7.

### Description
Document metadata (title, hash, counts) exists in two stores with **inverted read/write authority**:
1. `eq_*_kv` key `{doc_id}-metadata` — written always (`processor/status_updates.rs::ensure_document_source_type`, `text_insert.rs` enrich); **read by dashboard/list** (`stats.rs::try_kv_storage_stats` L160-292, `documents/query/list.rs`).
2. `documents` relational table — written best-effort, `#[cfg(feature = "postgres")]`, only when `pdf_storage` present (`text_insert.rs` L1095-1139); **NOT consulted by dashboard/list**.

### Evidence (verified 2026-06-25)
- `handlers/workspaces/stats.rs` L107-110: `fetch_workspace_stats_uncached` calls `try_kv_storage_stats` exclusively; `try_postgres_stats` is `#[allow(dead_code)]` (L130).
- `handlers/documents/query/list.rs` L62-99: scans KV metadata keys + filters by tenant context; never queries `documents` table.
- `text_insert.rs` L1095-1139: the `documents` table write is gated behind `#[cfg(feature = "postgres")]` + `pdf_storage` presence + UUID parse success — three conditions for a "best-effort, non-fatal" write.

### Impact
- **User-visible bug**: Dashboard shows 0 documents when KV metadata is scoped to a different workspace_id than the selected workspace, even though `documents` table has rows. (Documented in file 11.)
- **Source-of-truth ambiguity**: The README previously claimed `documents` was primary; code says otherwise.

### Recommendation
- **P5-01 (file 12)**: Decide dashboard `document_count` read authority. Recommended: relational `documents` table primary (it has proper workspace_id indexing), KV as fallback. Update `try_kv_storage_stats` accordingly.
- Reclassify as R-CONS-04 in README (done).
