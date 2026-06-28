# 001 — Needs Analysis: Tier 2 & Tier 3 Storage Optimization

> **Parent**: [SPEC-007 README](README.md) · **Refs**: [SPEC-005](../005-filter.md), [002-codebase-audit](002-codebase-audit.md)

---

## 1. Core Problem

Tier 1 filtering (SPEC-005, shipped) resolves document IDs via KV scan, then
removes non-matching chunks **after** the vector similarity search completes.
This has two cost centers:

1. **Wasted ANN computation** — pgvector's HNSW evaluates cosine distance for
   vectors that will be discarded. At 10K documents (~500K vectors), if a filter
   selects 10% of documents, ~90% of the HNSW traversal visits irrelevant
   vectors.

2. **Wasted `matches_tenant_filter` calls** — 17 call sites in application code
   iterate over vector results and discard non-tenant items. This duplicates
   work that PostgreSQL can do natively with a WHERE clause.

## 2. Who Needs This

| Persona                      | Pain Point                                             | Tier |
| ---------------------------- | ------------------------------------------------------ | ---- |
| **Large corpus user**        | >5K documents; filtered queries slower than unfiltered | 2    |
| **Multi-tenant operator**    | Tenant isolation is post-retrieval; DB-level is safer  | 2    |
| **High-selectivity user**    | Filters that match <5% of docs waste >95% compute      | 2    |
| **Enterprise at 100K+ docs** | JSONB extraction per-row is CPU bound; B-tree instant  | 3    |
| **Compliance auditor**       | Wants provable tenant data isolation at DB level       | 3    |

## 3. Requirements (Tier 2)

### R-T2-01: SQL-Level Metadata Pre-Filtering

The `VectorStorage::query()` method MUST accept an optional metadata filter
that is translated into a SQL `WHERE` clause **before** the HNSW index scan.

**Acceptance criteria:**

- Query with `metadata_filter: { "document_id": ["doc-a", "doc-b"] }` returns
  ONLY vectors whose `metadata->>'document_id'` matches one of the listed values.
- Query without metadata_filter behaves identically to today (backward compat).
- Performance: Filtered query on 500K vectors (selecting 10%) must be ≤2x the
  cost of an unfiltered query on 50K vectors.

### R-T2-02: GIN Index on Metadata Column

A GIN index (`jsonb_path_ops`) MUST be added to every vector table's `metadata`
column, enabling efficient JSONB key lookups.

**Acceptance criteria:**

- `EXPLAIN` for a metadata-filtered vector query shows GIN index usage.
- Index creation migration is idempotent (`CREATE INDEX IF NOT EXISTS`).
- Index applies to both default vector table and per-workspace tables.

### R-T2-03: Tenant/Workspace Filter Unification

The 17 `matches_tenant_filter` call sites SHOULD be replaceable with SQL-level
metadata filtering, reducing application-level post-filter to zero for
tenant/workspace isolation.

**Acceptance criteria:**

- A query with `tenant_id` and `workspace_id` params produces the same results
  as today but with the filter applied at SQL level.
- Zero `matches_tenant_filter` calls remain in hot query paths (vector_queries.rs,
  query_modes.rs) after migration.

### R-T2-04: Memory Backend Parity

`MemoryVectorStorage::query()` MUST support the same metadata filter interface.

**Acceptance criteria:**

- All existing unit tests pass without modification.
- New tests verify metadata filtering in MemoryVectorStorage.

## 4. Requirements (Tier 3)

### R-T3-01: Materialized Filter Columns

The vector table schema MUST be extended with explicit `document_id`,
`tenant_id`, and `workspace_id` TEXT columns alongside the JSONB metadata.

**Acceptance criteria:**

- Migration adds columns with `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`.
- Existing vectors are backfilled from `metadata->>'document_id'` etc.
- New upserts populate both JSONB metadata AND dedicated columns (dual-write).

### R-T3-02: B-Tree Indexes on Filter Columns

B-tree indexes MUST be created on `document_id`, `tenant_id`, `workspace_id`.

**Acceptance criteria:**

- `EXPLAIN` shows B-tree index scan (not seq scan) for column-based filters.
- Composite index `(tenant_id, workspace_id)` for tenant+workspace queries.

### R-T3-03: Column-Based Query Path

`PgVectorStorage::query()` MUST use column-based WHERE clauses instead of
JSONB extraction when materialized columns are available.

**Acceptance criteria:**

- Filtered query on 500K vectors (10% selectivity) is ≤1.5x unfiltered on 50K.
- JSONB-based path still works as fallback (pre-migration tables).

### R-T3-04: Automatic Backfill Migration

Existing vector data MUST be backfilled from JSONB metadata to new columns
without downtime.

**Acceptance criteria:**

- Migration script handles NULL metadata gracefully.
- Backfill processes in batches (10K rows per batch) to avoid lock contention.
- Progress is logged for observability.

## 5. Non-Requirements (Explicitly Excluded)

| Excluded Item                 | Reason                                                   |
| ----------------------------- | -------------------------------------------------------- |
| Partial HNSW index per tenant | Storage cost is N×(full HNSW), deferred to Tier 4        |
| RLS policy integration        | Orthogonal concern; RLS already exists for graph/KV      |
| Vector dimension in filter    | Handled by workspace isolation (SPEC-033)                |
| Date-range SQL filtering      | `created_at` is NOT in vector metadata; stays in KV scan |
| Cross-workspace vector query  | Architecturally forbidden by per-workspace tables        |

## 6. Success Metrics

| Metric                                        | Tier 1 Baseline | Tier 2 Target    | Tier 3 Target             |
| --------------------------------------------- | --------------- | ---------------- | ------------------------- |
| 10% selectivity on 500K vectors (p50 latency) | ~45ms           | ~15ms            | ~8ms                      |
| matches_tenant_filter calls in hot path       | 17              | 0                | 0                         |
| Schema migration downtime                     | N/A             | 0 (index only)   | 0 (add column + backfill) |
| Backward compatibility breakage               | N/A             | 0 compile errors | 0 compile errors          |

## 7. Dependency Map

```
SPEC-005 (Tier 1: shipped)
    |
    +---> SPEC-007 Tier 2 (this spec)
    |         |
    |         +-- R-T2-01: VectorStorage trait change
    |         +-- R-T2-02: GIN index migration
    |         +-- R-T2-03: Tenant filter SQL unification
    |         +-- R-T2-04: MemoryVectorStorage parity
    |
    +---> SPEC-007 Tier 3 (depends on Tier 2)
              |
              +-- R-T3-01: ALTER TABLE ADD COLUMN migration
              +-- R-T3-02: B-tree indexes
              +-- R-T3-03: Column-based query path
              +-- R-T3-04: Backfill migration script
```

---

**Next**: [002 — Codebase Audit](002-codebase-audit.md)
