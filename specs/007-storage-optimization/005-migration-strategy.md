# 005 — Migration Strategy & Backward Compatibility

> **Parent**: [SPEC-007 README](README.md) · **Refs**: [003-tier2](003-tier2-sql-prefilter.md), [004-tier3](004-tier3-materialized-columns.md)
> **Requirements**: R-T2-03 (zero downtime), R-T3-03 (backward compat)
> **Convention**: Next available migration numbers are 027, 028, 029

---

## 1. Migration Overview

```
Timeline
─────────────────────────────────────────────────────────────────────────→ time

Phase 1: Tier 2          Phase 2: Tier 3 Schema      Phase 3: Tier 3 Query
(migrate 027)            (migrate 028)               (code switch)

┌─────────────────┐      ┌───────────────────┐       ┌─────────────────┐
│ 027: GIN index  │      │ 028: ADD COLUMN   │       │ Code: detect    │
│   on metadata   │      │   + backfill      │       │   columns →     │
│                 │      │   + B-tree indexes│       │   use column    │
│ + MetadataFilter│      │                   │       │   WHERE clauses │
│ + query_filtered│      │                   │       │                 │
└────────┬────────┘      └────────┬──────────┘       └────────┬────────┘
         │                        │                           │
    Tier 2 active            Tier 3 data ready         Tier 3 active
    (JSONB WHERE)            (columns populated)       (column WHERE)
```

## 2. Migration Numbering

| Migration | Purpose                                                                       | Tier | Category      |
| --------- | ----------------------------------------------------------------------------- | :--: | ------------- |
| **027**   | GIN index on `metadata` JSONB column                                          |  2   | Index         |
| **028**   | ADD COLUMN (document_id, tenant_id, workspace_id) + backfill + B-tree indexes |  3   | Schema + Data |

## 3. Migration 027 — GIN Index (Tier 2)

```sql
-- 027_add_gin_index_metadata.sql
-- @implements SPEC-007 R-T2-02

-- Default vector table
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_eq_default_vectors_metadata
    ON public.eq_default_vectors
    USING GIN (metadata jsonb_path_ops);

-- Per-workspace vector tables
DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN
        SELECT tablename FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq_%_vectors'
          AND tablename != 'eq_default_vectors'
    LOOP
        EXECUTE format(
            'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_metadata ON public.%I USING GIN (metadata jsonb_path_ops)',
            tbl.tablename, tbl.tablename
        );
    END LOOP;
END $$;
```

**Characteristics**:

- **Lock**: CONCURRENTLY → no table lock, readers/writers unblocked
- **Duration**: ~30 sec per 500K rows (I/O bound, indexes JSONB keys)
- **Rollback**: `DROP INDEX CONCURRENTLY idx_...`

### 3.1 Per-Workspace Table Handling

Problem: `PgWorkspaceVectorRegistry::create_workspace_storage()` creates new
tables dynamically. After migration 027, new tables need GIN indexes too.

