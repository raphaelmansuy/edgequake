-- ============================================================================
-- Migration 071: HNSW parameter optimization — ef_construction 64→32 (SPEC-034 IMP-04)
-- Version: 2.0.0 — 2026-06-30
--
-- PURPOSE:
--   Rebuild the HNSW vector similarity index with ef_construction=32 (from 64)
--   to reduce index size by ~35% with <2% recall degradation at ef_search=64.
--
-- PROBLEM (from SPEC-034 evidence):
--   Current: ef_construction=64 → 909 MB index (12.3× raw heap)
--   Target:  ef_construction=32 → ~600 MB index (still excellent recall)
--
-- TRANSACTION SAFETY:
--   * Uses regular CREATE INDEX (no CONCURRENTLY) — works in sqlx transaction.
--   * WHY no CONCURRENTLY: PostgreSQL forbids CONCURRENTLY inside transactions.
--     Regular CREATE INDEX holds a SHARE lock during build (allows reads, blocks
--     writes). During migration (service startup), no concurrent writes occur,
--     so the brief lock is acceptable.
--   * IDEMPOTENT: checks ef_construction in existing indexdef before rebuilding.
--   * Safe to re-run: skips tables that are already optimized.
--
-- TIMING NOTE:
--   Expect 30-120 seconds per workspace vectors table for ~5,000 vectors
--   at dim=1024. This runs at service startup during migration, acceptable.
--
-- ROLLBACK:
--   Recreate the index with ef_construction=64 using the same pattern.
-- ============================================================================

DO $$
DECLARE
  v_tbl     text;
  v_old_idx text;
  v_new_idx text;
  v_cur_def text;
BEGIN
  -- Verify pgvector is available
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
    RAISE NOTICE 'SPEC-034 M071: pgvector not installed — skipping HNSW optimization';
    RETURN;
  END IF;

  FOR v_tbl IN
    SELECT tablename
    FROM   pg_tables
    WHERE  schemaname = 'public'
      AND  tablename  LIKE 'eq_%_vectors'
      AND  tablename  NOT LIKE '%_stats'
    ORDER  BY tablename
  LOOP
    v_old_idx := v_tbl || '_embedding_idx';
    v_new_idx := v_tbl || '_embedding_idx_v2';

    -- ------------------------------------------------------------------
    -- Idempotency: skip if already rebuilt with ef_construction=32.
    -- ------------------------------------------------------------------
    SELECT indexdef INTO v_cur_def
    FROM   pg_indexes
    WHERE  schemaname = 'public' AND indexname = v_old_idx;

    IF v_cur_def IS NOT NULL AND v_cur_def LIKE '%ef_construction=32%' THEN
      RAISE NOTICE 'SPEC-034 M071: % already at ef_construction=32 — skipping', v_tbl;
      CONTINUE;
    END IF;

    -- Check v2 doesn't already exist (resume safety for interrupted migrations)
    IF EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = 'public' AND indexname = v_new_idx
    ) THEN
      -- v2 exists but rename not done yet — clean it up and restart
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_new_idx);
      RAISE NOTICE 'SPEC-034 M071: Cleaned up partial v2 index % for table %', v_new_idx, v_tbl;
    END IF;

    -- ------------------------------------------------------------------
    -- Step 1: Create new HNSW index with ef_construction=32.
    -- Uses regular CREATE INDEX (holds ShareLock during build — acceptable
    -- at startup since no concurrent writes occur during migration).
    -- ------------------------------------------------------------------
    EXECUTE format(
      'CREATE INDEX %I ON public.%I '
      'USING hnsw (embedding vector_cosine_ops) '
      'WITH (m = 16, ef_construction = 32)',
      v_new_idx, v_tbl
    );
    RAISE NOTICE 'SPEC-034 M071: Created % (ef_construction=32) for table %', v_new_idx, v_tbl;

    -- ------------------------------------------------------------------
    -- Step 2: Drop the old index (ef_construction=64).
    -- ------------------------------------------------------------------
    IF EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = 'public' AND indexname = v_old_idx
    ) THEN
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_old_idx);
      RAISE NOTICE 'SPEC-034 M071: Dropped old index % (ef_construction=64)', v_old_idx;
    END IF;

    -- ------------------------------------------------------------------
    -- Step 3: Rename new index to the canonical name.
    -- ------------------------------------------------------------------
    EXECUTE format('ALTER INDEX public.%I RENAME TO %I', v_new_idx, v_old_idx);
    RAISE NOTICE 'SPEC-034 M071: Renamed % → %', v_new_idx, v_old_idx;

  END LOOP;

  RAISE NOTICE 'SPEC-034 M071: HNSW optimization complete';
END $$;
