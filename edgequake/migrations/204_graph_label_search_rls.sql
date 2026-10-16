-- Keyword checks call search_labels with LIMIT 1. The statement was
-- `lower(node_id) LIKE '%…%'` unioned with the same predicate on
-- `entity_type`, over every node in the workspace. `text LIKE` is not
-- leakproof, so under forced row security the trigram indexes cannot be
-- index conditions. As edgequake_tenant_access that plan bitmap-scans
-- idx_eq_graph_nodes_tenant_ws twice and filters afterwards: 64,404 nodes,
-- 1.741 s, 41,785 buffer reads. The keyword "person" does this on the mix
-- query path.
--
-- This function runs as the table owner, which bypasses row security, and
-- applies the session tenant itself. When app.current_tenant_id is set,
-- that value wins over the caller argument. The node arm then uses
-- idx_eq_graph_nodes_id_trgm and stops at LIMIT (0.9 ms warm for
-- "person"). The type arm is a loose index scan of
-- idx_eq_graph_nodes_ws_type (13 distinct types, 1.063 ms) and the LIKE
-- runs on those types, not on every PERSON row.

CREATE OR REPLACE FUNCTION public.eq_graph_search_labels(
    p_namespace text,
    p_query text,
    p_limit integer,
    p_tenant text,
    p_workspace text
) RETURNS TABLE(label text)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = public
AS $fn$
DECLARE
    v_tenant text := NULLIF(current_setting('app.current_tenant_id', true), '');
    v_workspace text := NULLIF(current_setting('app.current_workspace_id', true), '');
BEGIN
    IF p_limit IS NULL OR p_limit < 1 OR p_namespace IS NULL OR p_query IS NULL THEN
        RETURN;
    END IF;
    IF v_tenant IS NULL THEN
        v_tenant := NULLIF(p_tenant, '');
    END IF;
    IF v_workspace IS NULL THEN
        v_workspace := NULLIF(p_workspace, '');
    END IF;

    IF v_tenant IS NOT NULL AND v_workspace IS NOT NULL THEN
        RETURN QUERY
        SELECT picked.label
        FROM (
            (
                SELECT n.node_id AS label
                FROM public.eq_graph_nodes n
                WHERE n.namespace = p_namespace
                  AND n.tenant_id = v_tenant
                  AND n.workspace_id = v_workspace
                  AND lower(n.node_id) LIKE '%' || p_query || '%'
                ORDER BY n.node_id
                LIMIT p_limit
            )
            UNION
            (
                WITH RECURSIVE types AS (
                    (
                        SELECT n.entity_type
                        FROM public.eq_graph_nodes n
                        WHERE n.namespace = p_namespace
                          AND n.workspace_id = v_workspace
                          AND n.tenant_id = v_tenant
                          AND n.entity_type IS NOT NULL
                          AND n.entity_type <> ''
                        ORDER BY n.entity_type
                        LIMIT 1
                    )
                    UNION ALL
                    SELECT next_type.entity_type
                    FROM types
                    JOIN LATERAL (
                        SELECT n.entity_type
                        FROM public.eq_graph_nodes n
                        WHERE n.namespace = p_namespace
                          AND n.workspace_id = v_workspace
                          AND n.entity_type > types.entity_type
                          AND n.entity_type <> ''
                          AND n.tenant_id = v_tenant
                        ORDER BY n.entity_type
                        LIMIT 1
                    ) next_type ON true
                )
                SELECT types.entity_type
                FROM types
                WHERE types.entity_type IS NOT NULL
                  AND lower(types.entity_type) LIKE '%' || p_query || '%'
                ORDER BY types.entity_type
                LIMIT p_limit
            )
        ) picked
        WHERE picked.label IS NOT NULL AND picked.label <> ''
        ORDER BY picked.label
        LIMIT p_limit;
        RETURN;
    END IF;

    RETURN QUERY
    SELECT picked.label
    FROM (
        (
            SELECT n.node_id AS label
            FROM public.eq_graph_nodes n
            WHERE n.namespace = p_namespace
              AND (v_tenant IS NULL OR n.tenant_id = v_tenant)
              AND (v_workspace IS NULL OR n.workspace_id = v_workspace)
              AND lower(n.node_id) LIKE '%' || p_query || '%'
            ORDER BY n.node_id
            LIMIT p_limit
        )
        UNION
        (
            SELECT n.entity_type
            FROM public.eq_graph_nodes n
            WHERE n.namespace = p_namespace
              AND (v_tenant IS NULL OR n.tenant_id = v_tenant)
              AND (v_workspace IS NULL OR n.workspace_id = v_workspace)
              AND n.entity_type IS NOT NULL
              AND lower(n.entity_type) LIKE '%' || p_query || '%'
            ORDER BY n.entity_type
            LIMIT p_limit
        )
    ) picked
    WHERE picked.label IS NOT NULL AND picked.label <> ''
    ORDER BY picked.label
    LIMIT p_limit;
END;
$fn$;

REVOKE ALL ON FUNCTION public.eq_graph_search_labels(text, text, integer, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.eq_graph_search_labels(text, text, integer, text, text) TO edgequake_tenant_access;
