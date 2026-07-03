-- ============================================================================
-- Migration 078: AGE child-table workspace indexes + ANALYZE (SPEC-040 #262)
-- Version: 1.0.1 — 2026-07-03 (SPEC-041 #273: fix invalid JSON text-extraction operator typo)
--
-- PURPOSE:
--   Repair legacy installs where migration 014 created indexes on inheritance
--   parent tables (_ag_label_vertex / _ag_label_edge) while all rows live in
--   child label tables ("Node" / "EDGE"). Ensures workspace filter predicates
--   and start_id::text joins use expression indexes on the scanned rels.
--
--   Complements graph_lifecycle.rs ensure_indexes() and migration 072.
--
-- TRANSACTION SAFETY:
--   * Regular CREATE INDEX (no CONCURRENTLY) — works inside sqlx transaction.
--   * IDEMPOTENT: IF NOT EXISTS checks via pg_indexes.
--   * AGE not installed → graceful no-op.
--
-- ROLLBACK:
--   DROP INDEX on child tables only; no data loss.
-- ============================================================================

DO $$
DECLARE
  v_graph text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'SPEC-040 M078: AGE not installed — skipping child workspace stats repair';
    RETURN;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'SPEC-040 M078: ag_catalog missing — skipping';
    RETURN;
  END IF;

  FOR v_graph IN
    SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    IF to_regclass(format('%I."Node"', v_graph)) IS NULL THEN
      RAISE NOTICE 'SPEC-040 M078: No Node table in graph % — skipping', v_graph;
      CONTINUE;
    END IF;

    -- Child "Node" expression indexes (workspace / tenant filters)
    IF NOT EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE schemaname = v_graph AND indexname = 'idx_node_workspace_id'
    ) THEN
      EXECUTE format(
        'CREATE INDEX idx_node_workspace_id ON %I."Node"
         ((ag_catalog.agtype_to_json(properties)->>''workspace_id''))',
        v_graph
      );
      RAISE NOTICE 'SPEC-040 M078: Created idx_node_workspace_id on %."Node"', v_graph;
    END IF;

    IF NOT EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE schemaname = v_graph AND indexname = 'idx_node_tenant_id'
    ) THEN
      EXECUTE format(
        'CREATE INDEX idx_node_tenant_id ON %I."Node"
         ((ag_catalog.agtype_to_json(properties)->>''tenant_id''))',
        v_graph
      );
      RAISE NOTICE 'SPEC-040 M078: Created idx_node_tenant_id on %."Node"', v_graph;
    END IF;

    IF to_regclass(format('%I."EDGE"', v_graph)) IS NOT NULL THEN
      IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = v_graph AND indexname = 'idx_edge_start_id_text'
      ) THEN
        EXECUTE format(
          'CREATE INDEX idx_edge_start_id_text ON %I."EDGE" ((start_id::text))',
          v_graph
        );
        RAISE NOTICE 'SPEC-040 M078: Created idx_edge_start_id_text on %."EDGE"', v_graph;
      END IF;

      IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = v_graph AND indexname = 'idx_edge_end_id_text'
      ) THEN
        EXECUTE format(
          'CREATE INDEX idx_edge_end_id_text ON %I."EDGE" ((end_id::text))',
          v_graph
        );
        RAISE NOTICE 'SPEC-040 M078: Created idx_edge_end_id_text on %."EDGE"', v_graph;
      END IF;

      EXECUTE format('ANALYZE %I."EDGE"', v_graph);
    END IF;

    EXECUTE format('ANALYZE %I."Node"', v_graph);
    RAISE NOTICE 'SPEC-040 M078: Repaired child indexes + ANALYZE for graph %', v_graph;
  END LOOP;

  RAISE NOTICE 'SPEC-040 M078: Child workspace stats index repair complete';
END $$;
