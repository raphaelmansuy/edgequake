-- SPEC-044 — Cypher parameter contract (triple-track: PG16/17/18 + AGE 1.6.0/1.7.0)
--
-- Official contract:
--   https://age.apache.org/age-manual/master/advanced/prepared_statements.html
--   https://github.com/apache/age/issues/315
--
-- Usage (inside EdgeQuake postgres image container):
--   psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 \
--     -v expected_pg_major=16 -v expected_age_min=1.6.0 \
--     -f specs/044-upgrate-issue-study/e2e/sql/cypher_param_contract.sql
--
-- Variables (substituted by run_triple_track_cypher_proof.sh via sed):
--   @expected_pg_major@  — 16 | 17 | 18
--   @expected_age_min@   — 1.6.0 | 1.7.0

\echo '=== BT-044-TT-01/02/03: Version gates ==='

DO $$
DECLARE
  v_major int;
  v_age text;
  v_vector text;
  v_expected_major int := @expected_pg_major@;
  v_expected_age text := '@expected_age_min@';
BEGIN
  SELECT current_setting('server_version_num')::int / 10000 INTO v_major;
  SELECT extversion INTO v_age FROM pg_extension WHERE extname = 'age';
  SELECT extversion INTO v_vector FROM pg_extension WHERE extname = 'vector';

  IF v_major <> v_expected_major THEN
    RAISE EXCEPTION 'BT-044-TT-01 FAIL: PG major % <> expected %', v_major, v_expected_major;
  END IF;
  RAISE NOTICE 'BT-044-TT-01 PASS: PostgreSQL major %', v_major;

  IF v_age IS NULL OR string_to_array(v_age, '.')::int[] < string_to_array(v_expected_age, '.')::int[] THEN
    RAISE EXCEPTION 'BT-044-TT-02 FAIL: age >= % required (got %)', v_expected_age, v_age;
  END IF;
  RAISE NOTICE 'BT-044-TT-02 PASS: age extversion %', v_age;

  IF v_vector IS NULL OR string_to_array(v_vector, '.')::int[] < string_to_array('0.8.3', '.')::int[] THEN
    RAISE EXCEPTION 'BT-044-TT-03 FAIL: vector >= 0.8.3 required (got %)', v_vector;
  END IF;
  RAISE NOTICE 'BT-044-TT-03 PASS: vector extversion %', v_vector;
END $$;

\echo '=== BT-044-TT-12: Tier-specific PG feature gate ==='

DO $$
DECLARE
  v_major int;
BEGIN
  SELECT current_setting('server_version_num')::int / 10000 INTO v_major;
  IF v_major = 18 THEN
    PERFORM uuidv7();
    RAISE NOTICE 'BT-044-TT-12 PASS: uuidv7() present on PG18';
  ELSE
    BEGIN
      PERFORM uuidv7();
      RAISE EXCEPTION 'BT-044-TT-12 FAIL: uuidv7() should not exist on PG%', v_major;
    EXCEPTION WHEN undefined_function THEN
      RAISE NOTICE 'BT-044-TT-12 PASS: uuidv7() absent on PG% (expected)', v_major;
    END;
  END IF;
END $$;

\echo '=== Setup graph for Cypher param probes ==='

LOAD 'age';
SET search_path = ag_catalog, "$user", public;

SELECT create_graph('spec044_cypher_contract');
SELECT create_vlabel('spec044_cypher_contract', 'Node');
SELECT create_elabel('spec044_cypher_contract', 'EDGE');

SELECT * FROM cypher('spec044_cypher_contract', $$
  CREATE (a:Node {node_id: 'SPEC044_A', entity_type: 'PROBE'})
  CREATE (b:Node {node_id: 'SPEC044_B', entity_type: 'PROBE'})
  CREATE (a)-[:EDGE {source_id: 'SPEC044_A', target_id: 'SPEC044_B'}]->(b)
$$) AS (r agtype);

\echo '=== BT-044-TT-04: Negative — inline agtype literal MUST fail ==='

DO $bt044_tt04$
BEGIN
  PERFORM * FROM cypher('spec044_cypher_contract', $cypher$
    MATCH (n:Node {node_id: $node_id}) RETURN n LIMIT 1
  $cypher$, '{"node_id":"SPEC044_A"}'::agtype) AS (n agtype);
  RAISE EXCEPTION 'BT-044-TT-04 FAIL: inline agtype literal should be rejected by AGE';
EXCEPTION WHEN OTHERS THEN
  IF SQLERRM LIKE '%must be a parameter%' THEN
    RAISE NOTICE 'BT-044-TT-04 PASS: AGE rejects inline agtype third arg (%)' , SQLERRM;
  ELSE
    RAISE EXCEPTION 'BT-044-TT-04 FAIL: unexpected error: %', SQLERRM;
  END IF;
END $bt044_tt04$;

\echo '=== BT-044-TT-05/06/07: Positive — PREPARE/EXECUTE with bare $1 (AGE manual) ==='

-- Node read (BT-044-TT-06)
PREPARE spec044_read_node(agtype) AS
SELECT * FROM cypher('spec044_cypher_contract', $$
  MATCH (n:Node {node_id: $node_id}) RETURN n
$$, $1) AS (n agtype);

EXECUTE spec044_read_node('{"node_id":"SPEC044_A"}');
\echo 'BT-044-TT-06 PASS: prepared read node'

-- Node delete (BT-044-TT-05) — recreate after delete for edge test
PREPARE spec044_delete_node(agtype) AS
SELECT * FROM cypher('spec044_cypher_contract', $$
  MATCH (n:Node {node_id: $node_id}) DETACH DELETE n
$$, $1) AS (a agtype);

-- Edge delete (BT-044-TT-07)
PREPARE spec044_delete_edge(agtype) AS
SELECT * FROM cypher('spec044_cypher_contract', $$
  MATCH (a:Node {node_id: $source_id})-[r:EDGE]->(b:Node {node_id: $target_id})
  DELETE r
$$, $1) AS (a agtype);

EXECUTE spec044_delete_edge('{"source_id":"SPEC044_A","target_id":"SPEC044_B"}');
\echo 'BT-044-TT-07 PASS: prepared delete edge'

EXECUTE spec044_delete_node('{"node_id":"SPEC044_A"}');
EXECUTE spec044_delete_node('{"node_id":"SPEC044_B"}');
\echo 'BT-044-TT-05 PASS: prepared delete node'

DEALLOCATE spec044_read_node;
DEALLOCATE spec044_delete_node;
DEALLOCATE spec044_delete_edge;

SELECT drop_graph('spec044_cypher_contract', true);

\echo '=== BT-044-TT-04/05/06/07 COMPLETE ==='
