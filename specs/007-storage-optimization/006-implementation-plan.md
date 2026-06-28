# 006 — Implementation Plan

> **Parent**: [SPEC-007 README](README.md) · **Cross-refs**: All documents 001–005
> **Requirements**: All R-T2-_ and R-T3-_ from [001-needs-analysis](001-needs-analysis.md)

---

## 1. Phase Overview

```
Phase 1        Phase 2           Phase 3           Phase 4
Tier 2 Core    Tier 2 Migrate    Tier 3 Schema     Tier 3 Query
(1 week)       (1 week)          (3 days)          (3 days)
────────────   ───────────────   ───────────────   ───────────────
MetadataFilter  Replace           ADD COLUMN         Column-based
query_filtered  matches_tenant_   + backfill         WHERE clauses
GIN migration   filter at 17+     + B-tree           FilterTier
                call sites        indexes            detection
                                  create_table()     Dual-write
```

## 2. Phase 1 — Tier 2 Core Infrastructure

**Goal**: Add metadata filtering capability to VectorStorage trait + PgVectorStorage + MemoryVectorStorage + GIN index.

### 2.1 Tasks

| #   | Task                                                                   | File(s)                                                       |  Risk  |
| --- | ---------------------------------------------------------------------- | ------------------------------------------------------------- | :----: |
| 1.1 | Define `MetadataFilter` type                                           | `edgequake-storage/src/traits/vector.rs`                      |  Low   |
| 1.2 | Add `query_filtered()` to `VectorStorage` trait with default impl      | `edgequake-storage/src/traits/vector.rs`                      |  Low   |
| 1.3 | Implement `query_filtered()` in `PgVectorStorage`                      | `edgequake-storage/src/adapters/postgres/vector.rs`           | Medium |
| 1.4 | Implement `query_filtered()` in `MemoryVectorStorage`                  | `edgequake-storage/src/adapters/memory/vector.rs`             |  Low   |
| 1.5 | Implement `query_filtered()` in `PgWorkspaceVectorRegistry` (delegate) | `edgequake-storage/src/adapters/postgres/workspace_vector.rs` |  Low   |
| 1.6 | Write migration 027 (GIN index)                                        | `migrations/027_add_gin_index_metadata.sql`                   |  Low   |
| 1.7 | Update `create_table()` in PgVectorStorage to include GIN index        | `edgequake-storage/src/adapters/postgres/vector.rs`           |  Low   |
| 1.8 | Unit tests for MetadataFilter + query_filtered                         | `edgequake-storage/src/adapters/postgres/tests/`              |  Low   |
| 1.9 | Integration test: GIN index is used (EXPLAIN ANALYZE)                  | `edgequake-storage/tests/`                                    |  Low   |

### 2.2 MetadataFilter Design (from [003](003-tier2-sql-prefilter.md))

```rust
// edgequake-storage/src/traits/vector.rs

/// Metadata-based filter for vector queries (Tier 2+).
/// All fields are optional; only non-None fields participate in filtering.
/// Multiple fields are AND-combined.
#[derive(Debug, Clone, Default)]
pub struct MetadataFilter {
    /// Filter by document ID(s). Matches JSONB key "document_id" OR "source_document_id".
    pub document_ids: Option<Vec<String>>,
    /// Filter by tenant ID.
    pub tenant_id: Option<String>,
    /// Filter by workspace ID.
    pub workspace_id: Option<String>,
}

impl MetadataFilter {
    pub fn is_empty(&self) -> bool {
        self.document_ids.is_none()
            && self.tenant_id.is_none()
            && self.workspace_id.is_none()
    }
}
```

### 2.3 Trait Method (backward-compatible)

```rust
// Added to VectorStorage trait — default delegates to existing query()
#[async_trait]
pub trait VectorStorage: Send + Sync {
    // ... existing methods ...

    /// Query with metadata filter (Tier 2+).
    /// Default: ignores metadata_filter, delegates to query().
    async fn query_filtered(
        &self,
        embedding: &[f32],
        top_k: usize,
        filter_ids: Option<&[String]>,
        metadata_filter: Option<&MetadataFilter>,
    ) -> Result<Vec<VectorSearchResult>> {
        // Default implementation ignores metadata_filter
        let _ = metadata_filter;
        self.query(embedding, top_k, filter_ids).await
    }
}
```

