-- ============================================================================
-- Migration 072: Edge text-cast expression indexes (SPEC-034 IMP-07)
-- Version: 2.0.0 — 2026-06-30
--
-- PURPOSE:
--   Add expression indexes ON "EDGE" ((start_id::text)) and ((end_id::text))
--   to fix the full sequential scan in pg_get_incident_edges_batch.
--
-- PROBLEM (from SPEC-034 code audit + EXPLAIN ANALYZE evidence):
--   pg_get_incident_edges_batch joins on:
--     JOIN _ag_label_edge e ON e.start_id::text = sv.id::text
--   The existing idx_edge_start_id is on raw graphid (binary type).
--   PostgreSQL cannot use a btree(graphid) index for a ::text cast comparison.
--   Result: Seq Scan on all 67,636 edges per BFS traversal step (~200ms/level).
--   Fix: expression indexes on (start_id::text) match the cast predicate exactly.
--   Expected speedup: ~40× per BFS level (200ms → 5ms).
--
-- TRANSACTION SAFETY:
--   * Regular CREATE INDEX (no CONCURRENTLY) — works inside sqlx transaction.
--   * IDEMPOTENT: IF NOT EXISTS on every CREATE INDEX.
--   * AGE not installed → graceful no-op.
--
-- ROLLBACK:
--   DROP INDEX IF EXISTS <graph>.idx_<prefix>_edge_start_id_text;
--   DROP INDEX IF EXISTS <graph>.idx_<prefix>_edge_end_id_text;
-- ============================================================================

DO $$
DECLARE
  v_graph    text;
  v_prefix   text;
  v_start_i  text;
  v_end_i    text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'SPEC-034 M072: AGE not installed — skipping edge text-cast indexes';
    RETURN;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'SPEC-034 M072: ag_catalog missing — skipping';
    RETURN;
  END IF;

  FOR v_graph IN
    SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    -- Verify the "EDGE" table exists in this graph (AGE creates it lazily)
    IF NOT EXISTS (
      SELECT 1 FROM pg_tables
      WHERE  schemaname = v_graph AND tablename = 'EDGE'
    ) THEN
      RAISE NOTICE 'SPEC-034 M072: No EDGE table in graph % (no edges yet) — skipping', v_graph;
      CONTINUE;
    END IF;

    v_prefix  := replace(v_graph, '.', '_');
    v_start_i := 'idx_' || v_prefix || '_edge_start_id_text';
    v_end_i   := 'idx_' || v_prefix || '_edge_end_id_text';

    -- -----------------------------------------------------------------------
    -- start_id::text expression index
    -- WHY: Matches the exact cast predicate `e.start_id::text = sv.id::text`
    --      used in pg_get_incident_edges_batch BFS join.
    -- -----------------------------------------------------------------------
    IF NOT EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = v_graph AND indexname = v_start_i
    ) THEN
      EXECUTE format(
        'CREATE INDEX %I ON %I."EDGE" ((start_id::text))',
        v_start_i, v_graph
      );
      RAISE NOTICE 'SPEC-034 M072: Created start_id text index % on graph %', v_start_i, v_graph;
    ELSE
      RAISE NOTICE 'SPEC-034 M072: start_id text index % already exists', v_start_i;
    END IF;

    -- -----------------------------------------------------------------------
    -- end_id::text expression index
    -- WHY: Symmetric — also used in BFS for incoming-edge traversal.
    -- -----------------------------------------------------------------------
    IF NOT EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = v_graph AND indexname = v_end_i
    ) THEN
      EXECUTE format(
        'CREATE INDEX %I ON %I."EDGE" ((end_id::text))',
        v_end_i, v_graph
      );
      RAISE NOTICE 'SPEC-034 M072: Created end_id text index % on graph %', v_end_i, v_graph;
    ELSE
      RAISE NOTICE 'SPEC-034 M072: end_id text index % already exists', v_end_i;
    END IF;

  END LOOP;

  RAISE NOTICE 'SPEC-034 M072: Edge text-cast index creation complete';
END $$;
