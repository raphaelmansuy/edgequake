-- ============================================================================
-- Migration 075: Fix native graph write helper functions (SPEC-034 IMP-01)
-- Version: 1.0.0 — 2026-06-30
--
-- PURPOSE:
--   Fix the helper functions from migration 067 that have two bugs:
--
-- BUG 1 — Wrong join column (ag_graph.oid vs ag_graph.graphid):
--   Migration 067 used `JOIN ag_catalog.ag_graph g ON l.graph = g.oid`.
--   The correct column is `g.graphid` (AGE's ag_graph PK is named `graphid`,
--   not `oid`). Using `g.oid` would give an error or wrong result.
--
-- BUG 2 — Wrong graphid bit shift (<<32 vs <<48):
--   Migration 067 used `(label_id << 32) | seq_val`.
--   ACTUAL encoding (verified from live data): `(label_id << 48) | seq_val`.
--   Evidence: graphid 844424930151567 = (3 << 48) | 19599 where 3 = Node id.
--
-- BUG 3 — Hardcoded sequence name vs ag_label.seq_name:
--   Migration 067 constructed the seq as `p_label || '_id_seq'`.
--   The correct seq_name is stored in `ag_catalog.ag_label.seq_name`.
--   Both happen to be the same format, but reading from the catalog is robust.
--
-- BUG 4 — Cast chain: ::jsonb::agtype does not exist in AGE.
--   The cast chain `text::jsonb` works (std PostgreSQL), but `jsonb::agtype`
--   is NOT registered. The correct cast is `text::ag_catalog.agtype` (via
--   agtype's input function `agtype_in`, which accepts text in agtype format).
--   This is fixed in the Rust native upsert SQL (nodes_ops.rs, edges_ops.rs)
--   alongside this migration; Migration 075 just documents the co-fix.
--
-- TRANSACTION SAFETY: No CONCURRENTLY — fully transaction-safe.
-- IDEMPOTENT: Uses CREATE OR REPLACE inside a guarded DO block.
-- ============================================================================

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'SPEC-034 M075: AGE not installed — skipping'; RETURN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'SPEC-034 M075: ag_catalog missing — skipping'; RETURN;
  END IF;

  -- =========================================================================
  -- FIX 1: eq_get_label_oid — use g.graphid (not g.oid), return l.id (not l.oid)
  -- =========================================================================
  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_get_label_oid(p_graph text, p_label text)
    RETURNS bigint AS $b$
    DECLARE v_id bigint;
    BEGIN
      -- WHY g.graphid: AGE's ag_graph PK is named 'graphid', not 'oid'.
      -- WHY l.id: The label id (small integer 1..N) is the field used in the
      --           high bits of the graphid (verified: 3 for Node, 4 for EDGE).
      SELECT l.id INTO v_id
      FROM ag_catalog.ag_label l
      JOIN ag_catalog.ag_graph g ON l.graph = g.graphid
      WHERE g.name = p_graph AND l.name = p_label;
      RETURN v_id;
    EXCEPTION WHEN others THEN RETURN NULL;
    END;
    $b$ LANGUAGE plpgsql STABLE;
  $f$;

  -- =========================================================================
  -- FIX 2+3: eq_next_graphid — use <<48 and read seq_name from catalog
  -- =========================================================================
  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_next_graphid(p_graph text, p_label text)
    RETURNS ag_catalog.graphid AS $b$
    DECLARE
      v_id      bigint;
      v_seqname text;
      v_seq     bigint;
    BEGIN
      -- Read label id AND sequence name from AGE catalog.
      -- WHY g.graphid: see eq_get_label_oid above.
      -- WHY l.seq_name: the catalog stores the correct seq name per label.
      SELECT l.id, l.seq_name INTO v_id, v_seqname
      FROM ag_catalog.ag_label l
      JOIN ag_catalog.ag_graph g ON l.graph = g.graphid
      WHERE g.name = p_graph AND l.name = p_label;

      IF v_id IS NULL THEN
        RAISE EXCEPTION 'SPEC-034: AGE label % not found in graph %', p_label, p_graph;
      END IF;

      -- Advance the AGE-managed sequence for this label.
      EXECUTE format('SELECT nextval(%L)', format('%I.%I', p_graph, v_seqname))
        INTO v_seq;

      -- WHY <<48: Verified from live graphids (e.g., 844424930151567):
      --   upper bits 844424930151567 >> 48 = 3 (Node label id) ✓
      --   lower bits 844424930151567 & (2^48-1) = 19599 (seq value) ✓
      --   Formula: (label_id << 48) | seq_val  (NOT label_id << 32)
      RETURN ((v_id << 48) | v_seq)::ag_catalog.graphid;
    END;
    $b$ LANGUAGE plpgsql;
  $f$;

  -- eq_next_node_id / eq_next_edge_id: no body changes (just call eq_next_graphid)
  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_next_node_id(p_graph text)
    RETURNS ag_catalog.graphid AS $b$
    BEGIN RETURN eq_next_graphid(p_graph, 'Node'); END;
    $b$ LANGUAGE plpgsql;
  $f$;

  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_next_edge_id(p_graph text)
    RETURNS ag_catalog.graphid AS $b$
    BEGIN RETURN eq_next_graphid(p_graph, 'EDGE'); END;
    $b$ LANGUAGE plpgsql;
  $f$;

  RAISE NOTICE 'SPEC-034 M075: Helper functions corrected (<<48 shift, graphid join, seq_name from catalog)';
END $$;
