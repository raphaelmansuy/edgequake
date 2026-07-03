-- ============================================================================
-- Migration 079: AGE child Node index reconcile (SPEC-041 #273)
-- Version: 1.0.0 — 2026-07-03
--
-- PURPOSE:
--   Safety net for ALL upgrade paths where migration 078 did not create child
--   "Node" / "EDGE" indexes (v0.13.2 skip-path, graphs added after M078, etc.).
--   Logic SSOT: migrations/support/078/apply.sql (keep in sync).
--
--   Idempotent: IF NOT EXISTS on every index; AGE absent → no-op.
-- ============================================================================

DO $$
DECLARE
  v_graph text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'SPEC-041 M079: AGE not installed — skipping child index reconcile';
    RETURN;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'SPEC-041 M079: ag_catalog missing — skipping';
    RETURN;
  END IF;

  FOR v_graph IN
    SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    IF to_regclass(format('%I."Node"', v_graph)) IS NULL THEN
      RAISE NOTICE 'SPEC-041 M079: No Node table in graph % — skipping', v_graph;
      CONTINUE;
    END IF;

    IF NOT EXISTS (
      SELECT 1 FROM pg_indexes
      WHERE schemaname = v_graph AND indexname = 'idx_node_workspace_id'
    ) THEN
      EXECUTE format(
        'CREATE INDEX idx_node_workspace_id ON %I."Node"
         ((ag_catalog.agtype_to_json(properties)->>''workspace_id''))',
        v_graph
      );
      RAISE NOTICE 'SPEC-041 M079: Created idx_node_workspace_id on %."Node"', v_graph;
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
      RAISE NOTICE 'SPEC-041 M079: Created idx_node_tenant_id on %."Node"', v_graph;
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
        RAISE NOTICE 'SPEC-041 M079: Created idx_edge_start_id_text on %."EDGE"', v_graph;
      END IF;

      IF NOT EXISTS (
        SELECT 1 FROM pg_indexes
        WHERE schemaname = v_graph AND indexname = 'idx_edge_end_id_text'
      ) THEN
        EXECUTE format(
          'CREATE INDEX idx_edge_end_id_text ON %I."EDGE" ((end_id::text))',
          v_graph
        );
        RAISE NOTICE 'SPEC-041 M079: Created idx_edge_end_id_text on %."EDGE"', v_graph;
      END IF;

      EXECUTE format('ANALYZE %I."EDGE"', v_graph);
    END IF;

    EXECUTE format('ANALYZE %I."Node"', v_graph);
    RAISE NOTICE 'SPEC-041 M079: Reconciled child indexes + ANALYZE for graph %', v_graph;
  END LOOP;

  RAISE NOTICE 'SPEC-041 M079: Child node index reconcile complete';
END $$;
