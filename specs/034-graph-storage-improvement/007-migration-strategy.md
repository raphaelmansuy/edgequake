# SPEC-034-007: Migration Strategy

> **Lens**: DevOps / Database Migration Expert  
> **Version**: 1.0.0 — 2026-06-30  
> **Constraint**: Zero-downtime, automatic migrations via sqlx `_sqlx_migrations`

---

## 1. Migration Principles for This Spec

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  CONSTRAINTS                                                                │
│                                                                             │
│  1. All migrations must be IDEMPOTENT (safe to re-run)                     │
│  2. All DDL changes to production tables use CONCURRENTLY where possible   │
│  3. Data is never deleted — only indexes are dropped/replaced              │
│  4. Each migration is self-contained and independently rollback-able       │
│  5. Graph data schema (AGE vertex/edge format) is NOT changed              │
│  6. Rust code changes are gated behind feature flags during transition     │
│  7. Migrations target ALL graph instances (any graph name matching pattern) │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Migration Plan Overview

```
Migration 067 ─ Add eq_next_node_graphid() helper function (for IMP-01)
Migration 068 ─ Drop KV GIN value index (IMP-03) 
Migration 069 ─ Drop duplicate content_tsv index on vectors tables (IMP-05)
Migration 070 ─ Index consolidation on AGE graph tables (IMP-02)
Migration 071 ─ HNSW parameter optimization: rebuild with ef_construction=32 (IMP-04)
Migration 072 ─ Add unique constraint for native SQL upsert conflict target (IMP-01)
```

---

## 3. Migration 067 — Native Graph Write Helpers

**File**: `edgequake/migrations/067_native_graph_write_helpers.sql`

```sql
-- Migration 067: Add helper functions for native SQL AGE write path
-- WHY: IMP-01 requires generating valid AGE graphids outside of cypher()
-- SAFE: Pure function additions — no data changes
-- IDEMPOTENT: Uses CREATE OR REPLACE

-- Function: Get or create label OID for a given graph+label name
CREATE OR REPLACE FUNCTION eq_get_label_oid(graph_name text, label_name text)
RETURNS bigint AS $$
DECLARE
  result bigint;
BEGIN
  SELECT l.oid::bigint INTO result
  FROM ag_catalog.ag_label l
  JOIN ag_catalog.ag_graph g ON l.graph = g.oid
  WHERE g.name = graph_name AND l.name = label_name;
  RETURN result;
END;
$$ LANGUAGE plpgsql STABLE;

-- Function: Generate next valid AGE graphid for a label
CREATE OR REPLACE FUNCTION eq_next_graphid(graph_name text, label_name text)
RETURNS ag_catalog.graphid AS $$
DECLARE
  label_oid bigint;
  seq_val bigint;
  seq_name text;
BEGIN
  label_oid := eq_get_label_oid(graph_name, label_name);
  IF label_oid IS NULL THEN
    RAISE EXCEPTION 'Label % not found in graph %', label_name, graph_name;
  END IF;
  seq_name := format('%I.%I', graph_name, label_name || '_id_seq');
  EXECUTE format('SELECT nextval(%L)', seq_name) INTO seq_val;
  RETURN ((label_oid << 32) | seq_val)::ag_catalog.graphid;
END;
$$ LANGUAGE plpgsql;

-- Convenience wrappers for Node and EDGE labels in any graph
CREATE OR REPLACE FUNCTION eq_next_node_id(graph_name text)
RETURNS ag_catalog.graphid AS $$
BEGIN
  RETURN eq_next_graphid(graph_name, 'Node');
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION eq_next_edge_id(graph_name text)
RETURNS ag_catalog.graphid AS $$
BEGIN
  RETURN eq_next_graphid(graph_name, 'EDGE');
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION eq_next_node_id IS 
  'SPEC-034 IMP-01: Generate valid AGE graphid for Node label (native SQL path)';
```

---

## 4. Migration 068 — Drop KV GIN Value Index

**File**: `edgequake/migrations/068_drop_kv_gin_value_index.sql`

