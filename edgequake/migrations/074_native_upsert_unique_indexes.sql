-- ============================================================================
-- Migration 074: Unique indexes for IMP-01 native SQL write path (SPEC-034)
-- Version: 4.0.0 — 2026-06-30
--
-- PURPOSE:
--   Add UNIQUE expression indexes required for the native SQL write path
--   (INSERT ... ON CONFLICT DO UPDATE). PostgreSQL ON CONFLICT requires a
--   UNIQUE index on the exact conflict target expression.
--
-- INDEXES ADDED (per AGE graph):
--
--   idx_node_prop_node_id_unique   ON "Node"
--     Expression: (ag_catalog.agtype_to_json(properties)->>'node_id')
--     WHY: pg_upsert_nodes_batch_native ON CONFLICT target.
--          Replaces the non-unique idx_node_prop_node_id_btree.
--
--   idx_edge_source_target_unique  ON "EDGE"
--     Expressions: (source_id), (target_id) from agtype properties
--     WHY: pg_upsert_edges_batch_native ON CONFLICT target.
--
-- TRANSACTION SAFETY:
--   * No CONCURRENTLY — fully transaction-safe.
--   * IDEMPOTENT: IF NOT EXISTS / IF EXISTS guards on all DDL.
--
-- EDGE CASES (verified against live database):
--   * ag_catalog.graphid has no < > operators → use ctid for dedup comparison.
--     max(ctid) per group = the last-written row → KEEP that one.
--   * chr(0) (null byte) not allowed in PostgreSQL text → avoid string concat
--     for dedup count; use GROUP BY / HAVING count(*) > 1 instead.
--   * Operator precedence: ->> binds looser than || in some contexts.
--     Use explicit parentheses around all ->>'key' extractions to be safe.
--   * Node/EDGE tables may not exist yet (AGE lazy creation) → skip gracefully.
--   * Duplicate rows ARE present in live data (49 nodes, 118 edges found).
--     The dedup step is essential, not just a safety net.
-- ============================================================================

