# SPEC-007: Vector Storage Optimization — Tier 2 & Tier 3

> **Parent**: [SPEC-005: Date & Document Pattern Filters](../005-filter.md)
> **Status**: Draft
> **Priority**: High
> **Target**: Post-Tier 1 (Tier 1 already shipped)

## Context

SPEC-005 defined a three-tier filtering strategy for narrowing RAG queries to
specific documents. **Tier 1** (post-retrieval context filter) is implemented
and working. This specification covers **Tier 2** (SQL-level metadata
pre-filter) and **Tier 3** (materialized columns with dedicated indexes).

## Problem Statement

With Tier 1, vector search computes cosine similarity for ALL vectors in the
table, then discards non-matching results in application code. This works well
for <1K documents but degrades at scale:

| Document Count | Vectors (est.) | Tier 1 Waste | SQL Pre-Filter Savings |
| :------------: | :------------: | :----------: | :--------------------: |
|      100       |      ~5K       |     <5%      |       Negligible       |
|     1,000      |      ~50K      |    10-40%    |          ~30%          |
|     10,000     |     ~500K      |    40-80%    |          ~60%          |
|    100,000     |      ~5M       |    80-95%    |          ~90%          |

## Document Index

| #   | Document                                                          | Purpose                                       |
| --- | ----------------------------------------------------------------- | --------------------------------------------- |
| 001 | [Needs Analysis](001-needs-analysis.md)                           | Clear requirements for Tier 2 & Tier 3        |
| 002 | [Codebase Audit](002-codebase-audit.md)                           | Deep analysis of current storage architecture |
| 003 | [Tier 2: SQL Pre-Filter](003-tier2-sql-prefilter.md)              | JSONB metadata WHERE clause on vector queries |
| 004 | [Tier 3: Materialized Columns](004-tier3-materialized-columns.md) | Dedicated columns + B-tree + partial indexes  |
| 005 | [Migration Strategy](005-migration-strategy.md)                   | Backward compatibility & automatic migration  |
| 006 | [Implementation Plan](006-implementation-plan.md)                 | Phased plan with roadblock mitigations        |

## Architecture Evolution

```
CURRENT (Tier 1)                 TIER 2                          TIER 3
+-------------------+    +-------------------------+    +---------------------------+
| Vector Table      |    | Vector Table            |    | Vector Table              |
| +-----------+     |    | +-----------+           |    | +-----------+             |
| | id  TEXT  |     |    | | id  TEXT  |           |    | | id  TEXT  |             |
| | embedding |     |    | | embedding |           |    | | embedding |             |
| | metadata  |--+  |    | | metadata  |--+        |    | | metadata  |             |
| | created_at|  |  |    | | created_at|  |        |    | | doc_id    |----B-tree   |
| +-----------+  |  |    | +-----------+  |        |    | | tenant_id |----B-tree   |
|                |  |    |                |        |    | | ws_id     |----B-tree   |
| HNSW index     |  |    | HNSW index    |        |    | | created_at|             |
| (embedding)    |  |    | (embedding)   |        |    | +-----------+             |
|                |  |    |                |        |    |                           |
| NO metadata    |  |    | +GIN index ----+        |    | HNSW index               |
| index          |  |    | (metadata jsonb_ops)    |    | (embedding)               |
+-------------------+    +-------------------------+    |                           |
                                                        | Partial HNSW per tenant   |
  Post-retrieval          SQL WHERE on JSONB            | (optional, high storage)  |
  filter in app code      before ANN scan               +---------------------------+
```

## Decision Record

| Decision                              | Rationale                                                                         | Cross-ref                                             |
| ------------------------------------- | --------------------------------------------------------------------------------- | ----------------------------------------------------- |
| Tier 2 before Tier 3                  | GIN index + JSONB WHERE is non-breaking, no schema migration                      | [003](003-tier2-sql-prefilter.md) §Rationale          |
| Extend trait with default             | Adding `metadata_filter` param with `None` default preserves all existing callers | [003](003-tier2-sql-prefilter.md) §Trait-Change       |
| Also push tenant filter to SQL        | Replaces 17 `matches_tenant_filter` sites, unifying all pre-filter at SQL level   | [003](003-tier2-sql-prefilter.md) §Tenant-Unification |
| Tier 3 column backfill from JSONB     | Zero-downtime migration: add columns, backfill, then switch queries               | [004](004-tier3-materialized-columns.md) §Migration   |
| Keep JSONB metadata alongside columns | Backward compat for any code reading metadata; columns are acceleration layer     | [005](005-migration-strategy.md) §Dual-Write          |
