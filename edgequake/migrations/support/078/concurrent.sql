-- SPEC-040 Migration 078 — CONCURRENT index build (production / large graphs)
--
-- IMPORTANT:
--   - Run OUTSIDE a transaction (psql default autocommit OK)
--   - Run during low-traffic window; each index builds without blocking writes
--   - Idempotent: IF NOT EXISTS on each index
--   - Run after sqlx M078 marker on small graphs; use this for >100k nodes
--
-- Usage:
--   psql "$DATABASE_URL" -f edgequake/migrations/support/078/concurrent.sql

\set ON_ERROR_STOP on
SET lock_timeout = '5s';
SET statement_timeout = '0';

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'SPEC-040 M078 concurrent: AGE not installed — skipping';
        RETURN;
    END IF;
    RAISE NOTICE 'SPEC-040 M078 concurrent: starting child-table index build...';
END $$;

SELECT format(
    'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_node_workspace_id ON %I."Node"
     ((ag_catalog.agtype_to_json(properties)->>''workspace_id''));',
    g.name
)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I."Node"', g.name)) IS NOT NULL
\gexec

SELECT format(
    'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_node_tenant_id ON %I."Node"
     ((ag_catalog.agtype_to_json(properties)->>''tenant_id''));',
    g.name
)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I."Node"', g.name)) IS NOT NULL
\gexec

SELECT format(
    'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_edge_start_id_text ON %I."EDGE" ((start_id::text));',
    g.name
)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I."EDGE"', g.name)) IS NOT NULL
\gexec

SELECT format(
    'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_edge_end_id_text ON %I."EDGE" ((end_id::text));',
    g.name
)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I."EDGE"', g.name)) IS NOT NULL
\gexec

SELECT format('ANALYZE %I."Node";', g.name)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I."Node"', g.name)) IS NOT NULL
\gexec

SELECT format('ANALYZE %I."EDGE";', g.name)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I."EDGE"', g.name)) IS NOT NULL
\gexec

DO $$ BEGIN RAISE NOTICE 'SPEC-040 M078 concurrent: complete'; END $$;
