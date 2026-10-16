-- Forced row security runs before a user predicate unless every function
-- in that predicate is leakproof. `jsonb ->>` is not. Equality on a
-- stored generated column is, so these copies of modality and
-- current_stage can be index conditions for edgequake_tenant_access.
-- The expression indexes from 189, 198, and 201 stay; they still serve
-- the table owner.

ALTER TABLE public.chunks
    ADD COLUMN IF NOT EXISTS modality text
    GENERATED ALWAYS AS (metadata ->> 'modality') STORED;

CREATE INDEX IF NOT EXISTS idx_chunks_tenant_modality
    ON public.chunks (tenant_id, modality)
    WHERE modality IS NOT NULL;

ALTER TABLE public.entities
    ADD COLUMN IF NOT EXISTS modality text
    GENERATED ALWAYS AS (metadata ->> 'modality') STORED;

CREATE INDEX IF NOT EXISTS idx_entities_tenant_modality
    ON public.entities (tenant_id, modality)
    WHERE modality IS NOT NULL;

ALTER TABLE public.documents
    ADD COLUMN IF NOT EXISTS current_stage text
    GENERATED ALWAYS AS (metadata ->> 'current_stage') STORED;

CREATE INDEX IF NOT EXISTS idx_documents_ws_tenant_stage_col
    ON public.documents (workspace_id, tenant_id, current_stage, created_at DESC, id DESC)
    WHERE tenant_id IS NOT NULL AND current_stage IS NOT NULL;

-- search_nodes filters with LIKE on node_id and description. LIKE is not
-- leakproof, so the tenant role cannot use the trigram indexes and walks
-- the workspace. This function runs as the table owner, applies the
-- session tenant itself, and keeps the degree order.

CREATE OR REPLACE FUNCTION public.eq_graph_search_nodes(
    p_namespace text,
    p_query text,
    p_limit integer,
    p_entity_type text,
    p_tenant text,
    p_workspace text
) RETURNS TABLE(node_id text, properties jsonb, degree bigint)
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

    RETURN QUERY
    SELECT n.node_id, n.properties, n.degree
    FROM public.eq_graph_nodes n
    WHERE n.namespace = p_namespace
      AND (
            lower(n.node_id) LIKE '%' || p_query || '%'
            OR lower(COALESCE(n.properties ->> 'description', '')) LIKE '%' || p_query || '%'
          )
      AND (v_tenant IS NULL OR n.tenant_id = v_tenant)
      AND (v_workspace IS NULL OR n.workspace_id = v_workspace)
      AND (
            p_entity_type IS NULL
            OR p_entity_type = ''
            OR upper(COALESCE(n.entity_type, '')) = upper(p_entity_type)
          )
    ORDER BY n.degree DESC, n.node_id
    LIMIT p_limit;
END;
$fn$;

REVOKE ALL ON FUNCTION public.eq_graph_search_nodes(text, text, integer, text, text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.eq_graph_search_nodes(text, text, integer, text, text, text) TO edgequake_tenant_access;