### 2.4 PgVectorStorage SQL Generation

```rust
// Pseudocode for query_filtered in PgVectorStorage
async fn query_filtered(&self, embedding, top_k, filter_ids, metadata_filter) -> Result<...> {
    let mut conditions: Vec<String> = vec![];
    let mut args = PgArguments::default();
    let mut param_idx = 1usize;

    // $1: embedding
    args.add(embedding_str);
    param_idx += 1;

    // Optional: filter by IDs (existing behavior)
    if let Some(ids) = filter_ids {
        conditions.push(format!("id = ANY(${param_idx}::text[])"));
        args.add(ids);
        param_idx += 1;
    }

    // Tier 2: metadata filter
    if let Some(mf) = metadata_filter {
        if let Some(doc_ids) = &mf.document_ids {
            // Handle both "document_id" and "source_document_id" keys
            conditions.push(format!(
                "(metadata->>'document_id' = ANY(${p}::text[]) OR metadata->>'source_document_id' = ANY(${p}::text[]))",
                p = param_idx
            ));
            args.add(doc_ids);
            param_idx += 1;
        }
        if let Some(tid) = &mf.tenant_id {
            conditions.push(format!("metadata->>'tenant_id' = ${param_idx}"));
            args.add(tid);
            param_idx += 1;
        }
        if let Some(wid) = &mf.workspace_id {
            conditions.push(format!("metadata->>'workspace_id' = ${param_idx}"));
            args.add(wid);
            param_idx += 1;
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score FROM {} {} ORDER BY embedding <=> $1::vector LIMIT {}",
        self.table_name, where_clause, top_k
    );

    // Execute with dynamic args
    let rows = sqlx::query_with(&sql, args).fetch_all(&pool).await?;
    // ... map to VectorSearchResult
}
```

### 2.5 Rollback

- Remove `query_filtered()` method and `MetadataFilter` type
- Drop GIN index via migration rollback
- All callers still use `query()` (unchanged)

---

## 3. Phase 2 — Tier 2 Migration (Caller Sites)

**Goal**: Replace all `matches_tenant_filter()` call sites with SQL-level filtering
via `query_filtered()`. Remove ~200 lines of post-filter code.

### 3.1 Tasks

