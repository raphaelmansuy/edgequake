-- ============================================================================
-- Migration 071: HNSW parameter optimization — ef_construction 64→32 (SPEC-034 IMP-04)
-- Version: 2.1.0 — 2026-07-03 (SPEC-042 / GitHub #275 dimension guard)
--
-- PURPOSE:
--   Rebuild the HNSW vector similarity index with ef_construction=32 (from 64)
--   to reduce index size by ~35% with <2% recall degradation at ef_search=64.
--
-- DIMENSION POLICY (pgvector HNSW ceilings — SSOT: capabilities.rs):
--   vector   column: HNSW max 2000 dimensions
--   halfvec  column: HNSW max 4000 dimensions
--   dim > 4000: skip ANN index (sequential scan fallback; issue #275)
--   dim ∈ (2000, 4000]: promote vector → halfvec before index build
--
-- TRANSACTION SAFETY:
--   * Uses regular CREATE INDEX (no CONCURRENTLY) — works in sqlx transaction.
--   * IDEMPOTENT: checks ef_construction in existing indexdef before rebuilding.
-- ============================================================================

DO $$
DECLARE
  v_tbl     text;
  v_old_idx text;
  v_new_idx text;
  v_cur_def text;
  v_dim     int;
  v_udt     text;
  v_opclass text;
  idx       record;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector') THEN
    RAISE NOTICE 'SPEC-034 M071: pgvector not installed — skipping HNSW optimization';
    RETURN;
  END IF;

  FOR v_tbl IN
    SELECT tablename
    FROM   pg_tables
    WHERE  schemaname = 'public'
      AND  tablename  LIKE 'eq\_%\_vectors' ESCAPE '\'
      AND  tablename  NOT LIKE '%\_stats' ESCAPE '\'
    ORDER  BY tablename
  LOOP
    SELECT c.udt_name
    INTO v_udt
    FROM information_schema.columns c
    WHERE c.table_schema = 'public'
      AND c.table_name = v_tbl
      AND c.column_name = 'embedding';

    IF NOT FOUND THEN
      CONTINUE;
    END IF;

    SELECT (regexp_match(format_type(a.atttypid, a.atttypmod), 'vector\((\d+)\)'))[1]::int
    INTO v_dim
    FROM pg_attribute a
    JOIN pg_class c ON c.oid = a.attrelid
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname = 'public'
      AND c.relname = v_tbl
      AND a.attname = 'embedding'
      AND NOT a.attisdropped;

    IF v_dim IS NULL AND v_udt = 'halfvec' THEN
      SELECT (regexp_match(format_type(a.atttypid, a.atttypmod), 'halfvec\((\d+)\)'))[1]::int
      INTO v_dim
      FROM pg_attribute a
      JOIN pg_class c ON c.oid = a.attrelid
      JOIN pg_namespace n ON n.oid = c.relnamespace
      WHERE n.nspname = 'public'
        AND c.relname = v_tbl
        AND a.attname = 'embedding'
        AND NOT a.attisdropped;
    END IF;

    IF v_dim IS NULL THEN
      RAISE NOTICE 'SPEC-034 M071: % — cannot resolve embedding dimension, skipping', v_tbl;
      CONTINUE;
    END IF;

    IF v_dim > 4000 THEN
      RAISE NOTICE 'SPEC-034 M071: % dim=% exceeds halfvec HNSW max (4000) — skipping ANN index (#275)', v_tbl, v_dim;
      CONTINUE;
    END IF;

    IF v_dim > 2000 THEN
      IF v_udt = 'vector' THEN
        FOR idx IN
          SELECT indexname
          FROM pg_indexes
          WHERE schemaname = 'public'
            AND tablename = v_tbl
            AND indexdef ILIKE '%embedding%'
            AND (indexdef ILIKE '%USING hnsw%' OR indexdef ILIKE '%USING ivfflat%')
        LOOP
          EXECUTE format('DROP INDEX IF EXISTS %I', idx.indexname);
        END LOOP;
        EXECUTE format(
          'ALTER TABLE %I ALTER COLUMN embedding TYPE halfvec(%s) USING embedding::halfvec(%s)',
          v_tbl, v_dim, v_dim
        );
        v_udt := 'halfvec';
        RAISE NOTICE 'SPEC-034 M071: % promoted vector(%) → halfvec(%) for HNSW (#275)', v_tbl, v_dim, v_dim;
      END IF;
      v_opclass := 'halfvec_cosine_ops';
    ELSE
      v_opclass := 'vector_cosine_ops';
    END IF;

    v_old_idx := v_tbl || '_embedding_idx';
    v_new_idx := v_tbl || '_embedding_idx_v2';

    SELECT indexdef INTO v_cur_def
    FROM   pg_indexes
    WHERE  schemaname = 'public' AND indexname = v_old_idx;

    IF v_cur_def IS NOT NULL
       AND v_cur_def LIKE '%ef_construction=32%'
       AND v_cur_def ILIKE '%' || v_opclass || '%' THEN
      RAISE NOTICE 'SPEC-034 M071: % already at ef_construction=32 with % — skipping', v_tbl, v_opclass;
      CONTINUE;
    END IF;

    IF EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = 'public' AND indexname = v_new_idx
    ) THEN
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_new_idx);
      RAISE NOTICE 'SPEC-034 M071: Cleaned up partial v2 index % for table %', v_new_idx, v_tbl;
    END IF;

    EXECUTE format(
      'CREATE INDEX %I ON public.%I '
      'USING hnsw (embedding %s) '
      'WITH (m = 16, ef_construction = 32)',
      v_new_idx, v_tbl, v_opclass
    );
    RAISE NOTICE 'SPEC-034 M071: Created % (ef_construction=32, %) for table %', v_new_idx, v_opclass, v_tbl;

    IF EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = 'public' AND indexname = v_old_idx
    ) THEN
      EXECUTE format('DROP INDEX IF EXISTS public.%I', v_old_idx);
      RAISE NOTICE 'SPEC-034 M071: Dropped old index %', v_old_idx;
    END IF;

    EXECUTE format('ALTER INDEX public.%I RENAME TO %I', v_new_idx, v_old_idx);
    RAISE NOTICE 'SPEC-034 M071: Renamed % → %', v_new_idx, v_old_idx;

  END LOOP;

  RAISE NOTICE 'SPEC-034 M071: HNSW optimization complete';
END $$;
