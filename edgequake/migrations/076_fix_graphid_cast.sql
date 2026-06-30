-- ============================================================================
-- Migration 076: Fix graphid cast in eq_next_graphid (SPEC-034 IMP-01)
-- Version: 1.0.0 — 2026-06-30
--
-- PURPOSE:
--   Fix the final casting bug in eq_next_graphid from Migration 075.
--
-- BUG:
--   Migration 075 used `((v_id << 48) | v_seq)::ag_catalog.graphid`.
--   There is NO registered bigint → graphid cast in AGE.
--   Error: "cannot cast type bigint to graphid"
--
-- FIX:
--   Use `((v_id << 48) | v_seq)::text::ag_catalog.graphid`.
--   The `graphid` type's input function (`graphid_in`) accepts the decimal
--   text representation of the 64-bit integer. Verified: the text cast
--   '844424930151567'::ag_catalog.graphid works correctly.
--
-- Cast chain: bigint → text → graphid_in → graphid  ✓
--
-- TRANSACTION SAFETY: No CONCURRENTLY — fully transaction-safe.
-- IDEMPOTENT: Uses CREATE OR REPLACE.
-- ============================================================================

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'SPEC-034 M076: AGE not installed — skipping'; RETURN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.schemata WHERE schema_name = 'ag_catalog') THEN
    RAISE NOTICE 'SPEC-034 M076: ag_catalog missing — skipping'; RETURN;
  END IF;

  -- Fix eq_next_graphid: use ::text::ag_catalog.graphid cast chain.
  -- All other logic (<<48 shift, g.graphid join, seq_name from catalog)
  -- remains identical to Migration 075.
  EXECUTE $f$
    CREATE OR REPLACE FUNCTION eq_next_graphid(p_graph text, p_label text)
    RETURNS ag_catalog.graphid AS $b$
    DECLARE
      v_id      bigint;
      v_seqname text;
      v_seq     bigint;
    BEGIN
      SELECT l.id, l.seq_name INTO v_id, v_seqname
      FROM ag_catalog.ag_label l
      JOIN ag_catalog.ag_graph g ON l.graph = g.graphid
      WHERE g.name = p_graph AND l.name = p_label;

      IF v_id IS NULL THEN
        RAISE EXCEPTION 'SPEC-034: AGE label % not found in graph %', p_label, p_graph;
      END IF;

      EXECUTE format('SELECT nextval(%L)', format('%I.%I', p_graph, v_seqname))
        INTO v_seq;

      -- WHY ::text::graphid: no registered bigint→graphid cast in AGE.
      -- graphid_in() accepts the decimal text representation of the 64-bit value.
      -- Formula: (label_id << 48) | seq_val — verified from live graphid decoding.
      RETURN (((v_id << 48) | v_seq)::text)::ag_catalog.graphid;
    END;
    $b$ LANGUAGE plpgsql;
  $f$;

  RAISE NOTICE 'SPEC-034 M076: eq_next_graphid fixed (::text::graphid cast)';
END $$;
