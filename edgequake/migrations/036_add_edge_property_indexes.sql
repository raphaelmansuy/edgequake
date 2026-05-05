-- Migration 036: Add expression indexes on Apache AGE edge table properties
--
-- WHY: The `get_edges_for_node_set` function was rewritten from Cypher to native
-- SQL (see edgequake-storage PR). The new SQL filters directly on edge properties:
--
--   WHERE ag_catalog.agtype_to_json(properties)->>'source_id' IN (...)
--     AND ag_catalog.agtype_to_json(properties)->>'target_id' IN (...)
--
-- Without indexes, this requires a full seq-scan of `_ag_label_edge`. With
-- expression indexes on source_id and target_id, PostgreSQL can use bitmap index
-- scans and reduce edge lookup from O(N_edges) to O(matches).
--
-- Migration 014 already created similar indexes on `_ag_label_vertex`. This
-- migration adds the complementary indexes on `_ag_label_edge`.
--
-- Safe to run multiple times (all are CREATE INDEX IF NOT EXISTS).

DO $$
DECLARE
    graph_name text;
    graph_schema text;
BEGIN
    -- Only run if Apache AGE is installed
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'Apache AGE not installed — skipping edge index creation';
        RETURN;
    END IF;

    FOR graph_name IN
        SELECT name FROM ag_catalog.ag_graph
    LOOP
        graph_schema := graph_name;
        RAISE NOTICE 'Creating edge property indexes for graph: %', graph_name;

        -- Index 1: source_id — used in `get_edges_for_node_set` IN filter
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_edge_source_id ON %I."_ag_label_edge" '
                '((ag_catalog.agtype_to_json(properties)->>''source_id''))',
                replace(graph_name, '.', '_'),
                graph_schema
            );
            RAISE NOTICE '  ✓ Created edge index on source_id for graph %', graph_name;
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create edge source_id index for %: %', graph_name, SQLERRM;
        END;

        -- Index 2: target_id — used in `get_edges_for_node_set` IN filter
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_edge_target_id ON %I."_ag_label_edge" '
                '((ag_catalog.agtype_to_json(properties)->>''target_id''))',
                replace(graph_name, '.', '_'),
                graph_schema
            );
            RAISE NOTICE '  ✓ Created edge index on target_id for graph %', graph_name;
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create edge target_id index for %: %', graph_name, SQLERRM;
        END;

        -- Index 3: workspace_id — used in tenant isolation filter on edges
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_edge_workspace_id ON %I."_ag_label_edge" '
                '((ag_catalog.agtype_to_json(properties)->>''workspace_id''))',
                replace(graph_name, '.', '_'),
                graph_schema
            );
            RAISE NOTICE '  ✓ Created edge index on workspace_id for graph %', graph_name;
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create edge workspace_id index for %: %', graph_name, SQLERRM;
        END;

        -- Index 4: tenant_id — used in tenant isolation filter on edges
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_edge_tenant_id ON %I."_ag_label_edge" '
                '((ag_catalog.agtype_to_json(properties)->>''tenant_id''))',
                replace(graph_name, '.', '_'),
                graph_schema
            );
            RAISE NOTICE '  ✓ Created edge index on tenant_id for graph %', graph_name;
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create edge tenant_id index for %: %', graph_name, SQLERRM;
        END;

    END LOOP;

    RAISE NOTICE 'Edge property index creation completed';
END $$;

-- Verification: list all edge indexes created
DO $$
DECLARE
    rec RECORD;
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE '=== Edge Property Indexes Summary ===';
        FOR rec IN
            SELECT schemaname, tablename, indexname
            FROM pg_indexes
            WHERE tablename = '_ag_label_edge'
              AND (
                    indexname LIKE 'idx_%_edge_source_id'
                 OR indexname LIKE 'idx_%_edge_target_id'
                 OR indexname LIKE 'idx_%_edge_workspace_id'
                 OR indexname LIKE 'idx_%_edge_tenant_id'
              )
            ORDER BY schemaname, indexname
        LOOP
            RAISE NOTICE 'Index: %.% on %', rec.schemaname, rec.indexname, rec.tablename;
        END LOOP;
    END IF;
END $$;
