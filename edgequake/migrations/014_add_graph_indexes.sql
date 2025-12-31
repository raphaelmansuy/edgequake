-- ============================================================================
-- EdgeQuake Graph Performance Indexes
-- Version: 1.0.0
-- Date: 2025-12-30
-- Description: Add indexes to AGE graph nodes for faster filtering
-- ============================================================================
--
-- This migration adds indexes to Apache AGE graph vertex properties
-- to improve query performance for tenant/workspace filtering.
--
-- INDEXES CREATED:
--   - tenant_id for multi-tenancy filtering
--   - workspace_id for workspace isolation
--   - entity_type for entity filtering
--   - Combined (tenant_id, workspace_id) for common queries
--
-- ============================================================================

DO $$ 
DECLARE
    graph_name TEXT;
    vertex_table TEXT;
    index_exists BOOLEAN;
BEGIN
    -- Check if AGE extension is available
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'Apache AGE extension not available - skipping graph indexes';
        RETURN;
    END IF;

    -- Find all graphs in the database
    FOR graph_name IN 
        SELECT name FROM ag_catalog.ag_graph
    LOOP
        RAISE NOTICE 'Adding indexes to graph: %', graph_name;
        
        -- Get the vertex table name (schema.table format)
        vertex_table := graph_name || '._ag_label_vertex';
        
        -- Index 1: tenant_id for multi-tenancy queries
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_tenant_id ON %I._ag_label_vertex ((ag_catalog.agtype_to_json(properties)->>''tenant_id''))',
                replace(graph_name, '.', '_'),
                graph_name
            );
            RAISE NOTICE '  ✓ Created index on tenant_id';
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create tenant_id index: %', SQLERRM;
        END;
        
        -- Index 2: workspace_id for workspace filtering
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_workspace_id ON %I._ag_label_vertex ((ag_catalog.agtype_to_json(properties)->>''workspace_id''))',
                replace(graph_name, '.', '_'),
                graph_name
            );
            RAISE NOTICE '  ✓ Created index on workspace_id';
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create workspace_id index: %', SQLERRM;
        END;
        
        -- Index 3: entity_type for entity filtering
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_entity_type ON %I._ag_label_vertex ((ag_catalog.agtype_to_json(properties)->>''entity_type''))',
                replace(graph_name, '.', '_'),
                graph_name
            );
            RAISE NOTICE '  ✓ Created index on entity_type';
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create entity_type index: %', SQLERRM;
        END;
        
        -- Index 4: Combined (tenant_id, workspace_id) for common queries
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_tenant_workspace ON %I._ag_label_vertex ((ag_catalog.agtype_to_json(properties)->>''tenant_id''), (ag_catalog.agtype_to_json(properties)->>''workspace_id''))',
                replace(graph_name, '.', '_'),
                graph_name
            );
            RAISE NOTICE '  ✓ Created index on (tenant_id, workspace_id)';
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create tenant_workspace index: %', SQLERRM;
        END;
        
        -- Index 5: node_id for fast node lookups
        BEGIN
            EXECUTE format(
                'CREATE INDEX IF NOT EXISTS idx_%s_node_id ON %I._ag_label_vertex ((ag_catalog.agtype_to_json(properties)->>''node_id''))',
                replace(graph_name, '.', '_'),
                graph_name
            );
            RAISE NOTICE '  ✓ Created index on node_id';
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE '  ✗ Failed to create node_id index: %', SQLERRM;
        END;
        
    END LOOP;
    
    RAISE NOTICE 'Graph index creation completed';
    
END $$;

-- ============================================================================
-- VERIFICATION: List all indexes created
-- ============================================================================

DO $$
DECLARE
    rec RECORD;
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE '=== Graph Indexes Summary ===';
        
        FOR rec IN
            SELECT 
                schemaname,
                tablename,
                indexname
            FROM pg_indexes
            WHERE indexname LIKE 'idx_%_tenant%'
               OR indexname LIKE 'idx_%_workspace%'
               OR indexname LIKE 'idx_%_entity%'
               OR indexname LIKE 'idx_%_node_id'
            ORDER BY schemaname, tablename, indexname
        LOOP
            RAISE NOTICE 'Index: %.% on %', rec.schemaname, rec.indexname, rec.tablename;
        END LOOP;
    END IF;
END $$;
