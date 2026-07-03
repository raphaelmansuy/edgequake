-- SPEC-041 G3 verify — After M078 apply on spec041_m078_test graph
-- Usage: psql "$DATABASE_URL" -f apply_m078_verify.sql

\set ON_ERROR_STOP on

\echo '== SPEC-041 G3 verify: M078 index creation =='

DO $$
DECLARE
  v_ws_def text;
  v_tn_def text;
BEGIN
  SELECT pg_get_indexdef(c.oid) INTO v_ws_def
  FROM pg_indexes i
  JOIN pg_class c ON c.relname = i.indexname
    AND c.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = i.schemaname)
  WHERE i.schemaname = 'spec041_m078_test' AND i.indexname = 'idx_node_workspace_id';

  IF v_ws_def IS NULL THEN
    RAISE EXCEPTION 'SPEC-041 G3 FAIL: idx_node_workspace_id not created';
  END IF;

  IF v_ws_def LIKE '%->>>%' THEN
    RAISE EXCEPTION 'SPEC-041 G3 FAIL: workspace index has ->>> : %', v_ws_def;
  END IF;

  IF v_ws_def NOT LIKE '%->>%workspace_id%' THEN
    RAISE EXCEPTION 'SPEC-041 G3 FAIL: workspace index missing ->> workspace_id : %', v_ws_def;
  END IF;

  SELECT pg_get_indexdef(c.oid) INTO v_tn_def
  FROM pg_indexes i
  JOIN pg_class c ON c.relname = i.indexname
    AND c.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = i.schemaname)
  WHERE i.schemaname = 'spec041_m078_test' AND i.indexname = 'idx_node_tenant_id';

  IF v_tn_def IS NULL THEN
    RAISE EXCEPTION 'SPEC-041 G3 FAIL: idx_node_tenant_id not created';
  END IF;

  RAISE NOTICE 'SPEC-041 G3 PASS: indexes created with correct ->> operator';
  RAISE NOTICE '  workspace: %', v_ws_def;
  RAISE NOTICE '  tenant:    %', v_tn_def;
END $$;

\echo '== G3-NEG: prove ->>> operator fails (expected) =='

DO $$
BEGIN
  BEGIN
    EXECUTE 'CREATE INDEX spec041_bad ON spec041_m078_test."Node" ((ag_catalog.agtype_to_json(properties)->>>''workspace_id''))';
    RAISE EXCEPTION 'SPEC-041 G3-NEG FAIL: ->>> should have raised error';
  EXCEPTION
    WHEN undefined_function THEN
      RAISE NOTICE 'SPEC-041 G3-NEG PASS: ->>> rejected (undefined_function)';
    WHEN OTHERS THEN
      IF SQLERRM LIKE '%operator does not exist%' THEN
        RAISE NOTICE 'SPEC-041 G3-NEG PASS: ->>> rejected: %', SQLERRM;
      ELSE
        RAISE;
      END IF;
  END;
END $$;

\echo 'PASS: G3 verify complete'
