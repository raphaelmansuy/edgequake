-- SPEC-041 G3 setup — Create isolated test graph and force M078 CREATE path
\set ON_ERROR_STOP on

\echo '== SPEC-041 G3 setup: isolated AGE graph =='

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE EXCEPTION 'SPEC-041 G3 SKIP: AGE extension not installed';
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM ag_catalog.ag_graph WHERE name = 'spec041_m078_test') THEN
    PERFORM ag_catalog.create_graph('spec041_m078_test');
    RAISE NOTICE 'Created graph spec041_m078_test';
  ELSE
    RAISE NOTICE 'Graph spec041_m078_test already exists';
  END IF;
END $$;

LOAD 'age';
SET search_path = ag_catalog, "$user", public;

SELECT * FROM cypher('spec041_m078_test', $$
  CREATE (n:Node {node_id: '_spec041_test', workspace_id: '00000000-0000-0000-0000-000000000001', tenant_id: '00000000-0000-0000-0000-000000000002'})
$$) AS (n agtype);

DROP INDEX IF EXISTS spec041_m078_test.idx_node_workspace_id;
DROP INDEX IF EXISTS spec041_m078_test.idx_node_tenant_id;
DROP INDEX IF EXISTS spec041_m078_test.idx_edge_start_id_text;
DROP INDEX IF EXISTS spec041_m078_test.idx_edge_end_id_text;

\echo 'G3 setup complete — ready for M078 apply'
