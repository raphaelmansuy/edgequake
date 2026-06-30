-- ============================================================================
-- Migration 069: Drop duplicate content_tsv FTS index (SPEC-034 IMP-05)
-- Version: 2.0.0 — 2026-06-30
--
-- PURPOSE:
--   Remove the duplicate GIN index on the `content_tsv` column on all
--   eq_*_vectors tables. Two identical GIN indexes exist per table;
--   only the canonical form is needed.
--
-- PROBLEM (from SPEC-034 evidence):
--   • eq_*_vectors_content_tsv_idx  (canonical — KEEP)
--   • idx_eq_*_vectors_content_tsv  (duplicate — DROP this one)
--   Each is ~1.9 MB. The duplicate adds zero query coverage.
--   Every vector upsert maintains BOTH indexes — double the GIN write cost.
--
-- TRANSACTION SAFETY:
--   * Regular DROP INDEX (no CONCURRENTLY) — works inside sqlx transaction.
--   * IDEMPOTENT: IF EXISTS guard on every DROP.
--   * The canonical index is always verified present before dropping the dup.
--
-- ROLLBACK:
--   CREATE INDEX idx_<tablename>_content_tsv ON <tablename> USING gin(content_tsv);
-- ============================================================================

DO $$
DECLARE
  v_tbl  text;
  v_dup  text;
  v_can  text;
BEGIN
  FOR v_tbl IN
    SELECT tablename
    FROM   pg_tables
    WHERE  schemaname = 'public'
      AND  tablename  LIKE 'eq_%_vectors'
      AND  tablename  NOT LIKE '%_stats'
    ORDER  BY tablename
  LOOP
    v_dup := 'idx_' || v_tbl || '_content_tsv';   -- duplicate to drop
    v_can := v_tbl || '_content_tsv_idx';          -- canonical to keep

    -- Safety guard: only drop the duplicate if the canonical still exists.
    -- If canonical is absent, dropping the dup would remove ALL FTS support.
    IF NOT EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = 'public' AND indexname = v_can
    ) THEN
      RAISE NOTICE 'SPEC-034 M069: Canonical FTS index % absent — skipping drop of % to preserve FTS', v_can, v_dup;
      CONTINUE;
    END IF;

    IF EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = 'public' AND indexname = v_dup
    ) THEN
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_dup);
      RAISE NOTICE 'SPEC-034 M069: Dropped duplicate FTS index %', v_dup;
    ELSE
      RAISE NOTICE 'SPEC-034 M069: Duplicate FTS index % already absent — skipping', v_dup;
    END IF;
  END LOOP;
END $$;
