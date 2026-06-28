# 004 — Tier 3: Materialized Columns & Dedicated Indexes

> **Parent**: [SPEC-007 README](README.md) · **Refs**: [003-tier2](003-tier2-sql-prefilter.md), [005-migration](005-migration-strategy.md)
> **Requirements**: R-T3-01 through R-T3-04
> **Depends on**: Tier 2 must be implemented first

---

## 1. Rationale

Tier 2 filters via JSONB extraction (`metadata->>'key'`). This works but has
per-row CPU cost: PostgreSQL must parse the JSONB binary for every candidate row
to extract the text value. At 500K+ vectors, this extraction becomes a measurable
bottleneck.

Tier 3 promotes the most-filtered fields to dedicated TEXT columns with B-tree
indexes. Column access is a direct pointer dereference — orders of magnitude
faster than JSONB extraction.

**When to trigger Tier 3**: When EXPLAIN ANALYZE on a Tier 2 query shows >30% of
time spent in JSONB extraction (visible as "Filter" cost in the plan).

## 2. Target Schema

```
BEFORE (Tier 2)                         AFTER (Tier 3)
+----------------------------+          +-----------------------------------+
| eq_{prefix}_vectors        |          | eq_{prefix}_vectors               |
+----------------------------+          +-----------------------------------+
| id          TEXT (PK)      |          | id          TEXT (PK)             |
| embedding   vector(N)      |          | embedding   vector(N)             |
| metadata    JSONB           |          | metadata    JSONB                 |
| created_at  TIMESTAMPTZ    |          | created_at  TIMESTAMPTZ           |
|                            |          | document_id TEXT         (NEW)    |
|                            |          | tenant_id   TEXT         (NEW)    |
|                            |          | workspace_id TEXT        (NEW)    |
+----------------------------+          +-----------------------------------+
| Indexes:                   |          | Indexes:                          |
|   HNSW (embedding)         |          |   HNSW (embedding)                |
|   GIN  (metadata)          |          |   GIN  (metadata) -- kept         |
+----------------------------+          |   B-tree (document_id)            |
                                        |   B-tree (tenant_id, workspace_id)|
                                        +-----------------------------------+
```

## 3. Schema Details

### 3.1 New Columns

```sql
-- All columns are nullable for backward compatibility.
-- Existing rows will have NULL until backfill runs.
-- New rows will always have values (dual-write, see §5).

ALTER TABLE eq_{prefix}_vectors
    ADD COLUMN IF NOT EXISTS document_id  TEXT,
    ADD COLUMN IF NOT EXISTS tenant_id    TEXT,
    ADD COLUMN IF NOT EXISTS workspace_id TEXT;
```

**NULL handling**: Queries MUST handle NULL columns gracefully:

