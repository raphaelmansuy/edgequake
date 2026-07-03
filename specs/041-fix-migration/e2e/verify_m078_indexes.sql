-- SPEC-041 G2 — Verify M078 index expressions use ->> not ->>>
-- Usage: psql "$DATABASE_URL" -v test_graph=eq_eq_default_graph -f verify_m078_indexes.sql

\set ON_ERROR_STOP on

\if :{?test_graph}
\else
\set test_graph eq_eq_default_graph
\endif

\echo '== SPEC-041 G2: M078 index definition audit =='
\echo 'Graph:' :test_graph

SELECT CASE
  WHEN to_regclass(format('%I."Node"', :'test_graph')) IS NULL THEN 'SKIP_NO_NODE'
  ELSE 'HAS_NODE'
END AS node_table_status;

SELECT
  i.indexname,
  CASE
    WHEN pg_get_indexdef(c.oid) LIKE '%->>>%' THEN 'FAIL_TRIPLE_GT'
    WHEN pg_get_indexdef(c.oid) LIKE '%->> %workspace_id%' OR pg_get_indexdef(c.oid) LIKE '%->> ''workspace_id''%' THEN 'OK_WORKSPACE'
    WHEN i.indexname = 'idx_node_workspace_id' THEN 'MISSING_OPERATOR'
    ELSE 'OTHER'
  END AS workspace_check,
  CASE
    WHEN pg_get_indexdef(c.oid) LIKE '%->>>%' THEN 'FAIL_TRIPLE_GT'
    WHEN pg_get_indexdef(c.oid) LIKE '%->> %tenant_id%' OR pg_get_indexdef(c.oid) LIKE '%->> ''tenant_id''%' THEN 'OK_TENANT'
    WHEN i.indexname = 'idx_node_tenant_id' THEN 'MISSING_OPERATOR'
    ELSE 'OTHER'
  END AS tenant_check,
  pg_get_indexdef(c.oid) AS indexdef
FROM pg_indexes i
JOIN pg_class c ON c.relname = i.indexname
  AND c.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = i.schemaname)
WHERE i.schemaname = :'test_graph'
  AND i.indexname IN ('idx_node_workspace_id', 'idx_node_tenant_id', 'idx_edge_start_id_text', 'idx_edge_end_id_text')
ORDER BY i.indexname;

DO $$
DECLARE
  v_bad int;
  v_graph text := current_setting('spec041.test_graph', true);
BEGIN
  IF v_graph IS NULL OR v_graph = '' THEN
    v_graph := 'eq_eq_default_graph';
  END IF;

  SELECT COUNT(*) INTO v_bad
  FROM pg_indexes i
  JOIN pg_class c ON c.relname = i.indexname
    AND c.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = i.schemaname)
  WHERE i.schemaname = v_graph
    AND i.indexname IN ('idx_node_workspace_id', 'idx_node_tenant_id')
    AND pg_get_indexdef(c.oid) LIKE '%->>>%';

  IF v_bad > 0 THEN
    RAISE EXCEPTION 'SPEC-041 G2 FAIL: % index(es) contain invalid ->>> operator', v_bad;
  END IF;
END $$;

\echo 'PASS: G2 index definition audit complete'
