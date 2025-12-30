-- ============================================================================
-- EdgeQuake Apache AGE Graph Extension Setup
-- Version: 1.0.0
-- Date: 2025-01-28
-- Description: Set up Apache AGE for graph database features
-- ============================================================================
--
-- Apache AGE is OPTIONAL - this migration will gracefully handle cases
-- where AGE is not installed on the PostgreSQL server.
--
-- REQUIREMENTS:
--   - PostgreSQL 11-17
--   - Apache AGE extension (optional)
--
-- ============================================================================

-- ============================================================================
-- SECTION 1: ATTEMPT AGE EXTENSION INSTALLATION
-- ============================================================================

DO $$ 
DECLARE
    age_available BOOLEAN := FALSE;
BEGIN
    -- Attempt to create the AGE extension
    BEGIN
        CREATE EXTENSION IF NOT EXISTS age CASCADE;
        age_available := TRUE;
        RAISE NOTICE 'Apache AGE extension installed successfully';
    EXCEPTION WHEN OTHERS THEN
        RAISE NOTICE 'Apache AGE extension not available: %', SQLERRM;
        RAISE NOTICE 'Graph operations will use relational fallback storage';
    END;

    -- If AGE is available, set up the search path
    IF age_available THEN
        EXECUTE 'SET search_path = ag_catalog, "$user", public';
        RAISE NOTICE 'AGE search path configured';
    END IF;
END $$;

-- ============================================================================
-- SECTION 2: CREATE GRAPH HELPER FUNCTIONS
-- ============================================================================

-- Function to safely create an AGE graph
CREATE OR REPLACE FUNCTION create_age_graph_safe(graph_name TEXT)
RETURNS TEXT AS $$
DECLARE
    result TEXT;
BEGIN
    -- Check if AGE extension exists
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RETURN 'AGE extension not available - using relational storage';
    END IF;

    -- Check if graph already exists
    EXECUTE format('SELECT 1 FROM ag_catalog.ag_graph WHERE name = %L', graph_name)
        INTO result;
    
    IF result IS NOT NULL THEN
        RETURN 'Graph already exists: ' || graph_name;
    END IF;

    -- Create the graph
    EXECUTE format('SELECT * FROM ag_catalog.create_graph(%L)', graph_name);
    RETURN 'Created graph: ' || graph_name;
EXCEPTION WHEN OTHERS THEN
    RETURN 'Error creating graph: ' || SQLERRM;
END;
$$ LANGUAGE plpgsql;

-- Function to check if AGE is available
CREATE OR REPLACE FUNCTION is_age_available()
RETURNS BOOLEAN AS $$
BEGIN
    RETURN EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age');
END;
$$ LANGUAGE plpgsql STABLE;

-- Function to drop an AGE graph safely
CREATE OR REPLACE FUNCTION drop_age_graph_safe(graph_name TEXT)
RETURNS TEXT AS $$
BEGIN
    -- Check if AGE extension exists
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RETURN 'AGE extension not available';
    END IF;

    -- Check if graph exists
    IF NOT EXISTS (
        SELECT 1 FROM ag_catalog.ag_graph WHERE name = graph_name
    ) THEN
        RETURN 'Graph does not exist: ' || graph_name;
    END IF;

    -- Drop the graph
    EXECUTE format('SELECT * FROM ag_catalog.drop_graph(%L, true)', graph_name);
    RETURN 'Dropped graph: ' || graph_name;
EXCEPTION WHEN OTHERS THEN
    RETURN 'Error dropping graph: ' || SQLERRM;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- SECTION 3: FALLBACK GRAPH STORAGE TABLES
-- ============================================================================
-- These tables are used when AGE is not available

-- Graph nodes (fallback)
CREATE TABLE IF NOT EXISTS graph_nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    graph_name VARCHAR(100) NOT NULL,
    node_id TEXT NOT NULL,
    label VARCHAR(100) NOT NULL DEFAULT 'Node',
    properties JSONB NOT NULL DEFAULT '{}',
    tenant_id UUID,
    workspace_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT graph_nodes_unique UNIQUE (graph_name, node_id)
);

CREATE INDEX IF NOT EXISTS idx_graph_nodes_graph ON graph_nodes(graph_name);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_label ON graph_nodes(graph_name, label);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_tenant ON graph_nodes(tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_graph_nodes_properties ON graph_nodes USING GIN (properties);

-- Graph edges (fallback)
CREATE TABLE IF NOT EXISTS graph_edges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    graph_name VARCHAR(100) NOT NULL,
    source_node_id TEXT NOT NULL,
    target_node_id TEXT NOT NULL,
    label VARCHAR(100) NOT NULL DEFAULT 'RELATED_TO',
    properties JSONB NOT NULL DEFAULT '{}',
    tenant_id UUID,
    workspace_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT graph_edges_unique UNIQUE (graph_name, source_node_id, target_node_id, label)
);

