-- ============================================================================
-- Migration 157: Bound GIN pending lists on AGE lineage indexes
-- Version: 1.0.0 — 2026-09-25
-- SPEC-149 document-scoped lineage (graph page filter / cascade discovery).
--
-- PURPOSE:
--   Lineage discovery probes up to 257 chunk ids per document against
--   idx_{node,edge}_source_ids_gin and idx_{node,edge}_source_chunk_ids_gin.
--   With fastupdate on, every probe also scans the unsorted pending list.
--   Measured on a 121k-node graph after M156: ~23.7k pending tuples per index
--   made each probe ~1.1 ms (discovery ~600 ms); after a flush ~0.001 ms
--   (discovery ~42 ms). Autovacuum does not flush when updates stay under the
--   dead-tuple threshold, and the default 4 MB limit lets the list regrow.
--
-- CHANGE:
--   1. gin_clean_pending_list() on the four lineage GIN indexes (flush now).
--   2. ALTER INDEX ... SET (gin_pending_list_limit = 256) so the list is
--      flushed at 256 kB (~32 pages) instead of 4 MB (~512 pages).
--
-- SAFETY:
--   * No row data changes; only index-internal layout and a reloption.
--   * Idempotent: re-running flushes an already-empty list and re-sets 256.
--   * ALTER INDEX SET takes ACCESS EXCLUSIVE on the index (catalog-only,
--     instant); flush runs first so the lock is held for milliseconds.
--   * AGE absent / index absent → clean skip.
--   * Rollback: ALTER INDEX ... RESET (gin_pending_list_limit).
--
-- SSOT body also at migrations/support/157/apply.sql for ops re-runs.
-- New graphs get the same limit from graph_lifecycle.rs CREATE INDEX DDL.
-- ============================================================================

SET search_path = public;
SET lock_timeout = '30s';

DO $$
DECLARE
  v_graph text;
  v_index text;
  v_flushed bigint;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'M157: AGE not installed — skipping GIN pending-list bound';
    RETURN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'M157: ag_catalog missing — skipping GIN pending-list bound';
    RETURN;
  END IF;

  FOR v_graph IN
    SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    FOREACH v_index IN ARRAY ARRAY[
      'idx_node_source_ids_gin',
      'idx_node_source_chunk_ids_gin',
      'idx_edge_source_ids_gin',
      'idx_edge_source_chunk_ids_gin'
    ]
    LOOP
      IF to_regclass(format('%I.%I', v_graph, v_index)) IS NULL THEN
        CONTINUE;
      END IF;
      EXECUTE format('SELECT gin_clean_pending_list(%L::regclass)', format('%I.%I', v_graph, v_index))
        INTO v_flushed;
      EXECUTE format('ALTER INDEX %I.%I SET (gin_pending_list_limit = 256)', v_graph, v_index);
      RAISE NOTICE 'M157: %.% flushed % pending pages, limit=256kB', v_graph, v_index, v_flushed;
    END LOOP;
  END LOOP;
END
$$;