DO $$
DECLARE
  v_graph      text;
  v_dup_count  bigint;
  v_node_uniq  text := 'idx_node_prop_node_id_unique';
  v_old_btree  text := 'idx_node_prop_node_id_btree';
  v_edge_uniq  text := 'idx_edge_source_target_unique';
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'SPEC-034 M074: AGE not installed — skipping'; RETURN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'SPEC-034 M074: ag_catalog missing — skipping'; RETURN;
  END IF;

  FOR v_graph IN
    SELECT name FROM ag_catalog.ag_graph ORDER BY name
  LOOP
    RAISE NOTICE 'SPEC-034 M074: Processing graph: %', v_graph;

    -- =======================================================================
    -- PART A: UNIQUE index on Node(node_id)
    -- =======================================================================
    IF NOT EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = v_graph AND tablename = 'Node') THEN
      RAISE NOTICE 'SPEC-034 M074: No Node table in % yet — skipping', v_graph;
    ELSE
      -- Count duplicate node_ids using GROUP BY / HAVING.
      -- WHY: Avoids string concatenation (chr(0) not permitted in pg text).
      --      Returns number of node_id VALUES that have more than one row.
      EXECUTE format(
        'SELECT count(*) FROM ('
        '  SELECT 1 FROM %I."Node"'
        '  GROUP BY ag_catalog.agtype_to_json(properties)->>''node_id'''
        '  HAVING count(*) > 1'
        ') t',
        v_graph
      ) INTO v_dup_count;

      IF v_dup_count > 0 THEN
        RAISE WARNING 'SPEC-034 M074: % duplicate node_id groups in %."Node" — deduplicating (keep max ctid)',
                      v_dup_count, v_graph;
        -- Delete all but the most recently written row per node_id.
        -- WHY ctid: ag_catalog.graphid has no comparison operators (<, >, <=).
        --   max(ctid) identifies the physical location of the last-written row.
        --   This is stable during the migration (no concurrent writes).
        EXECUTE format(
          'DELETE FROM %I."Node"'
          ' WHERE ctid NOT IN ('
          '   SELECT max(ctid) FROM %I."Node"'
          '   GROUP BY ag_catalog.agtype_to_json(properties)->>''node_id'''
          ' )',
          v_graph, v_graph
        );
        RAISE NOTICE 'SPEC-034 M074: Node dedup complete in % (kept newest per node_id)', v_graph;
      ELSE
        RAISE NOTICE 'SPEC-034 M074: No duplicate node_ids in % — proceeding', v_graph;
      END IF;

      -- Create UNIQUE index (replace old non-unique btree).
      IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname = v_graph AND indexname = v_node_uniq) THEN
        IF EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname = v_graph AND indexname = v_old_btree) THEN
          EXECUTE format('DROP INDEX IF EXISTS %I.%I', v_graph, v_old_btree);
          RAISE NOTICE 'SPEC-034 M074: Dropped non-unique % in %', v_old_btree, v_graph;
        END IF;
        -- WHY the same expression: the UNIQUE index serves both ON CONFLICT
        -- inference and the existing read path (btree lookups by node_id).
        EXECUTE format(
          'CREATE UNIQUE INDEX %I ON %I."Node"'
          ' ((ag_catalog.agtype_to_json(properties)->>''node_id''))',
          v_node_uniq, v_graph
        );
        RAISE NOTICE 'SPEC-034 M074: Created UNIQUE Node index % in %', v_node_uniq, v_graph;
      ELSE
        RAISE NOTICE 'SPEC-034 M074: % already present in % — skipping', v_node_uniq, v_graph;
      END IF;
    END IF;

    -- =======================================================================
    -- PART B: UNIQUE composite index on EDGE(source_id, target_id)
    -- =======================================================================
    IF NOT EXISTS (SELECT 1 FROM pg_tables WHERE schemaname = v_graph AND tablename = 'EDGE') THEN
      RAISE NOTICE 'SPEC-034 M074: No EDGE table in % yet — skipping', v_graph;
    ELSE
      -- Count duplicate (source_id, target_id) pairs using GROUP BY / HAVING.
      -- WHY explicit parens on (properties)->>'key': prevents operator-precedence
      --   ambiguity when multiple ->>'key' appear in the same expression list.
      EXECUTE format(
        'SELECT count(*) FROM ('
        '  SELECT 1 FROM %I."EDGE"'
        '  GROUP BY (ag_catalog.agtype_to_json(properties)->>''source_id''),'
        '           (ag_catalog.agtype_to_json(properties)->>''target_id'')'
        '  HAVING count(*) > 1'
        ') t',
        v_graph
      ) INTO v_dup_count;

      IF v_dup_count > 0 THEN
        RAISE WARNING 'SPEC-034 M074: % duplicate (source_id,target_id) groups in %."EDGE" — deduplicating',
                      v_dup_count, v_graph;
        EXECUTE format(
          'DELETE FROM %I."EDGE"'
          ' WHERE ctid NOT IN ('
          '   SELECT max(ctid) FROM %I."EDGE"'
          '   GROUP BY (ag_catalog.agtype_to_json(properties)->>''source_id''),'
          '            (ag_catalog.agtype_to_json(properties)->>''target_id'')'
          ' )',
          v_graph, v_graph
        );
        RAISE NOTICE 'SPEC-034 M074: EDGE dedup complete in % (kept newest per source/target)', v_graph;
      ELSE
        RAISE NOTICE 'SPEC-034 M074: No duplicate (source_id,target_id) in % — proceeding', v_graph;
      END IF;

      IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE schemaname = v_graph AND indexname = v_edge_uniq) THEN
        EXECUTE format(
          'CREATE UNIQUE INDEX %I ON %I."EDGE" ('
          '  (ag_catalog.agtype_to_json(properties)->>''source_id''),'
          '  (ag_catalog.agtype_to_json(properties)->>''target_id'')'
          ')',
          v_edge_uniq, v_graph
        );
        RAISE NOTICE 'SPEC-034 M074: Created UNIQUE EDGE index % in %', v_edge_uniq, v_graph;
      ELSE
        RAISE NOTICE 'SPEC-034 M074: % already present in % — skipping', v_edge_uniq, v_graph;
      END IF;
    END IF;

  END LOOP;
  RAISE NOTICE 'SPEC-034 M074: COMPLETE';
END $$;