CREATE INDEX IF NOT EXISTS idx_graph_edges_graph ON graph_edges(graph_name);
CREATE INDEX IF NOT EXISTS idx_graph_edges_source ON graph_edges(graph_name, source_node_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_target ON graph_edges(graph_name, target_node_id);
CREATE INDEX IF NOT EXISTS idx_graph_edges_label ON graph_edges(graph_name, label);
CREATE INDEX IF NOT EXISTS idx_graph_edges_tenant ON graph_edges(tenant_id, workspace_id);

-- ============================================================================
-- SECTION 4: ENABLE RLS ON FALLBACK TABLES
-- ============================================================================

ALTER TABLE graph_nodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE graph_edges ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS graph_nodes_tenant_isolation ON graph_nodes;
CREATE POLICY graph_nodes_tenant_isolation ON graph_nodes
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (tenant_id IS NULL OR tenant_id = current_tenant_id());

DROP POLICY IF EXISTS graph_edges_tenant_isolation ON graph_edges;
CREATE POLICY graph_edges_tenant_isolation ON graph_edges
    FOR ALL
    USING (
        tenant_id IS NULL 
        OR (
            tenant_id = current_tenant_id()
            AND (current_workspace_id() IS NULL OR workspace_id = current_workspace_id())
        )
    )
    WITH CHECK (tenant_id IS NULL OR tenant_id = current_tenant_id());

-- ============================================================================
-- SECTION 5: GRAPH QUERY HELPERS FOR FALLBACK STORAGE
-- ============================================================================

-- Function to upsert a node in fallback storage
CREATE OR REPLACE FUNCTION upsert_graph_node(
    p_graph_name TEXT,
    p_node_id TEXT,
    p_label TEXT,
    p_properties JSONB,
    p_tenant_id UUID DEFAULT NULL,
    p_workspace_id UUID DEFAULT NULL
)
RETURNS UUID AS $$
DECLARE
    result_id UUID;
BEGIN
    INSERT INTO graph_nodes (graph_name, node_id, label, properties, tenant_id, workspace_id)
    VALUES (p_graph_name, p_node_id, p_label, p_properties, p_tenant_id, p_workspace_id)
    ON CONFLICT (graph_name, node_id) DO UPDATE SET
        label = EXCLUDED.label,
        properties = EXCLUDED.properties,
        updated_at = NOW()
    RETURNING id INTO result_id;
    
    RETURN result_id;
END;
$$ LANGUAGE plpgsql;

-- Function to upsert an edge in fallback storage
CREATE OR REPLACE FUNCTION upsert_graph_edge(
    p_graph_name TEXT,
    p_source_id TEXT,
    p_target_id TEXT,
    p_label TEXT,
    p_properties JSONB,
    p_tenant_id UUID DEFAULT NULL,
    p_workspace_id UUID DEFAULT NULL
)
RETURNS UUID AS $$
DECLARE
    result_id UUID;
BEGIN
    INSERT INTO graph_edges (graph_name, source_node_id, target_node_id, label, properties, tenant_id, workspace_id)
    VALUES (p_graph_name, p_source_id, p_target_id, p_label, p_properties, p_tenant_id, p_workspace_id)
    ON CONFLICT (graph_name, source_node_id, target_node_id, label) DO UPDATE SET
        properties = EXCLUDED.properties
    RETURNING id INTO result_id;
    
    RETURN result_id;
END;
$$ LANGUAGE plpgsql;

-- Function to find neighbors in fallback storage
CREATE OR REPLACE FUNCTION get_node_neighbors(
    p_graph_name TEXT,
    p_node_id TEXT,
    p_direction TEXT DEFAULT 'both', -- 'outgoing', 'incoming', 'both'
    p_edge_label TEXT DEFAULT NULL
)
RETURNS TABLE (
    neighbor_id TEXT,
    edge_label TEXT,
    edge_properties JSONB,
    direction TEXT
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        e.target_node_id as neighbor_id,
        e.label as edge_label,
        e.properties as edge_properties,
        'outgoing'::TEXT as direction
    FROM graph_edges e
    WHERE e.graph_name = p_graph_name
      AND e.source_node_id = p_node_id
      AND (p_edge_label IS NULL OR e.label = p_edge_label)
      AND (p_direction IN ('outgoing', 'both'))
    UNION ALL
    SELECT 
        e.source_node_id as neighbor_id,
        e.label as edge_label,
        e.properties as edge_properties,
        'incoming'::TEXT as direction
    FROM graph_edges e
    WHERE e.graph_name = p_graph_name
      AND e.target_node_id = p_node_id
      AND (p_edge_label IS NULL OR e.label = p_edge_label)
      AND (p_direction IN ('incoming', 'both'));
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- COMPLETION NOTICE
-- ============================================================================

DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE '============================================';
        RAISE NOTICE 'AGE Graph Extension Setup Complete!';
        RAISE NOTICE 'Apache AGE is available - using native graph storage';
        RAISE NOTICE '============================================';
    ELSE
        RAISE NOTICE '============================================';
        RAISE NOTICE 'AGE Graph Extension Setup Complete!';
        RAISE NOTICE 'Apache AGE not available - using fallback tables';
        RAISE NOTICE 'Fallback tables: graph_nodes, graph_edges';
        RAISE NOTICE '============================================';
    END IF;
END $$;