```sql
-- Migration 068: Drop GIN index on KV value column
-- WHY: 112 MB index on 760 KB data — 155x overhead with zero query benefit
--      KV values are chunk text blobs; lookups always use the key (btree).
-- SAFE: DROP INDEX CONCURRENTLY — no table lock
-- ROLLBACK: CREATE INDEX CONCURRENTLY ... USING gin(value)

DO $$
DECLARE
  kv_tbl text;
  idx_name text;
BEGIN
  FOR kv_tbl IN
    SELECT tablename FROM pg_tables
    WHERE schemaname = 'public' AND tablename LIKE 'eq_%_kv'
  LOOP
    -- Pattern: eq_<tenant>_kv → index: eq_<tenant>_kv_value_gin
    idx_name := kv_tbl || '_value_gin';
    IF EXISTS (
      SELECT 1 FROM pg_indexes 
      WHERE schemaname = 'public' AND indexname = idx_name
    ) THEN
      EXECUTE format('DROP INDEX CONCURRENTLY IF EXISTS public.%I', idx_name);
      RAISE NOTICE 'Dropped KV GIN index: %', idx_name;
    ELSE
      RAISE NOTICE 'KV GIN index % not found (already dropped or never created)', idx_name;
    END IF;
  END LOOP;
END $$;
```

---

## 5. Migration 069 — Drop Duplicate FTS Index

**File**: `edgequake/migrations/069_drop_duplicate_fts_index.sql`

```sql
-- Migration 069: Remove duplicate content_tsv GIN indexes on vector tables
-- WHY: idx_eq_..._vectors_content_tsv is a duplicate of ..._vectors_content_tsv_idx
-- SAFE: DROP INDEX CONCURRENTLY — no table lock; one copy of the index remains

DO $$
DECLARE
  vec_tbl text;
  dup_idx text;
BEGIN
  FOR vec_tbl IN
    SELECT tablename FROM pg_tables
    WHERE schemaname = 'public' AND tablename LIKE 'eq_%_vectors'
      AND tablename NOT LIKE '%_stats'
  LOOP
    -- The duplicate has pattern: idx_<tablename>_content_tsv
    dup_idx := 'idx_' || vec_tbl || '_content_tsv';
    IF EXISTS (
      SELECT 1 FROM pg_indexes 
      WHERE schemaname = 'public' AND indexname = dup_idx
    ) THEN
      EXECUTE format('DROP INDEX CONCURRENTLY IF EXISTS public.%I', dup_idx);
      RAISE NOTICE 'Dropped duplicate FTS index: %', dup_idx;
    END IF;
  END LOOP;
END $$;
```

---

## 6. Migration 070 — AGE Index Consolidation

**File**: `edgequake/migrations/070_consolidate_age_indexes.sql`

```sql
-- Migration 070: Remove redundant/unused AGE graph indexes
-- WHY: 18+ indexes on Node label — 6× write amplification; 10+ never used
-- SAFE: DROP INDEX CONCURRENTLY (lock-free)
-- PREREQUISITE: Must have confirmed idx_scan=0 for removed indexes
-- NOTE: Targets ALL graphs matching 'eq_%_graph' pattern

DO $$
DECLARE
  g_name text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'AGE not installed — skipping';
    RETURN;
  END IF;

  FOR g_name IN SELECT name FROM ag_catalog.ag_graph LOOP
    -- Remove _ag_label_vertex indexes (these are on the fallback parent table
    -- which has 0 rows in EdgeQuake — all nodes are in the "Node" label table)
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex', 
      'idx_' || replace(g_name,'.','_') || '_node_id');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_' || replace(g_name,'.','_') || '_tenant_id');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_' || replace(g_name,'.','_') || '_workspace_id');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_' || replace(g_name,'.','_') || '_tenant_workspace');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_' || replace(g_name,'.','_') || '_entity_type');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_' || replace(g_name,'.','_') || '_vertex_source_id');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_' || replace(g_name,'.','_') || '_vertex_source_ids_gin');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_ag_vertex_props_gin');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_ag_vertex_tenant_id');
    PERFORM eq_drop_index_if_exists(g_name, '_ag_label_vertex',
      'idx_ag_vertex_workspace_id');

    -- Remove duplicate node_id index (agtype_access_operator form superseded by json->>'node_id')
    PERFORM eq_drop_index_if_exists(g_name, 'Node', 'idx_node_prop_node_id');

    RAISE NOTICE 'Consolidated indexes for graph: %', g_name;
  END LOOP;
END $$;

-- Helper: drop an index by graph+table+name pattern
CREATE OR REPLACE FUNCTION eq_drop_index_if_exists(
  g_name text, tbl text, idx text
) RETURNS void AS $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname = g_name AND indexname = idx) THEN
    EXECUTE format('DROP INDEX CONCURRENTLY IF EXISTS %I.%I', g_name, idx);
    RAISE NOTICE '  Dropped: %.%', g_name, idx;
  END IF;
END;
$$ LANGUAGE plpgsql;
```

---

