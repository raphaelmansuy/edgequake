-- ============================================================================
-- Migration 070: AGE graph index consolidation (SPEC-034 IMP-02)
-- Version: 2.0.0 — 2026-06-30
--
-- PURPOSE:
--   Remove redundant and unused indexes from AGE graph tables, reducing
--   write amplification from 17+ index maintenance ops per node INSERT
--   down to 5-6 essential indexes.
--
-- TRANSACTION SAFETY:
--   * Regular DROP INDEX (no CONCURRENTLY) — works inside sqlx transaction.
--   * IDEMPOTENT: helper checks pg_indexes before each DROP.
--   * AGE not installed → graceful no-op.
--
-- INDEXES TO DROP (confirmed by SPEC-034 code audit + pg_stat evidence):
--
--   _ag_label_vertex parent-table indexes (ALL) — 0 rows in parent table,
--     all data is in the "Node" label child table (AGE inheritance pattern).
--     Parent-table indexes are never scanned.
--
--   _ag_label_edge parent-table indexes (ALL) — same reason as above.
--
--   "EDGE" child-table indexes with confirmed 0 scans:
--     idx_edge_props_gin         (17 MB GIN — never used by any code path)
--     idx_edge_start_end         (4.9 MB composite btree — superseded)
--     idx_edge_source_target_btr (4.8 MB btree — 0 scans, no code uses it)
--
--   idx_node_prop_node_id (agtype_access_operator form on Node):
--     Superseded by idx_node_prop_node_id_btree (agtype_to_json form).
--     ONLY dropped when the btree variant is confirmed present.
--
-- INDEXES TO KEEP (code-path confirmed):
--   "Node": idx_node_id, idx_node_props_gin, idx_node_prop_node_id_btree,
--           idx_node_workspace_id, idx_node_tenant_id
--   "EDGE": idx_edge_start_id, idx_edge_end_id, idx_edge_source_id,
--           idx_edge_target_id
--
-- ROLLBACK:
--   Re-create removed indexes with CREATE INDEX.
--   Safe because no data was deleted — only indexes.
-- ============================================================================

-- ============================================================================
-- Helper: drop a single index from a graph schema if it exists.
-- Idempotent — safe to call even when the index doesn't exist.
-- ============================================================================
CREATE OR REPLACE FUNCTION eq_drop_graph_index_if_exists(
  p_schema text,
  p_index  text
) RETURNS void AS $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM pg_indexes
    WHERE  schemaname = p_schema AND indexname = p_index
  ) THEN
    -- Regular DROP INDEX (transaction-safe; brief lock on the index catalog).
    EXECUTE format('DROP INDEX IF EXISTS %I.%I', p_schema, p_index);
    RAISE NOTICE 'SPEC-034 M070: Dropped %.%', p_schema, p_index;
  END IF;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Main: iterate over all AGE graphs and remove unused indexes.
-- ============================================================================
DO $$
DECLARE
  v_graph   text;
  v_prefix  text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'SPEC-034 M070: AGE not installed — skipping index consolidation';
    RETURN;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'SPEC-034 M070: ag_catalog missing — skipping';
    RETURN;
  END IF;

  FOR v_graph IN
    SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    v_prefix := replace(v_graph, '.', '_');

    RAISE NOTICE 'SPEC-034 M070: Consolidating graph: %', v_graph;

    -- -----------------------------------------------------------------------
    -- Drop _ag_label_vertex parent-table indexes (0 rows — never scanned).
    -- -----------------------------------------------------------------------
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_node_id');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_tenant_id');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_workspace_id');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_tenant_workspace');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_entity_type');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_vertex_source_id');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_vertex_source_ids_gin');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_ag_vertex_props_gin');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_ag_vertex_tenant_id');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_ag_vertex_workspace_id');

    -- -----------------------------------------------------------------------
    -- Drop _ag_label_edge parent-table indexes (same reason).
    -- -----------------------------------------------------------------------
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_edge_start_id');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_' || v_prefix || '_edge_end_id');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_ag_edge_props_gin');

    -- -----------------------------------------------------------------------
    -- Drop unused "EDGE" child-table indexes (confirmed 0 scans in SPEC-034).
    -- -----------------------------------------------------------------------
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_edge_props_gin');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_edge_start_end');
    PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_edge_source_target_btr');

    -- -----------------------------------------------------------------------
    -- Drop duplicate node_id agtype_access_operator form on "Node".
    -- Safety: only drop if the btree variant (agtype_to_json form) exists,
    -- because pg_get_nodes_batch and BFS joins rely on the btree variant.
    -- -----------------------------------------------------------------------
    IF EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE  schemaname = v_graph AND indexname = 'idx_node_prop_node_id_btree'
    ) THEN
      PERFORM eq_drop_graph_index_if_exists(v_graph, 'idx_node_prop_node_id');
    ELSE
      RAISE NOTICE 'SPEC-034 M070: Skipping idx_node_prop_node_id drop — btree variant absent in %', v_graph;
    END IF;

  END LOOP;

  RAISE NOTICE 'SPEC-034 M070: Index consolidation complete';
END $$;
