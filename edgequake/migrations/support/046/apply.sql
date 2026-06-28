-- Migration 046 — graph tenant isolation + hot-path query indexes (SSOT)
--
-- WHY: Scoped graph reads filter on tenant_id / workspace_id (vertices) and
-- source_id / target_id (edges). Without expression indexes PostgreSQL falls
-- back to seq scans or nested-loop joins (22s+ on ~30k nodes — SPEC-006).
--
-- Safe to run on every bootstrap (CREATE INDEX IF NOT EXISTS).

DO $$
DECLARE
    graph_name text;
    idx_prefix text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'Apache AGE not installed — skipping migration 046 graph indexes';
        RETURN;
    END IF;

    FOR graph_name IN SELECT name FROM ag_catalog.ag_graph ORDER BY name
    LOOP
        idx_prefix := replace(graph_name, '.', '_');
        RAISE NOTICE 'Migration 046: ensuring graph perf indexes for %', graph_name;

        -- Vertex tenant isolation (list / popular / search)
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_tenant_id ON %I."_ag_label_vertex" '
                '((ag_catalog.agtype_to_json(properties)->>''tenant_id''))',
                idx_prefix, graph_name
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_workspace_id ON %I."_ag_label_vertex" '
                '((ag_catalog.agtype_to_json(properties)->>''workspace_id''))',
                idx_prefix, graph_name
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_tenant_workspace ON %I."_ag_label_vertex" '
                '((ag_catalog.agtype_to_json(properties)->>''tenant_id''), '
                '(ag_catalog.agtype_to_json(properties)->>''workspace_id''))',
                idx_prefix, graph_name
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_entity_type ON %I."_ag_label_vertex" '
                '((ag_catalog.agtype_to_json(properties)->>''entity_type''))',
                idx_prefix, graph_name
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_node_id ON %I."_ag_label_vertex" '
                '((ag_catalog.agtype_to_json(properties)->>''node_id''))',
                idx_prefix, graph_name
            );
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ vertex isolation indexes for %: %', graph_name, SQLERRM;
        END;

        -- Edge endpoint + tenant filters (get_edges_for_node_set, edge lists)
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_edge_source_id ON %I."_ag_label_edge" '
                '((ag_catalog.agtype_to_json(properties)->>''source_id''))',
                idx_prefix, graph_name
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_edge_target_id ON %I."_ag_label_edge" '
                '((ag_catalog.agtype_to_json(properties)->>''target_id''))',
                idx_prefix, graph_name
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_edge_tenant_id ON %I."_ag_label_edge" '
                '((ag_catalog.agtype_to_json(properties)->>''tenant_id''))',
                idx_prefix, graph_name
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_edge_workspace_id ON %I."_ag_label_edge" '
                '((ag_catalog.agtype_to_json(properties)->>''workspace_id''))',
                idx_prefix, graph_name
            );
            -- Structural join keys for filter-first degree aggregation
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_ag_edge_start_id ON %I."_ag_label_edge" (start_id)',
                idx_prefix, graph_name
            );
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_ag_edge_end_id ON %I."_ag_label_edge" (end_id)',
                idx_prefix, graph_name
            );
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ edge perf indexes for %: %', graph_name, SQLERRM;
        END;
    END LOOP;

    RAISE NOTICE 'Migration 046 graph isolation perf indexes complete';
END $$;