## 7. Migration 071 — HNSW Parameter Optimization

**File**: `edgequake/migrations/071_hnsw_optimize.sql`

```sql
-- Migration 071: Rebuild HNSW with optimized ef_construction=32
-- WHY: ef_construction=64 creates 909 MB index; =32 reduces to ~600 MB
--      with minimal recall degradation (<2% at ef_search=64)
-- NOTE: This migration is SLOW — creates a new index CONCURRENTLY
--       then drops the old one. Estimated time: 10-30 minutes for 5898 vectors.
-- SAFE: CONCURRENTLY means no table lock; queries continue using old index

DO $$
DECLARE
  vec_tbl text;
  new_idx text;
  old_idx text;
BEGIN
  FOR vec_tbl IN
    SELECT tablename FROM pg_tables
    WHERE schemaname = 'public' AND tablename LIKE 'eq_%_vectors'
      AND tablename NOT LIKE '%_stats'
  LOOP
    old_idx := vec_tbl || '_embedding_idx';
    new_idx := vec_tbl || '_embedding_idx_v2';
    
    IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname='public' AND indexname=new_idx) THEN
      EXECUTE format(
        'CREATE INDEX CONCURRENTLY %I ON public.%I 
         USING hnsw (embedding vector_cosine_ops) 
         WITH (m=16, ef_construction=32)',
        new_idx, vec_tbl
      );
      RAISE NOTICE 'Created new HNSW index: %', new_idx;
    END IF;
    
    IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname='public' AND indexname=old_idx) THEN
      EXECUTE format('DROP INDEX CONCURRENTLY IF EXISTS public.%I', old_idx);
      RAISE NOTICE 'Dropped old HNSW index: %', old_idx;
    END IF;
    
    EXECUTE format('ALTER INDEX public.%I RENAME TO %I', new_idx, old_idx);
    RAISE NOTICE 'Renamed % to %', new_idx, old_idx;
  END LOOP;
END $$;
```

**⚠ Warning**: This migration must be run manually or with a long timeout setting.  
Do NOT include in automatic migration runs without a `statement_timeout = 0` guard.

---

## 8. Backward Compatibility Matrix

```
CHANGE                       | BACKWARD COMPAT | FORWARD COMPAT | ROLLBACK
─────────────────────────────────────────────────────────────────────────────
Drop KV GIN index            | ✅ Safe         | ✅ Safe        | Recreate index
Drop duplicate FTS index     | ✅ Safe         | ✅ Safe        | Recreate index
Drop redundant AGE indexes   | ✅ Safe*        | ✅ Safe        | Recreate indexes
Add graphid helper functions | ✅ Safe         | ✅ Safe        | Drop functions
Rebuild HNSW (lower ef)      | ✅ Safe†        | ✅ Safe        | Rebuild original
Native SQL write path        | ✅ Feature flag  | ✅ Feature flag | Set flag=0
─────────────────────────────────────────────────────────────────────────────
* Safe after confirming pg_stat_user_indexes shows idx_scan=0 for each
† Recall degradation is <2% — acceptable; configurable via ef_search
```

---

## 9. Automatic Migration Integration

All migrations in this spec follow the existing sqlx migration pattern:

```rust
// Cargo.toml in edgequake-api:
// sqlx-cli runs all migrations in filename order
// Migrations are tracked in _sqlx_migrations table
// Each migration runs exactly once (checksum-based)

// For SLOW migrations (071), add a guard:
DO $$
BEGIN
  -- Skip if this migration has already been run (idempotency)
  IF EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'eq_eq_default_ws_00000000_vectors_embedding_idx'
             AND indexdef LIKE '%ef_construction=32%') THEN
    RAISE NOTICE 'Migration 071 already applied — skipping';
    RETURN;
  END IF;
  -- ... rest of migration
END $$;
```

---

## 10. Testing the Migration

```bash
# 1. Run migrations on a local copy of production data
make postgres-start
DATABASE_URL=postgres://edgequake:edgequake@localhost/edgequake_test \
  sqlx migrate run --source edgequake/migrations

# 2. Verify table sizes reduced
docker exec edgequake-postgres psql -U edgequake -d edgequake_test -c "
SELECT tablename, pg_size_pretty(pg_total_relation_size('public.'||tablename)) 
FROM pg_tables WHERE tablename LIKE 'eq_%' ORDER BY 2 DESC;"

# 3. Run full test suite
cargo test --workspace --lib

# 4. Verify no regression in EXPLAIN plans
cargo test -p edgequake-storage --test explain_coverage
```
