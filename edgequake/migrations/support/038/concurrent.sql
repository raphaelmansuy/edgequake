-- SPEC-006 Migration 038 — CONCURRENT index build (production / large graphs)
--
-- IMPORTANT:
--   - Run OUTSIDE a transaction (psql default autocommit OK)
--   - Run during low-traffic window; each index builds without blocking writes
--   - Idempotent: IF NOT EXISTS on each index
--   - Run support/038/preflight.sql first (or apply_038.sh --dry-run)
--
-- Usage:
--   edgequake/scripts/migrations/apply_038.sh --dry-run
--   edgequake/scripts/migrations/apply_038.sh --apply --concurrent --yes

\set ON_ERROR_STOP on
SET lock_timeout = '5s';
SET statement_timeout = '0';

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'AGE not installed — skipping concurrent index build';
        RETURN;
    END IF;
    RAISE NOTICE 'Starting concurrent index build (SPEC-006 038)...';
END $$;

-- Per-graph indexes (template executed via psql \gexec pattern below)
SELECT format(
    'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_vertex_source_id ON %I."_ag_label_vertex" '
    '((ag_catalog.agtype_to_json(properties)->>''source_id''));',
    replace(g.name, '.', '_'), g.name
)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I._ag_label_vertex', g.name)) IS NOT NULL
\gexec

SELECT format(
    'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_vertex_source_ids_gin ON %I."_ag_label_vertex" '
    'USING gin ((ag_catalog.agtype_to_json(properties)->''source_ids''));',
    replace(g.name, '.', '_'), g.name
)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I._ag_label_vertex', g.name)) IS NOT NULL
\gexec

SELECT format(
    'CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_%s_edge_source_ids_gin ON %I."_ag_label_edge" '
    'USING gin ((ag_catalog.agtype_to_json(properties)->''source_ids''));',
    replace(g.name, '.', '_'), g.name
)
FROM ag_catalog.ag_graph g
WHERE to_regclass(format('%I._ag_label_edge', g.name)) IS NOT NULL
\gexec

DO $$ BEGIN RAISE NOTICE 'Concurrent index build complete (038)'; END $$;
