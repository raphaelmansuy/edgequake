-- ============================================================================
-- E2E: AGE Expression Index Verification (SPEC-021 P0-02)
-- File: specs/021-storage-study/e2e/verify_age_indexes.sql
-- Run against production DB: psql -U edgequake -d edgequake -f this_file.sql
-- ============================================================================

-- Step 1: List all expression indexes on the AGE vertex table
SELECT
    i.relname AS index_name,
    pg_get_indexdef(ix.indexrelid) AS index_def,
    ix.indisvalid AS is_valid,
    ix.indisunique AS is_unique,
    pg_size_pretty(pg_relation_size(i.oid)) AS index_size
FROM pg_class t
JOIN pg_index ix ON t.oid = ix.indrelid
JOIN pg_class i ON i.oid = ix.indexrelid
JOIN pg_namespace n ON n.oid = i.relnamespace
WHERE t.relname = '"Node"'  -- AGE stores nodes in "Node" label table
ORDER BY i.relname;

-- Step 2: Verify expression index IS used for tenant_id filter
-- EXPECTED OUTPUT: "Index Cond" (not "Filter") in EXPLAIN output
-- If output shows "Filter" instead, the index is NOT being used.
EXPLAIN (ANALYZE false, BUFFERS false, FORMAT TEXT)
SELECT ag_catalog.agtype_to_json(properties)->>'node_id' AS node_id
FROM edgequake."Node"
WHERE ag_catalog.agtype_to_json(properties)->>'tenant_id' = 'test-tenant-id'
LIMIT 10;

-- Step 3: Check index statistics for freshness
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan AS times_used,
    idx_tup_read AS tuples_read,
    idx_tup_fetch AS tuples_fetched
FROM pg_stat_user_indexes
WHERE tablename = '"Node"'
ORDER BY idx_scan DESC;

-- Step 4: Verify tsvector stored column exists (after migration 039)
SELECT
    column_name,
    data_type,
    is_generated,
    generation_expression
FROM information_schema.columns
WHERE table_name = 'entities'
  AND column_name IN ('tsv', 'sync_status', 'source_chunk_ids');
-- EXPECT: 3 rows after migration 039 runs

-- Step 5: Check sync mode
SELECT key, value FROM server_config WHERE key = 'entity_sync_mode';
-- EXPECT: "disabled" before mig-040, "dual_write" during, "full" after backfill
