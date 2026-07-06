-- SPEC-044: Post-upgrade PostgreSQL + AGE health gates
-- Usage: psql "$DATABASE_URL" -f specs/044-upgrate-issue-study/e2e/sql/post_upgrade_health.sql
-- All checks should return at least one row with status = 'PASS'

\echo '=== SPEC-044 BT-044-27/34: Extension versions ==='
SELECT
  CASE
    WHEN vector_ext.extversion >= '0.8.0' THEN 'PASS'
    ELSE 'FAIL'
  END AS status,
  'pgvector >= 0.8.0' AS check_name,
  vector_ext.extversion AS detail
FROM pg_extension vector_ext
WHERE vector_ext.extname = 'vector'
UNION ALL
SELECT
  CASE
    WHEN age_ext.extversion >= '1.6.0' THEN 'PASS'
    ELSE 'FAIL'
  END,
  'age >= 1.6.0',
  age_ext.extversion
FROM pg_extension age_ext
WHERE age_ext.extname = 'age';

\echo '=== SPEC-044: AGE graph catalog present ==='
SELECT
  CASE WHEN COUNT(*) > 0 THEN 'PASS' ELSE 'FAIL' END AS status,
  'ag_graph has workspace graph' AS check_name,
  string_agg(name, ', ') AS detail
FROM ag_catalog.ag_graph
WHERE name LIKE 'eq_%' OR name LIKE '%_graph%';

\echo '=== SPEC-044 BT-044-27: Node/EDGE label tables exist ==='
SELECT
  CASE WHEN EXISTS (
    SELECT 1 FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname LIKE 'eq_%' AND c.relname = 'Node'
  ) THEN 'PASS' ELSE 'FAIL' END AS status,
  'Node label child table exists' AS check_name,
  COALESCE((
    SELECT n.nspname || '."Node"'
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname LIKE 'eq_%' AND c.relname = 'Node'
    LIMIT 1
  ), 'missing') AS detail;

SELECT
  CASE WHEN EXISTS (
    SELECT 1 FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname LIKE 'eq_%' AND c.relname = 'EDGE'
  ) THEN 'PASS' ELSE 'FAIL' END AS status,
  'EDGE label child table exists' AS check_name,
  COALESCE((
    SELECT n.nspname || '."EDGE"'
    FROM pg_class c
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE n.nspname LIKE 'eq_%' AND c.relname = 'EDGE'
    LIMIT 1
  ), 'missing') AS detail;

\echo '=== SPEC-044: sqlx migrations applied ==='
SELECT
  CASE WHEN COUNT(*) >= 70 THEN 'PASS' ELSE 'FAIL' END AS status,
  'sqlx migrations count' AS check_name,
  COUNT(*)::text AS detail
FROM _sqlx_migrations;

\echo '=== SPEC-044 BT-044-01: AGE rejects inline literal (negative probe) ==='
-- This query MUST fail on real AGE — documents the bug class.
-- Wrapped in DO block; PASS if exception message contains 'must be a parameter'.
DO $$
DECLARE
  graph_name text;
  err_text text;
BEGIN
  SELECT name INTO graph_name FROM ag_catalog.ag_graph LIMIT 1;
  IF graph_name IS NULL THEN
    RAISE NOTICE 'SKIP: no graph for negative probe';
    RETURN;
  END IF;

  PERFORM * FROM cypher(graph_name, $$
    MATCH (n:Node {node_id: $node_id}) RETURN n LIMIT 1
  $$, '{"node_id":"__spec044_probe__"}'::agtype) AS (n agtype);

  RAISE EXCEPTION 'FAIL: inline agtype literal should have been rejected';
EXCEPTION WHEN OTHERS THEN
  err_text := SQLERRM;
  IF err_text LIKE '%must be a parameter%' THEN
    RAISE NOTICE 'PASS: AGE correctly rejects inline agtype third arg';
  ELSE
    RAISE EXCEPTION 'FAIL: unexpected error: %', err_text;
  END IF;
END $$;

\echo '=== SPEC-044 BT-044-37: Recent failed documents (operator signal) ==='
SELECT
  CASE WHEN COUNT(*) = 0 THEN 'PASS' ELSE 'WARN' END AS status,
  'documents Failed last 24h' AS check_name,
  COUNT(*)::text AS detail
FROM documents
WHERE status = 'Failed'
  AND updated_at > now() - interval '24 hours';