- If `document_id IS NULL`, the vector predates Tier 3 → treat as "unknown"
- Filter queries should use `document_id = ANY($x)` (NULL won't match)

### 3.2 document_id Resolution

Due to the field name inconsistency (see [003 §Field-Name-Inconsistency](003-tier2-sql-prefilter.md)):

```
Vector Type    JSONB Key               → Column Value
───────────    ──────────────          ──────────────
Chunk          metadata.document_id    → document_id column
Entity         metadata.source_document_id → document_id column  (UNIFIED)
Relationship   metadata.source_document_id → document_id column  (UNIFIED)
```

The `document_id` column UNIFIES both JSONB keys into a single column.
The backfill migration uses COALESCE:

```sql
UPDATE eq_{prefix}_vectors SET
    document_id  = COALESCE(metadata->>'document_id', metadata->>'source_document_id'),
    tenant_id    = metadata->>'tenant_id',
    workspace_id = metadata->>'workspace_id'
WHERE document_id IS NULL;
```

### 3.3 Indexes

```sql
-- Single-column indexes for individual filters
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_{table}_document_id
    ON {table} (document_id)
    WHERE document_id IS NOT NULL;

-- Composite index for tenant+workspace isolation (most common filter)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_{table}_tenant_workspace
    ON {table} (tenant_id, workspace_id)
    WHERE tenant_id IS NOT NULL;

-- Optional: covering index includes document_id for 3-key filter
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_{table}_tenant_ws_doc
    ON {table} (tenant_id, workspace_id, document_id)
    WHERE tenant_id IS NOT NULL;
```

**Partial indexes** (WHERE ... IS NOT NULL) save space by excluding pre-Tier 3
rows that haven't been backfilled. After backfill completes, these can be
recreated without the WHERE clause.

## 4. Query Path Changes

### 4.1 Column-Based SQL (Tier 3 Active)

```sql
-- Tier 3: Use column-based filter (B-tree index scan)
SELECT id, metadata, 1 - (embedding <=> $1::vector) AS score
FROM eq_default_vectors
WHERE tenant_id = $2
  AND workspace_id = $3
  AND document_id = ANY($4::text[])
ORDER BY embedding <=> $1::vector
LIMIT $5;
```

### 4.2 Fallback to JSONB (Tier 2)

If the `document_id` column doesn't exist (pre-migration table), fall back to
JSONB extraction:

```rust
impl PgVectorStorage {
    /// Detect if materialized columns exist.
    /// Cached at initialization time (checked once per PgVectorStorage instance).
    async fn has_materialized_columns(&self) -> bool {
        // Check if document_id column exists via information_schema
        // Cache result in AtomicBool for zero-cost subsequent checks
    }

    async fn query_filtered(&self, ...) -> Result<Vec<VectorSearchResult>> {
        if self.has_materialized_columns().await {
            self.query_filtered_columns(/* use column-based WHERE */).await
        } else {
            self.query_filtered_jsonb(/* use JSONB WHERE, Tier 2 */).await
        }
    }
}
```

### 4.3 Detection Strategy

```rust
// Cached at PgVectorStorage::initialize() time
async fn detect_materialized_columns(&self) -> Result<bool> {
    let pool = self.pool.get().await?;
    let (schema, table) = self.parse_table_name();

    let result: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = $1 AND table_name = $2
            AND column_name = 'document_id'
        )"
    )
    .bind(schema)
    .bind(table)
    .fetch_optional(&pool)
    .await?;

    Ok(result.map(|(b,)| b).unwrap_or(false))
}
```

## 5. Dual-Write Strategy {#Dual-Write}

After migration, new upserts MUST populate BOTH the JSONB metadata AND the
dedicated columns. This ensures:

1. Old code that reads metadata JSONB still works
2. New code that reads columns gets values
3. GIN index (Tier 2) stays valid
4. B-tree indexes (Tier 3) stay valid

```rust
// Modified PgVectorStorage::upsert()
async fn upsert(&self, data: &[(String, Vec<f32>, serde_json::Value)]) -> Result<()> {
    let has_columns = self.has_materialized_columns.load(Ordering::Relaxed);

    for (id, embedding, metadata) in data {
        let embedding_str = Self::format_embedding(embedding);

        let sql = if has_columns {
            // Tier 3: Write to both JSONB and columns
            format!(
                r#"INSERT INTO {} (id, embedding, metadata, document_id, tenant_id, workspace_id)
                   VALUES ($1, $2::vector, $3,
                           COALESCE($3->>'document_id', $3->>'source_document_id'),
                           $3->>'tenant_id',
                           $3->>'workspace_id')
                   ON CONFLICT (id) DO UPDATE SET
                       embedding = EXCLUDED.embedding,
                       metadata = EXCLUDED.metadata,
                       document_id = EXCLUDED.document_id,
                       tenant_id = EXCLUDED.tenant_id,
                       workspace_id = EXCLUDED.workspace_id"#,
                self.table_name
            )
        } else {
            // Pre-Tier 3: Write only JSONB (existing behavior)
            format!(
                r#"INSERT INTO {} (id, embedding, metadata)
                   VALUES ($1, $2::vector, $3)
                   ON CONFLICT (id) DO UPDATE SET
                       embedding = EXCLUDED.embedding,
                       metadata = EXCLUDED.metadata"#,
                self.table_name
            )
        };

        sqlx::query(&sql)
            .bind(id)
            .bind(&embedding_str)
            .bind(metadata)
            .execute(&pool)
            .await?;
    }
    Ok(())
}
```

**Key insight**: The `COALESCE($3->>'document_id', $3->>'source_document_id')`
in the INSERT handles the field name inconsistency at write time, so the column
always has a unified value.

## 6. Backfill Migration {#Backfill}

### 6.1 Migration 028

```sql
-- Migration: 028_add_vector_materialized_columns.sql
--
-- Adds document_id, tenant_id, workspace_id columns to vector tables.
-- Backfills from JSONB metadata in batches.
-- @implements SPEC-007 R-T3-01, R-T3-02, R-T3-04

-- ============================================================
-- PHASE 1: Add columns (instant, no table rewrite)
-- ============================================================

-- Default vector table
ALTER TABLE public.eq_default_vectors
    ADD COLUMN IF NOT EXISTS document_id  TEXT,
    ADD COLUMN IF NOT EXISTS tenant_id    TEXT,
    ADD COLUMN IF NOT EXISTS workspace_id TEXT;

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
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS document_id TEXT', tbl.tablename);
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS tenant_id TEXT', tbl.tablename);
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS workspace_id TEXT', tbl.tablename);
    END LOOP;
END $$;

-- ============================================================
-- PHASE 2: Backfill from JSONB metadata (batched, non-blocking)
-- ============================================================

-- Default table backfill
DO $$
DECLARE
    batch_size INT := 10000;
    updated INT;
BEGIN
    LOOP
        UPDATE public.eq_default_vectors SET
            document_id  = COALESCE(metadata->>'document_id', metadata->>'source_document_id'),
            tenant_id    = metadata->>'tenant_id',
            workspace_id = metadata->>'workspace_id'
        WHERE document_id IS NULL
          AND ctid IN (
              SELECT ctid FROM public.eq_default_vectors
              WHERE document_id IS NULL
              LIMIT batch_size
          );
        GET DIAGNOSTICS updated = ROW_COUNT;
        RAISE NOTICE 'Backfilled % rows in eq_default_vectors', updated;
        EXIT WHEN updated < batch_size;
        -- Short sleep to avoid monopolizing the DB
        PERFORM pg_sleep(0.1);
    END LOOP;
END $$;

-- Per-workspace table backfill
DO $$
DECLARE
    tbl RECORD;
    batch_size INT := 10000;
    updated INT;
BEGIN
    FOR tbl IN
        SELECT tablename FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq_%_vectors'
          AND tablename != 'eq_default_vectors'
    LOOP
        LOOP
            EXECUTE format(
                'UPDATE public.%I SET
                    document_id = COALESCE(metadata->>''document_id'', metadata->>''source_document_id''),
                    tenant_id = metadata->>''tenant_id'',
                    workspace_id = metadata->>''workspace_id''
                WHERE document_id IS NULL
                AND ctid IN (
                    SELECT ctid FROM public.%I WHERE document_id IS NULL LIMIT %s
                )',
                tbl.tablename, tbl.tablename, batch_size
            );
            GET DIAGNOSTICS updated = ROW_COUNT;
            RAISE NOTICE 'Backfilled % rows in %', updated, tbl.tablename;
            EXIT WHEN updated < batch_size;
            PERFORM pg_sleep(0.1);
        END LOOP;
    END LOOP;
END $$;

-- ============================================================
-- PHASE 3: Create B-tree indexes (CONCURRENTLY = no lock)
-- ============================================================

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

### 6.2 Backfill Observability

The migration uses `RAISE NOTICE` for progress logging. In production, this
appears in PostgreSQL logs. For programmatic monitoring:

```
                     Backfill Progress
    ┌──────────────────────────────────────────────┐
    │ Phase 1: ADD COLUMN (instant)                │
    │   eq_default_vectors          ✓ done         │
    │   eq_default_ws_a1b2_vectors  ✓ done         │
    │   eq_default_ws_e5f6_vectors  ✓ done         │
    │                                              │
    │ Phase 2: Backfill (batched 10K)              │
    │   eq_default_vectors                         │
    │     batch 1: 10000 rows  [##########] done   │
    │     batch 2: 10000 rows  [##########] done   │
    │     batch 3: 5432 rows   [#####     ] done   │
    │                                              │
    │ Phase 3: CREATE INDEX CONCURRENTLY           │
    │   idx_eq_default_vectors_doc_id      ✓       │
    │   idx_eq_default_vectors_tenant_ws   ✓       │
    └──────────────────────────────────────────────┘
```

## 7. create_table() Updates

After Tier 3, `PgVectorStorage::create_table()` creates tables with columns:

```rust
async fn create_table(&self) -> Result<()> {
    let pool = self.pool.get().await?;

    // Create table with materialized columns
    let sql = format!(
        r#"CREATE TABLE IF NOT EXISTS {} (
            id           TEXT PRIMARY KEY,
            embedding    vector({}) NOT NULL,
            metadata     JSONB DEFAULT '{{}}',
            created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            document_id  TEXT,
            tenant_id    TEXT,
            workspace_id TEXT
        )"#,
        self.table_name, self.dimension
    );
    sqlx::query(&sql).execute(&pool).await?;

    // HNSW index (existing)
    // ... existing HNSW index creation ...

    // GIN index on metadata (Tier 2)
    let gin_sql = format!(
        "CREATE INDEX IF NOT EXISTS idx_{table}_metadata ON {full} USING GIN (metadata jsonb_path_ops)",
        table = self.safe_table_name(), full = self.table_name
    );
    sqlx::query(&gin_sql).execute(&pool).await.ok();

    // B-tree indexes (Tier 3)
    let doc_idx = format!(
        "CREATE INDEX IF NOT EXISTS idx_{table}_doc_id ON {full} (document_id) WHERE document_id IS NOT NULL",
        table = self.safe_table_name(), full = self.table_name
    );
    sqlx::query(&doc_idx).execute(&pool).await.ok();

    let tenant_idx = format!(
        "CREATE INDEX IF NOT EXISTS idx_{table}_tenant_ws ON {full} (tenant_id, workspace_id) WHERE tenant_id IS NOT NULL",
        table = self.safe_table_name(), full = self.table_name
    );
    sqlx::query(&tenant_idx).execute(&pool).await.ok();

    Ok(())
}
```

## 8. Performance Comparison

Expected query plan comparison (500K vectors, 10% document selectivity):

```
TIER 1 (current):
  Index Scan using eq_default_vectors_embedding_idx
    -> Filter: matches_tenant_filter (app code, 17 sites)
  Time: ~45ms (HNSW) + ~5ms (app filter) = ~50ms

TIER 2 (GIN + JSONB WHERE):
  Bitmap Heap Scan on eq_default_vectors
    -> BitmapAnd
      -> Bitmap Index Scan on idx_metadata (GIN)
      -> Index Scan using embedding_idx (HNSW)
  Time: ~15ms (GIN bitmap + HNSW intersect)

TIER 3 (B-tree + column WHERE):
  Index Scan using eq_default_vectors_embedding_idx
    -> Filter: document_id = ANY('{...}')
    -> Rows Removed by Filter: 0 (B-tree pre-filters)
  Time: ~8ms (B-tree instant + HNSW on filtered subset)
```

## 9. Roadblocks & Mitigations

| Roadblock                        | Severity | Mitigation                                               | Cross-ref |
| -------------------------------- | :------: | -------------------------------------------------------- | --------- |
| **ALTER TABLE on large table**   |   Low    | ADD COLUMN IF NOT EXISTS is instant (no rewrite)         | §3.1      |
| **Backfill locks**               |  Medium  | Batched UPDATE with ctid + pg_sleep                      | §6.1      |
| **CREATE INDEX CONCURRENTLY**    |   Low    | CONCURRENTLY avoids write locks; may take time           | §6.1      |
| **Dual-write overhead**          |   Low    | Single INSERT handles both JSONB and columns             | §5        |
| **Pre-migration tables**         |  Medium  | Auto-detect via information_schema; fallback to JSONB    | §4.2      |
| **Per-workspace table creation** |  Medium  | Update create_table() to include columns + indexes       | §7        |
| **NULL columns before backfill** |   Low    | Partial indexes exclude NULLs; queries handle gracefully | §3.1      |

---

**Previous**: [003 — Tier 2: SQL Pre-Filter](003-tier2-sql-prefilter.md) · **Next**: [005 — Migration Strategy](005-migration-strategy.md)
