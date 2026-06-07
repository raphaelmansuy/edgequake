-- SPEC-006 Migration 038 — Pre-flight checks (read-only, safe on production)
-- Run BEFORE applying 038_add_source_ids_gin_indexes.sql
--
-- Usage:
--   edgequake/scripts/migrations/apply_038.sh --dry-run
--   psql "$DATABASE_URL" -f edgequake/migrations/support/038/preflight.sql

DO $$
DECLARE
    graph_name text;
    graph_schema text;
    vertex_tbl regclass;
    edge_tbl regclass;
    vertex_count bigint;
    edge_count bigint;
    age_ok boolean;
BEGIN
    age_ok := EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age');
    RAISE NOTICE '=== SPEC-006 Migration 038 Pre-flight ===';
    RAISE NOTICE 'AGE extension installed: %', age_ok;

    IF NOT age_ok THEN
        RAISE NOTICE 'SKIP: No AGE — migration 038 is a no-op (safe)';
        RETURN;
    END IF;

    FOR graph_name IN SELECT name FROM ag_catalog.ag_graph LOOP
        graph_schema := graph_name;
        vertex_count := NULL;
        edge_count := NULL;
        vertex_tbl := to_regclass(format('%I._ag_label_vertex', graph_schema));
        edge_tbl := to_regclass(format('%I._ag_label_edge', graph_schema));

        RAISE NOTICE '--- Graph: % ---', graph_name;
        RAISE NOTICE '  _ag_label_vertex exists: %', vertex_tbl IS NOT NULL;
        RAISE NOTICE '  _ag_label_edge exists: %', edge_tbl IS NOT NULL;

        IF vertex_tbl IS NOT NULL THEN
            EXECUTE format('SELECT COUNT(*) FROM %s."_ag_label_vertex"', graph_schema)
                INTO vertex_count;
            RAISE NOTICE '  vertex rows: %', vertex_count;
        END IF;

        IF edge_tbl IS NOT NULL THEN
            EXECUTE format('SELECT COUNT(*) FROM %s."_ag_label_edge"', graph_schema)
                INTO edge_count;
            RAISE NOTICE '  edge rows: %', edge_count;
        END IF;

        IF vertex_count IS NOT NULL AND vertex_count > 500_000 THEN
            RAISE WARNING 'Large graph (%) — prefer 038_concurrent script for zero-downtime', vertex_count;
        END IF;
    END LOOP;

    RAISE NOTICE 'Pre-flight complete. No schema changes made.';
END $$;
