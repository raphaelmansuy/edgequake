-- ============================================================================
-- Migration 067: Native graph write helper functions (SPEC-034 IMP-01)
-- Version: 2.0.0 — 2026-06-30
--
-- PURPOSE:
--   Add PostgreSQL helper functions that generate valid AGE graphids from
--   outside the cypher() UDF. Prerequisite for the native SQL write path
--   (INSERT ... ON CONFLICT DO UPDATE) that replaces Cypher MERGE.
--
-- FUNCTIONS (created only when AGE is installed):
--   eq_get_label_oid(graph_name, label_name) → bigint
--   eq_next_graphid(graph_name, label_name)  → ag_catalog.graphid
--   eq_next_node_id(graph_name)              → ag_catalog.graphid
--   eq_next_edge_id(graph_name)              → ag_catalog.graphid
--
-- TRANSACTION SAFETY:
--   * No CONCURRENTLY operations — fully transaction-safe.
--   * Pure function additions — no data or index changes.
--   * IDEMPOTENT: CREATE OR REPLACE inside EXECUTE.
--   * AGE not installed → graceful no-op.
--
-- WHY EXECUTE (dynamic SQL):
--   The functions return ag_catalog.graphid (an AGE type). Using EXECUTE
--   defers type resolution to runtime, inside the DO block that already
--   confirmed AGE is present. Without EXECUTE, PostgreSQL would fail to
--   parse the CREATE FUNCTION at compile time if ag_catalog doesn't exist.
--
-- ROLLBACK:
--   DROP FUNCTION IF EXISTS eq_next_edge_id(text);
--   DROP FUNCTION IF EXISTS eq_next_node_id(text);
--   DROP FUNCTION IF EXISTS eq_next_graphid(text, text);
--   DROP FUNCTION IF EXISTS eq_get_label_oid(text, text);
-- ============================================================================

DO $$
BEGIN

  -- -------------------------------------------------------------------------
  -- Guard: only proceed when AGE extension is installed.
  -- -------------------------------------------------------------------------
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'SPEC-034 M067: AGE not installed — skipping native graph write helpers';
    RETURN;
  END IF;

  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'SPEC-034 M067: ag_catalog schema missing — skipping';
    RETURN;
  END IF;

  -- -------------------------------------------------------------------------
  -- STEP 1: eq_get_label_oid — resolve AGE label OID from graph+label name.
  -- WHY: AGE graphid = (label_oid << 32) | seq_val.
  --      The OID comes from ag_catalog.ag_label joined to ag_catalog.ag_graph.
  -- -------------------------------------------------------------------------
  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_get_label_oid(
      p_graph text,
      p_label text
    ) RETURNS bigint AS $b$
    DECLARE v_oid bigint;
    BEGIN
      SELECT l.oid::bigint INTO v_oid
      FROM   ag_catalog.ag_label l
      JOIN   ag_catalog.ag_graph g ON l.graph = g.oid
      WHERE  g.name = p_graph AND l.name = p_label;
      RETURN v_oid;
    EXCEPTION WHEN others THEN RETURN NULL;
    END;
    $b$ LANGUAGE plpgsql STABLE;
  $f$;

  -- -------------------------------------------------------------------------
  -- STEP 2: eq_next_graphid — generate the next AGE graphid for any label.
  -- WHY: Enables native SQL INSERT without cypher() UDF.
  --      Reads the AGE sequence directly so IDs are AGE-compatible.
  -- -------------------------------------------------------------------------
  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_next_graphid(
      p_graph text,
      p_label text
    ) RETURNS ag_catalog.graphid AS $b$
    DECLARE
      v_oid  bigint;
      v_seq  bigint;
      v_name text;
    BEGIN
      v_oid := eq_get_label_oid(p_graph, p_label);
      IF v_oid IS NULL THEN
        RAISE EXCEPTION 'SPEC-034: AGE label % not found in graph %', p_label, p_graph;
      END IF;
      v_name := format('%I.%I', p_graph, p_label || '_id_seq');
      EXECUTE format('SELECT nextval(%L)', v_name) INTO v_seq;
      -- AGE graphid encoding: high 32 bits = label OID, low 32 bits = seq
      RETURN ((v_oid << 32) | v_seq)::ag_catalog.graphid;
    END;
    $b$ LANGUAGE plpgsql;
  $f$;

  -- -------------------------------------------------------------------------
  -- STEP 3: eq_next_node_id — shorthand for Node label graphid generation.
  -- -------------------------------------------------------------------------
  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_next_node_id(p_graph text)
    RETURNS ag_catalog.graphid AS $b$
    BEGIN RETURN eq_next_graphid(p_graph, 'Node'); END;
    $b$ LANGUAGE plpgsql;
  $f$;

  -- -------------------------------------------------------------------------
  -- STEP 4: eq_next_edge_id — shorthand for EDGE label graphid generation.
  -- -------------------------------------------------------------------------
  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_next_edge_id(p_graph text)
    RETURNS ag_catalog.graphid AS $b$
    BEGIN RETURN eq_next_graphid(p_graph, 'EDGE'); END;
    $b$ LANGUAGE plpgsql;
  $f$;

  RAISE NOTICE 'SPEC-034 M067: Native graph write helpers created (eq_next_node_id, eq_next_edge_id)';

END $$;
