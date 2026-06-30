-- ============================================================================
-- Migration 068: Drop KV GIN value index (SPEC-034 IMP-03)
-- Version: 2.0.0 — 2026-06-30
--
-- PURPOSE:
--   Remove the GIN index on the `value` column of all eq_*_kv tables.
--
-- PROBLEM (from SPEC-034 evidence):
--   • Index size: 112 MB (155× the 760 KB heap)
--   • Scan count: 0 — zero queries use GIN content search on KV values
--   • KV values are 61 KB chunk text blobs; all lookups use the btree PK
--   • Every KV upsert maintains a 112 MB GIN index for zero benefit
--
-- TRANSACTION SAFETY:
--   * Regular DROP INDEX (no CONCURRENTLY) — works inside sqlx transaction.
--   * Brief ACCESS EXCLUSIVE lock per index — acceptable for fast DROP.
--   * IDEMPOTENT: IF EXISTS guard on every DROP.
--   * No data is ever changed — only the index structure is removed.
--
-- ROLLBACK:
--   CREATE INDEX <tablename>_value_gin ON <tablename> USING gin(value);
-- ============================================================================

DO $$
DECLARE
  v_tbl  text;
  v_idx  text;
BEGIN
  FOR v_tbl IN
    SELECT tablename
    FROM   pg_tables
    WHERE  schemaname = 'public'
      AND  tablename  LIKE 'eq_%_kv'
    ORDER  BY tablename
  LOOP
    v_idx := v_tbl || '_value_gin';

    IF EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = 'public' AND indexname = v_idx
    ) THEN
      -- Regular DROP INDEX (transaction-safe; brief table lock is acceptable
      -- since the drop operation itself is near-instantaneous).
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_idx);
      RAISE NOTICE 'SPEC-034 M068: Dropped KV GIN index % (freed ~112 MB)', v_idx;
    ELSE
      RAISE NOTICE 'SPEC-034 M068: KV GIN index % already absent — skipping', v_idx;
    END IF;
  END LOOP;
END $$;