| #   | Task                                                                        | File(s)                                                           |  Risk  |
| --- | --------------------------------------------------------------------------- | ----------------------------------------------------------------- | :----: |
| 2.1 | Add `metadata_filter` field to QueryRequest/QueryContext                    | `edgequake-query/src/engine.rs`, `edgequake-query/src/context.rs` |  Low   |
| 2.2 | Update `filter_context_by_document_ids()` to build MetadataFilter           | `edgequake-query/src/context_filter.rs`                           | Medium |
| 2.3 | Update vector_queries.rs: pass MetadataFilter to query_filtered() (6 sites) | `edgequake-query/src/sota_engine/vector_queries.rs`               | Medium |
| 2.4 | Update query_modes.rs: pass MetadataFilter to query_filtered() (11 sites)   | `edgequake-query/src/sota_engine/query_modes.rs`                  | Medium |
| 2.5 | Update query_basic.rs: construct MetadataFilter from QueryRequest           | `edgequake-query/src/sota_engine/query_entry/query_basic.rs`      |  Low   |
| 2.6 | Update query_workspace.rs: construct MetadataFilter from workspace config   | `edgequake-query/src/sota_engine/query_entry/query_workspace.rs`  |  Low   |
| 2.7 | Deprecate `matches_tenant_filter()` (mark #[deprecated], keep for rollback) | `edgequake-query/src/sota_engine/prompt.rs`                       |  Low   |
| 2.8 | Integration tests: verify SQL filter matches post-filter results            | `edgequake-query/tests/`                                          | Medium |
| 2.9 | Performance benchmark: Tier 1 vs Tier 2 latency comparison                  | `benches/`                                                        |  Low   |

### 3.2 Migration Strategy for 17 Call Sites

```
                    matches_tenant_filter → query_filtered
                    ─────────────────────────────────────
    Call Site Category          Count   Migration Approach
    ─────────────────────────   ─────   ──────────────────────────
    vector_queries.rs           6       Pass MetadataFilter from
      query_vectors()                     query context down to
      query_chunks()                      storage.query_filtered()
      query_entities()
      etc.

    query_modes.rs              11      Same pattern: extract tenant_id
      naive_mode()                        + workspace_id from context,
      local_mode()                        build MetadataFilter, pass
      global_mode()                       to query_filtered()
      hybrid_mode()
      mix_mode()
      etc.
    ─────────────────────────   ─────   ──────────────────────────
    Total                       17      All follow same pattern
```

### 3.3 Incremental Migration Approach

To minimize blast radius, migrate in sub-phases:

1. **2.A**: Migrate `vector_queries.rs` (6 sites) — these are the lowest-level callers
2. **2.B**: Migrate `query_modes.rs` (11 sites) — these call vector_queries.rs
3. **2.C**: Run full test suite, validate identical results
4. **2.D**: Mark `matches_tenant_filter()` as `#[deprecated]`

### 3.4 Rollback

- Revert caller changes (restore matches_tenant_filter usage)
- query_filtered() default impl still delegates to query() → safe
- No migration rollback needed (GIN index doesn't hurt)

---

## 4. Phase 3 — Tier 3 Schema Infrastructure

**Goal**: Add materialized columns, backfill from JSONB, create B-tree indexes.

### 4.1 Tasks

| #   | Task                                                                | File(s)                                             |  Risk  |
| --- | ------------------------------------------------------------------- | --------------------------------------------------- | :----: |
| 3.1 | Write migration 028 (ADD COLUMN + backfill)                         | `migrations/028_add_vector_columns.sql`             | Medium |
| 3.2 | Write migration 029 (B-tree indexes)                                | `migrations/029_add_vector_btree_indexes.sql`       |  Low   |
| 3.3 | Update `create_table()` to include columns + B-tree indexes         | `edgequake-storage/src/adapters/postgres/vector.rs` |  Low   |
| 3.4 | Implement dual-write in `upsert()`                                  | `edgequake-storage/src/adapters/postgres/vector.rs` | Medium |
| 3.5 | Implement `detect_filter_tier()`                                    | `edgequake-storage/src/adapters/postgres/vector.rs` | Medium |
| 3.6 | Cache filter tier in `AtomicU8` on PgVectorStorage                  | `edgequake-storage/src/adapters/postgres/vector.rs` |  Low   |
| 3.7 | Test migration on empty database                                    | `edgequake-storage/tests/`                          |  Low   |
| 3.8 | Test migration on database with existing data                       | `edgequake-storage/tests/`                          | Medium |
| 3.9 | Test backfill correctness (COALESCE document_id/source_document_id) | `edgequake-storage/tests/`                          | Medium |

### 4.2 Rollback

- Drop columns: `ALTER TABLE ... DROP COLUMN IF EXISTS document_id, ...`
- Drop indexes: `DROP INDEX CONCURRENTLY IF EXISTS idx_...`
- Code auto-detects Tier 2 and falls back

---

## 5. Phase 4 — Tier 3 Query Activation

**Goal**: Switch query path from JSONB WHERE to column-based WHERE when Tier 3 is detected.

### 5.1 Tasks

| #   | Task                                                                 | File(s)                                             |  Risk  |
| --- | -------------------------------------------------------------------- | --------------------------------------------------- | :----: |
| 4.1 | Add `FilterTier` enum                                                | `edgequake-storage/src/adapters/postgres/vector.rs` |  Low   |
| 4.2 | Implement column-based query path in `query_filtered()`              | `edgequake-storage/src/adapters/postgres/vector.rs` | Medium |
| 4.3 | Route query to correct path based on FilterTier                      | `edgequake-storage/src/adapters/postgres/vector.rs` |  Low   |
| 4.4 | Integration test: column-based filter produces same results as JSONB | `edgequake-storage/tests/`                          | Medium |
| 4.5 | EXPLAIN ANALYZE test: B-tree index is used                           | `edgequake-storage/tests/`                          |  Low   |
| 4.6 | Performance benchmark: Tier 2 vs Tier 3 latency                      | `benches/`                                          |  Low   |
| 4.7 | End-to-end query test with full pipeline                             | `edgequake-core/tests/`                             | Medium |

### 5.2 Column-Based SQL

```sql
-- Tier 3 query (generated by Rust code)
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM eq_default_vectors
WHERE document_id = ANY($2::text[])     -- B-tree index scan
  AND tenant_id = $3                    -- composite index
  AND workspace_id = $4
ORDER BY embedding <=> $1::vector
LIMIT $5;
```

### 5.3 Rollback

- Revert to JSONB query path in query_filtered()
- Or: just remove FilterTier detection → falls back to Tier 2

---

## 6. File Change Matrix

Complete inventory of files modified across all phases:

```
File                                                    │ Ph1 │ Ph2 │ Ph3 │ Ph4
────────────────────────────────────────────────────────┼─────┼─────┼─────┼────
edgequake-storage/src/traits/vector.rs                  │  ✓  │     │     │
  MetadataFilter type, query_filtered() trait method    │     │     │     │
                                                        │     │     │     │
edgequake-storage/src/adapters/postgres/vector.rs       │  ✓  │     │  ✓  │ ✓
  query_filtered() impl, create_table(), upsert(),      │     │     │     │
  detect_filter_tier(), column-based query path         │     │     │     │
                                                        │     │     │     │
edgequake-storage/src/adapters/memory/vector.rs         │  ✓  │     │     │
  query_filtered() impl (HashMap filter)                │     │     │     │
                                                        │     │     │     │
edgequake-storage/src/adapters/postgres/               │  ✓  │     │     │
  workspace_vector.rs                                   │     │     │     │
  query_filtered() delegation                           │     │     │     │
                                                        │     │     │     │
edgequake-query/src/engine.rs                           │     │  ✓  │     │
  Add metadata_filter to QueryRequest                   │     │     │     │
                                                        │     │     │     │
edgequake-query/src/context.rs                          │     │  ✓  │     │
  Add metadata_filter to QueryContext                   │     │     │     │
                                                        │     │     │     │
edgequake-query/src/context_filter.rs                   │     │  ✓  │     │
  Build MetadataFilter from allowed_document_ids        │     │     │     │
                                                        │     │     │     │
edgequake-query/src/sota_engine/vector_queries.rs       │     │  ✓  │     │
  Replace matches_tenant_filter with query_filtered     │     │     │     │
  (6 sites)                                             │     │     │     │
                                                        │     │     │     │
edgequake-query/src/sota_engine/query_modes.rs          │     │  ✓  │     │
  Replace matches_tenant_filter with query_filtered     │     │     │     │
  (11 sites)                                            │     │     │     │
                                                        │     │     │     │
edgequake-query/src/sota_engine/prompt.rs               │     │  ✓  │     │
  Deprecate matches_tenant_filter()                     │     │     │     │
                                                        │     │     │     │
edgequake-query/src/sota_engine/query_entry/            │     │  ✓  │     │
  query_basic.rs                                        │     │     │     │
  Construct MetadataFilter from QueryRequest            │     │     │     │
                                                        │     │     │     │
edgequake-query/src/sota_engine/query_entry/            │     │  ✓  │     │
  query_workspace.rs                                    │     │     │     │
  Construct MetadataFilter from workspace config        │     │     │     │
                                                        │     │     │     │
migrations/027_add_gin_index_metadata.sql               │  ✓  │     │     │
                                                        │     │     │     │
migrations/028_add_vector_columns.sql                   │     │     │  ✓  │
                                                        │     │     │     │
migrations/029_add_vector_btree_indexes.sql             │     │     │  ✓  │
────────────────────────────────────────────────────────┴─────┴─────┴─────┴────
Total files: 14                                           6     8     4     1
```

## 7. Risk Register

All identified roadblocks from [003](003-tier2-sql-prefilter.md) and [004](004-tier3-materialized-columns.md)
consolidated with mitigations:

| ID  | Risk                                                         | Phase | Severity | Mitigation                                                                                      |
| --- | ------------------------------------------------------------ | :---: | :------: | ----------------------------------------------------------------------------------------------- |
| R1  | Dynamic PgArguments binding complexity                       |   1   |  Medium  | Proven pattern: `sqlx::query_with()` + `PgArguments::add()`. Unit test each filter combination. |
| R2  | `query_filtered()` trait breaks existing impls               |   1   |   Low    | Default impl delegates to `query()`. Zero change for impls that don't override.                 |
| R3  | GIN index creation blocks writes                             |   1   |   Low    | `CONCURRENTLY` avoids locks. May take 30s+ on large tables.                                     |
| R4  | Field name inconsistency (document_id vs source_document_id) |  1,3  |  Medium  | SQL: `OR` both keys in JSONB path. Column: `COALESCE()` in backfill. Unified in column.         |
| R5  | 17 call sites to update                                      |   2   |  Medium  | Incremental sub-phases (2.A→2.B→2.C→2.D). Full test suite between each.                         |
| R6  | Per-workspace tables need same indexes/columns               |  1,3  |  Medium  | Migration uses DO $$ loop over pg_tables. create_table() updated for new tables.                |
| R7  | Backfill on large tables                                     |   3   |  Medium  | Batched 10K rows, pg_sleep(0.1) between batches. RAISE NOTICE for progress.                     |
| R8  | CREATE INDEX CONCURRENTLY inside transaction                 |   3   |   High   | Split: migration 028 (transactional) + 029 (non-transactional).                                 |
| R9  | Filter tier detection adds cold-start latency                |   4   |   Low    | Cached in AtomicU8. Single query at startup (~1ms).                                             |

## 8. Test Strategy Summary

```
                      Testing Pyramid
                      ───────────────

                    ┌─────────────────┐
                    │   E2E Tests     │
                    │  Full pipeline  │ ← Phase 4.7
                    │  query → result │
                    ├─────────────────┤
                    │ Integration     │
                    │  PgVector +     │ ← Phase 1.9, 2.8, 4.4
                    │  real database  │
                    ├─────────────────┤
                    │   Unit Tests    │
                    │  MetadataFilter │ ← Phase 1.8
                    │  SQL generation │
                    │  MemoryVector   │
                    │  FilterTier     │
                    └─────────────────┘
```

### 8.1 Key Test Cases

| Test                                            | Validates             | Phase |
| ----------------------------------------------- | --------------------- | :---: |
| Empty MetadataFilter → same as query()          | Backward compat       |   1   |
| Single field filter (document_id only)          | SQL generation        |   1   |
| Multi-field filter (all 3 fields)               | AND combination       |   1   |
| document_id matches source_document_id          | OR handling           |   1   |
| MemoryVectorStorage filter                      | Non-Postgres path     |   1   |
| matches_tenant_filter == query_filtered results | Migration correctness |   2   |
| Backfill COALESCE correctness                   | Column population     |   3   |
| NULL column handling                            | Pre-backfill queries  |   3   |
| FilterTier::Tier3 → column WHERE                | Query routing         |   4   |
| FilterTier fallback chain                       | Degradation           |   4   |
| 1M vectors + filter → <15ms                     | Performance SLA       |   4   |

## 9. Definition of Done

### Phase 1 Complete When:

- [ ] `MetadataFilter` type exists and is exported
- [ ] `query_filtered()` on VectorStorage trait with default impl
- [ ] PgVectorStorage generates correct SQL with dynamic WHERE
- [ ] MemoryVectorStorage filters in-memory correctly
- [ ] Migration 027 creates GIN index on all vector tables
- [ ] `create_table()` includes GIN index for new tables
- [ ] All unit + integration tests pass
- [ ] `cargo clippy` clean, `cargo fmt` clean

### Phase 2 Complete When:

- [ ] QueryRequest has `metadata_filter` field
- [ ] All 17 `matches_tenant_filter` sites replaced with `query_filtered`
- [ ] `matches_tenant_filter()` marked `#[deprecated]`
- [ ] Integration test: SQL filter == post-filter results for 1000 vectors
- [ ] Full test suite passes (`cargo test --workspace --lib`)

### Phase 3 Complete When:

- [ ] Migration 028 adds columns + backfills
- [ ] Migration 029 creates B-tree indexes
- [ ] `create_table()` includes columns + indexes for new tables
- [ ] Dual-write in `upsert()` populates columns + JSONB
- [ ] Backfill correctly handles document_id/source_document_id
- [ ] All existing tests still pass

### Phase 4 Complete When:

- [ ] `FilterTier` enum and detection logic work
- [ ] Column-based WHERE clause used when Tier 3 detected
- [ ] Graceful fallback to Tier 2/1 when columns don't exist
- [ ] EXPLAIN ANALYZE confirms B-tree index usage
- [ ] E2E test: full query pipeline with Tier 3 active
- [ ] Performance benchmark shows improvement over Tier 2

---

**Previous**: [005 — Migration Strategy](005-migration-strategy.md) · **Index**: [SPEC-007 README](README.md)
