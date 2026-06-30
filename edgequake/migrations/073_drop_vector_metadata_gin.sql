-- ============================================================================
-- Migration 073: Drop vector metadata GIN index (SPEC-034 IMP-08)
-- Version: 2.0.0 — 2026-06-30
--
-- PURPOSE:
--   Remove the GIN index on the `metadata` JSONB column of all
--   eq_*_vectors tables.
--
-- PROBLEM (from SPEC-034 code audit + pg_stat evidence):
--   • Index size: ~13 MB per workspace vectors table
--   • Scan count: 0 — zero code paths use GIN containment search on metadata
--   • All metadata queries use `metadata->>'key' = value` (equality on
--     extracted text), which uses the btree indexes doc_id_idx / tenant_ws_idx
--   • Maintaining this GIN on every vector upsert is pure write overhead
--
-- TRANSACTION SAFETY:
--   * Regular DROP INDEX (no CONCURRENTLY) — works inside sqlx transaction.
--   * IDEMPOTENT: IF EXISTS guard on every DROP.
--   * Verified: btree metadata indexes (doc_id_idx, tenant_ws_idx) are KEPT.
--
-- ROLLBACK:
--   CREATE INDEX <tablename>_metadata_idx ON <tablename> USING gin(metadata);
-- ============================================================================

DO $$
DECLARE
  v_tbl text;
  v_idx text;
BEGIN
  FOR v_tbl IN
    SELECT tablename
    FROM   pg_tables
    WHERE  schemaname = 'public'
      AND  tablename  LIKE 'eq_%_vectors'
      AND  tablename  NOT LIKE '%_stats'
    ORDER  BY tablename
  LOOP
    v_idx := v_tbl || '_metadata_idx';

    IF EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = 'public' AND indexname = v_idx
    ) THEN
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_idx);
      RAISE NOTICE 'SPEC-034 M073: Dropped vector metadata GIN index % (~13 MB freed)', v_idx;
    ELSE
      RAISE NOTICE 'SPEC-034 M073: Vector metadata GIN index % already absent — skipping', v_idx;
    END IF;
  END LOOP;

  RAISE NOTICE 'SPEC-034 M073: Vector metadata GIN removal complete';
END $$;