Solution: Update `PgVectorStorage::create_table()` to include GIN index creation
(see [004 §7](004-tier3-materialized-columns.md#7-create_table-updates-)).

```
CREATE TABLE path
    ┌─────────────────────────────┐
    │  create_table() called      │
    │  ┌───────────────────────┐  │
    │  │ CREATE TABLE IF NOT   │  │
    │  │  EXISTS ...           │  │
    │  └─────────┬─────────────┘  │
    │            │                │
    │  ┌─────────▼─────────────┐  │
    │  │ CREATE INDEX (HNSW)   │  │◄─── Existing
    │  └─────────┬─────────────┘  │
    │            │                │
    │  ┌─────────▼─────────────┐  │
    │  │ CREATE INDEX (GIN)    │  │◄─── Tier 2 (NEW)
    │  └─────────┬─────────────┘  │
    │            │                │
    │  ┌─────────▼─────────────┐  │
    │  │ CREATE INDEX (B-tree) │  │◄─── Tier 3 (NEW)
    │  └─────────────────────────┘  │
    └─────────────────────────────┘
```

## 4. Migration 028 — Materialized Columns (Tier 3)

Full SQL in [004 §6.1](004-tier3-materialized-columns.md#61-migration-028).

**Three phases within this single migration**:

1. **ADD COLUMN** (instant, no table rewrite)
2. **Backfill** (batched UPDATE, 10K rows per batch, non-blocking)
3. **CREATE INDEX CONCURRENTLY** (B-tree on new columns)

### 4.1 Transaction Safety

Important: `CREATE INDEX CONCURRENTLY` cannot run inside a transaction block.
sqlx runs each migration inside a transaction by default.

**Solution options**:

| Option                 | Approach                                     | Complexity |
| ---------------------- | -------------------------------------------- | :--------: |
| A. Split migration     | Phase 1+2 in 028, Phase 3 in 029             |    Low     |
| B. Raw SQL outside txn | Use sqlx `run_unchecked()` or `COMMIT` trick |   Medium   |
| C. Application-level   | Create indexes in Rust code after migration  |    Low     |

**Recommended: Option A** — Split into two migrations:

- `028_add_vector_columns.sql` — ADD COLUMN + backfill (transactional)
- `029_add_vector_btree_indexes.sql` — CREATE INDEX CONCURRENTLY (non-transactional)

### 4.2 Migration 029 — B-tree Indexes

```sql
-- 029_add_vector_btree_indexes.sql
-- @implements SPEC-007 R-T3-02
-- NOTE: This migration must run outside a transaction (CREATE INDEX CONCURRENTLY)

-- Default table
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_eq_default_vectors_doc_id
    ON public.eq_default_vectors (document_id)
    WHERE document_id IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_eq_default_vectors_tenant_ws
    ON public.eq_default_vectors (tenant_id, workspace_id)
    WHERE tenant_id IS NOT NULL;

-- Per-workspace tables
DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN
        SELECT tablename FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq_%_vectors'
          AND tablename != 'eq_default_vectors'
    LOOP
        EXECUTE format(
            'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_doc_id ON public.%I (document_id) WHERE document_id IS NOT NULL',
            tbl.tablename, tbl.tablename
        );
        EXECUTE format(
            'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_tenant_ws ON public.%I (tenant_id, workspace_id) WHERE tenant_id IS NOT NULL',
            tbl.tablename, tbl.tablename
        );
    END LOOP;
END $$;
```

## 5. Backward Compatibility Matrix

```
Server Version  │ Migration State    │ Query Behavior        │ Write Behavior
────────────────┼────────────────────┼───────────────────────┼────────────────
Pre-Tier 2      │ No 027/028/029     │ Tier 1 (post-filter)  │ JSONB only
Tier 2 active   │ 027 applied        │ JSONB WHERE + GIN     │ JSONB only
Tier 3 partial  │ 028 applied        │ JSONB WHERE + GIN     │ Dual-write ✓
                │                    │ (columns exist but    │ (columns + JSONB)
                │                    │  detection not yet    │
                │                    │  enabled)             │
Tier 3 active   │ 028+029 applied    │ Column WHERE + B-tree │ Dual-write ✓
                │ + code deployed    │ (fallback to JSONB    │ (columns + JSONB)
                │                    │  if detection fails)  │
```

### 5.1 Graceful Degradation

The system automatically detects the current migration state and selects the
appropriate query path:

```rust
/// Determines the filter tier for this storage instance.
/// Cached at initialization time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterTier {
    /// No metadata filtering infrastructure
    Tier1PostFilter,
    /// GIN index on metadata, JSONB WHERE clauses
    Tier2JsonbPreFilter,
    /// Materialized columns with B-tree indexes
    Tier3ColumnPreFilter,
}

impl PgVectorStorage {
    async fn detect_filter_tier(&self) -> FilterTier {
        // Check for materialized columns
        if self.has_column("document_id").await {
            // Check if B-tree index exists
            if self.has_index(&format!("idx_{}_doc_id", self.safe_table_name())).await {
                return FilterTier::Tier3ColumnPreFilter;
            }
            // Columns exist but no indexes yet (028 applied, 029 pending)
            // Still use JSONB path - column scan without index is worse than GIN
            return FilterTier::Tier2JsonbPreFilter;
        }

        // Check for GIN index on metadata
        if self.has_index(&format!("idx_{}_metadata", self.safe_table_name())).await {
            return FilterTier::Tier2JsonbPreFilter;
        }

        // No infrastructure → post-filter only
        FilterTier::Tier1PostFilter
    }
}
```

### 5.2 Version Detection SQL

```sql
-- Check if GIN index exists (Tier 2 ready)
SELECT EXISTS (
    SELECT 1 FROM pg_indexes
    WHERE tablename = $1
      AND indexname LIKE '%metadata%'
) AS has_gin;

-- Check if materialized columns exist (Tier 3 data ready)
SELECT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public'
      AND table_name = $1
      AND column_name = 'document_id'
) AS has_columns;

-- Check if B-tree indexes exist (Tier 3 fully active)
SELECT EXISTS (
    SELECT 1 FROM pg_indexes
    WHERE tablename = $1
      AND indexname LIKE '%doc_id%'
) AS has_btree;
```

## 6. Rollback Strategy

### 6.1 Tier 2 Rollback

```sql
-- Undo migration 027
DROP INDEX CONCURRENTLY IF EXISTS idx_eq_default_vectors_metadata;

DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN
        SELECT tablename FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq_%_vectors'
    LOOP
        EXECUTE format(
            'DROP INDEX CONCURRENTLY IF EXISTS idx_%s_metadata',
            tbl.tablename
        );
    END LOOP;
END $$;
```

**Impact**: System gracefully falls back to Tier 1 (post-filter). No data loss.

### 6.2 Tier 3 Rollback

```sql
-- Undo migration 029 (indexes)
DROP INDEX CONCURRENTLY IF EXISTS idx_eq_default_vectors_doc_id;
DROP INDEX CONCURRENTLY IF EXISTS idx_eq_default_vectors_tenant_ws;

-- Undo migration 028 (columns) — OPTIONAL, columns don't hurt
ALTER TABLE public.eq_default_vectors
    DROP COLUMN IF EXISTS document_id,
    DROP COLUMN IF EXISTS tenant_id,
    DROP COLUMN IF EXISTS workspace_id;
```

**Impact**: System gracefully falls back to Tier 2 (JSONB WHERE). No data loss.
Columns can be left in place (they're just NULL for new writes without dual-write
code, and don't affect correctness).

## 7. Zero-Downtime Deployment Sequence

```
┌──────────────────────────────────────────────────────────────────────┐
│                    DEPLOYMENT SEQUENCE                                │
│                                                                      │
│  1. Deploy code with query_filtered() + FilterTier detection         │
│     (code is backward-compatible, detects Tier 1 and behaves same)   │
│                                                                      │
│  2. Run migration 027 (GIN index)                                    │
│     → CONCURRENTLY: no downtime                                      │
│     → Code auto-detects Tier 2, starts using JSONB WHERE             │
│                                                                      │
│  3. Monitor: EXPLAIN ANALYZE confirms GIN usage                      │
│     → Validate Tier 2 performance meets SLA                          │
│                                                                      │
│  4. Run migration 028 (ADD COLUMN + backfill)                        │
│     → ADD COLUMN: instant                                            │
│     → Backfill: batched, non-blocking, progress logged               │
│     → Code auto-detects columns exist, starts dual-write             │
│                                                                      │
│  5. Run migration 029 (B-tree indexes)                               │
│     → CONCURRENTLY: no downtime                                      │
│     → Code auto-detects Tier 3, starts using column WHERE            │
│                                                                      │
│  6. Monitor: EXPLAIN ANALYZE confirms B-tree usage                   │
│     → Validate Tier 3 performance meets SLA                          │
│                                                                      │
│  ROLLBACK at any step: just undo the migration.                      │
│  Code auto-detects lower tier and falls back gracefully.             │
└──────────────────────────────────────────────────────────────────────┘
```

## 8. Testing the Migration

### 8.1 Pre-Migration Tests

```rust
#[tokio::test]
async fn test_query_works_without_gin_index() {
    // Ensure query_filtered falls back to post-filter (Tier 1)
    // when no GIN index exists
}

#[tokio::test]
async fn test_query_works_without_materialized_columns() {
    // Ensure query_filtered uses JSONB WHERE (Tier 2)
    // when columns don't exist but GIN does
}
```

### 8.2 Migration Tests

```rust
#[tokio::test]
async fn test_migration_027_creates_gin_index() {
    // Apply migration 027
    // Verify GIN index exists via pg_indexes
}

#[tokio::test]
async fn test_migration_028_adds_columns_and_backfills() {
    // Insert test vectors with JSONB metadata
    // Apply migration 028
    // Verify columns exist
    // Verify column values match JSONB metadata values
    // Verify COALESCE handled document_id/source_document_id correctly
}

#[tokio::test]
async fn test_migration_028_backfill_handles_missing_keys() {
    // Insert vectors with partial metadata (e.g., no tenant_id)
    // Apply migration 028
    // Verify column is NULL (not error)
}
```

### 8.3 Post-Migration Tests

```rust
#[tokio::test]
async fn test_tier3_query_uses_column_filter() {
    // Apply all migrations
    // Execute query with MetadataFilter
    // Verify EXPLAIN shows B-tree index scan (not GIN)
}

#[tokio::test]
async fn test_dual_write_populates_both() {
    // With all migrations applied
    // Upsert a vector
    // Verify metadata JSONB has values
    // Verify columns have values
    // Verify column values match JSONB values
}
```

## 9. Risks & Mitigations

| Risk                                        | Impact | Mitigation                                                        |
| ------------------------------------------- | :----: | ----------------------------------------------------------------- |
| Migration 028 backfill slow on large tables | Medium | Batched (10K), pg_sleep between batches; monitor via RAISE NOTICE |
| CONCURRENTLY index creation fails halfway   |  Low   | Re-run; IF NOT EXISTS is idempotent; invalid indexes detectable   |
| sqlx transaction wraps CONCURRENTLY         |  High  | Split migrations: 028 (transactional), 029 (non-transactional)    |
| Dual-write adds INSERT latency              |  Low   | Single SQL statement, ~0.1ms overhead per upsert                  |
| Detection query adds cold-start latency     |  Low   | Cached once at `PgVectorStorage` initialization                   |
| New workspace tables miss columns           | Medium | `create_table()` updated to include columns + indexes             |

---

**Previous**: [004 — Tier 3: Materialized Columns](004-tier3-materialized-columns.md) · **Next**: [006 — Implementation Plan](006-implementation-plan.md)
