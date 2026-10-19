-- Typo recovery for graph label search under forced row security.
--
-- eq_graph_search_labels (204) matches substrings. When it finds nothing,
-- the adapter retried with `lower(node_id) % $q` ordered by similarity(). That
-- retry ran as edgequake_tenant_access. pg_trgm's `%` is not leakproof, so
-- forced row security evaluates the tenant predicate first and `%` cannot be an
-- index condition. The plan bitmap-scans every node in the workspace and
-- filters each one. Workspace of 66,682 nodes, "lymph node":
--
--     as edgequake_tenant_access   835 ms   Rows Removed by Filter: 66682
--     as table owner (this fn)      28 ms   idx_eq_graph_nodes_id_trgm
--
-- Multi-word keywords take this path on every query: five retrieval requests
-- issued thirteen of these and spent 1.2 s of 1.75 s total SQL time in them.
--
-- This function runs as the table owner, like 204 and 206, and applies the
-- session tenant itself. When app.current_tenant_id / app.current_workspace_id
-- are set they win over the caller's arguments, so a caller cannot widen the
-- scope. EXECUTE goes to edgequake_tenant_access only.

CREATE OR REPLACE FUNCTION public.eq_graph_search_labels_fuzzy(
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

    -- Fully scoped: plain equalities keep the planner on the scope index.
    IF v_tenant IS NOT NULL AND v_workspace IS NOT NULL THEN
        RETURN QUERY
        SELECT n.node_id
        FROM public.eq_graph_nodes n
        WHERE n.namespace = p_namespace
          AND n.tenant_id = v_tenant
          AND n.workspace_id = v_workspace
          AND lower(n.node_id) % p_query
        ORDER BY public.similarity(lower(n.node_id), p_query) DESC, n.node_id
        LIMIT p_limit;
        RETURN;
    END IF;

    RETURN QUERY
    SELECT n.node_id
    FROM public.eq_graph_nodes n
    WHERE n.namespace = p_namespace
      AND (v_tenant IS NULL OR n.tenant_id = v_tenant)
      AND (v_workspace IS NULL OR n.workspace_id = v_workspace)
      AND lower(n.node_id) % p_query
    ORDER BY public.similarity(lower(n.node_id), p_query) DESC, n.node_id
    LIMIT p_limit;
END;
$fn$;

REVOKE ALL ON FUNCTION public.eq_graph_search_labels_fuzzy(text, text, integer, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.eq_graph_search_labels_fuzzy(text, text, integer, text, text) TO edgequake_tenant_access;
