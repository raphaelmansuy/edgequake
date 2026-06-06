-- SPEC-006 Migration 038 — Size-aware index apply (SSOT for bootstrap + ops)
--
-- Used by:
--   migration_bootstrap.rs (startup, after sqlx marker 038)
--   edgequake/scripts/migrations/apply_038.sh --apply
--
-- Session variable (optional, set by bootstrap):
--   edgequake.migration_large_graph_threshold — default 500000 vertices
--
-- Large graphs are SKIPPED here; use concurrent.sql for zero-downtime builds.

DO $$
DECLARE
    graph_name text;
    graph_schema text;
    idx_prefix text;
    vertex_tbl regclass;
    edge_tbl regclass;
    vertex_count bigint;
    threshold bigint;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'AGE not installed — skipping source_ids index apply';
        RETURN;
    END IF;

    threshold := COALESCE(
        NULLIF(current_setting('edgequake.migration_large_graph_threshold', true), '')::bigint,
        500000
    );

    RAISE NOTICE 'Migration 038 apply — large graph threshold: % vertices', threshold;

    FOR graph_name IN SELECT name FROM ag_catalog.ag_graph LOOP
        graph_schema := graph_name;
        idx_prefix := replace(graph_name, '.', '_');
        vertex_tbl := to_regclass(format('%I._ag_label_vertex', graph_schema));
        edge_tbl := to_regclass(format('%I._ag_label_edge', graph_schema));

        IF vertex_tbl IS NULL AND edge_tbl IS NULL THEN
            RAISE NOTICE 'Skip graph % — AGE label tables not present', graph_name;
            CONTINUE;
        END IF;

        vertex_count := NULL;
        IF vertex_tbl IS NOT NULL THEN
            EXECUTE format('SELECT COUNT(*) FROM %s."_ag_label_vertex"', graph_schema)
                INTO vertex_count;
        END IF;

        IF vertex_count IS NOT NULL AND vertex_count >= threshold THEN
            RAISE WARNING 'Defer graph % (%) — exceeds threshold %; use apply_038.sh --concurrent',
                graph_name, vertex_count, threshold;
            CONTINUE;
        END IF;

        RAISE NOTICE 'Applying source_ids indexes for graph: % (vertices=%)', graph_name, COALESCE(vertex_count, 0);

        IF vertex_tbl IS NOT NULL THEN
            BEGIN
                EXECUTE format(
                    'CREATE INDEX IF NOT EXISTS idx_%s_vertex_source_id ON %I."_ag_label_vertex" '
                    '((ag_catalog.agtype_to_json(properties)->>''source_id''))',
                    idx_prefix, graph_schema
                );
                RAISE NOTICE '  ✓ vertex source_id btree';
            EXCEPTION WHEN OTHERS THEN
                RAISE NOTICE '  ✗ vertex source_id: %', SQLERRM;
            END;

            BEGIN
                EXECUTE format(
                    'CREATE INDEX IF NOT EXISTS idx_%s_vertex_source_ids_gin ON %I."_ag_label_vertex" '
                    'USING gin ((ag_catalog.agtype_to_json(properties)->''source_ids''))',
                    idx_prefix, graph_schema
                );
                RAISE NOTICE '  ✓ vertex source_ids GIN';
            EXCEPTION WHEN OTHERS THEN
                RAISE NOTICE '  ✗ vertex source_ids GIN: %', SQLERRM;
            END;
        END IF;

        IF edge_tbl IS NOT NULL THEN
            BEGIN
                EXECUTE format(
                    'CREATE INDEX IF NOT EXISTS idx_%s_edge_source_ids_gin ON %I."_ag_label_edge" '
                    'USING gin ((ag_catalog.agtype_to_json(properties)->''source_ids''))',
                    idx_prefix, graph_schema
                );
                RAISE NOTICE '  ✓ edge source_ids GIN';
            EXCEPTION WHEN OTHERS THEN
                RAISE NOTICE '  ✗ edge source_ids GIN: %', SQLERRM;
            END;
        END IF;
    END LOOP;

    RAISE NOTICE 'Migration 038 size-aware apply complete';
END $$;
