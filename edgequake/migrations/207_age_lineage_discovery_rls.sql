-- Issue #478: document lineage discovery under forced AGE row security.
--
-- The tenant guard on AGE labels compares
-- agtype_to_json(properties)->>'tenant_id'. That expression is not leakproof,
-- so edgequake_tenant_access cannot use the lineage GIN / citation btree
-- indexes. The planner index-scans every node or edge in the tenant
-- (idx_*_tenant_id) and filters the probes afterwards. On a ~160k-node graph
-- that exceeds SOURCE_DISCOVERY_STATEMENT_TIMEOUT_MS (2s) and
-- GET /api/v1/lineage/documents/{id} returns STORAGE_DEADLINE_EXCEEDED.
--
-- These functions run as their owner. The migration role bypasses row
-- security (superuser or BYPASSRLS). Inside the function the session tenant
-- is applied as a normal filter, so the GIN and citation btree indexes are
-- index conditions. When the caller has SET ROLE edgequake_tenant_access and
-- app.current_tenant_id is unset, the functions return no rows.

CREATE OR REPLACE FUNCTION public.eq_age_lineage_sql(
    p_kind text,
    p_graph text,
    p_tenant text,
    p_workspace text
) RETURNS text
LANGUAGE plpgsql
STABLE
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_eq boolean;
    v_src text;
    v_tgt text;
    v_tenant_pred text;
    v_ws_pred text;
    v_alias text;
BEGIN
    IF p_graph IS NULL OR p_graph !~ '^[A-Za-z_][A-Za-z0-9_]*$' THEN
        RAISE EXCEPTION 'refusing unsafe graph name';
    END IF;
    IF p_kind NOT IN ('nodes', 'edges', 'edges_singular') THEN
        RAISE EXCEPTION 'unknown lineage query';
    END IF;

    v_alias := CASE WHEN p_kind = 'nodes' THEN 'v' ELSE 'e' END;
    v_tenant_pred := CASE
        WHEN p_tenant IS NULL THEN 'TRUE'
        ELSE format(
            'ag_catalog.agtype_to_json(%I.properties)->>''tenant_id'' = $4',
            v_alias
        )
    END;
    v_ws_pred := CASE
        WHEN p_workspace IS NULL THEN 'TRUE'
        ELSE format(
            'ag_catalog.agtype_to_json(%I.properties)->>''workspace_id'' = $5',
            v_alias
        )
    END;

    IF p_kind = 'nodes' THEN
        RETURN format($q$
            WITH probes AS MATERIALIZED (
              SELECT probe_id FROM unnest($1::text[]) AS t(probe_id)
              UNION
              SELECT (p.prefix || gs.i::text) AS probe_id
              FROM unnest($2::text[]) AS p(prefix)
              CROSS JOIN generate_series(0, $3::int - 1) AS gs(i)
            ),
            probe_ids AS MATERIALIZED (SELECT array_agg(probe_id) AS ids FROM probes)
            SELECT ag_catalog.agtype_to_json(v.properties) AS props
            FROM %I.%I v
            CROSS JOIN probe_ids p
            WHERE %s
              AND %s
              AND (
                ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids') ?| p.ids
                OR ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_chunk_ids') ?| p.ids
              )
            LIMIT 5000
        $q$, p_graph, 'Node', v_tenant_pred, v_ws_pred);
    END IF;

    SELECT
        EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = p_graph AND table_name = 'EDGE' AND column_name = 'eq_source_id'
        )
        AND EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = p_graph AND table_name = 'EDGE' AND column_name = 'eq_target_id'
        )
    INTO v_eq;
    v_src := 'ag_catalog.agtype_to_json(e.properties)->>''source_id''';
    v_tgt := 'ag_catalog.agtype_to_json(e.properties)->>''target_id''';
    IF v_eq THEN
        v_src := 'COALESCE(e.eq_source_id, ' || v_src || ')';
        v_tgt := 'COALESCE(e.eq_target_id, ' || v_tgt || ')';
    END IF;

    IF p_kind = 'edges' THEN
        RETURN format($q$
            WITH probes AS MATERIALIZED (
              SELECT probe_id FROM unnest($1::text[]) AS t(probe_id)
              UNION
              SELECT (p.prefix || gs.i::text) AS probe_id
              FROM unnest($2::text[]) AS p(prefix)
              CROSS JOIN generate_series(0, $3::int - 1) AS gs(i)
            ),
            probe_ids AS MATERIALIZED (SELECT array_agg(probe_id) AS ids FROM probes)
            SELECT ag_catalog.agtype_to_json(e.properties) AS props,
                   %s AS source_id,
                   %s AS target_id
            FROM %I.%I e
            CROSS JOIN probe_ids p
            WHERE %s
              AND %s
              AND %s IS NOT NULL
              AND %s IS NOT NULL
              AND (
                ((ag_catalog.agtype_to_json(e.properties))::jsonb -> 'source_ids') ?| p.ids
                OR ((ag_catalog.agtype_to_json(e.properties))::jsonb -> 'source_chunk_ids') ?| p.ids
              )
            LIMIT 5000
        $q$, v_src, v_tgt, p_graph, 'EDGE', v_tenant_pred, v_ws_pred, v_src, v_tgt);
    END IF;

    -- OFFSET 0 keeps the citation equality as the index condition. Without the
    -- fence the planner flattens the join and sequential-scans the label.
    RETURN format($q$
        WITH probes AS MATERIALIZED (
          SELECT probe_id FROM unnest($1::text[]) AS t(probe_id)
          UNION
          SELECT (p.prefix || gs.i::text) AS probe_id
          FROM unnest($2::text[]) AS p(prefix)
          CROSS JOIN generate_series(0, $3::int - 1) AS gs(i)
        )
        SELECT hit.props, hit.source_id, hit.target_id
        FROM (
            SELECT props, source_id, target_id
            FROM probes pr
            JOIN LATERAL (
              SELECT ag_catalog.agtype_to_json(e.properties) AS props,
                     %s AS source_id,
                     %s AS target_id
              FROM %I.%I e
              WHERE ag_catalog.agtype_to_json(e.properties)->>'source_chunk_id' = pr.probe_id
                AND %s
                AND %s
                AND %s IS NOT NULL
                AND %s IS NOT NULL
              OFFSET 0
            ) hit ON true
            UNION ALL
            SELECT props, source_id, target_id
            FROM probes pr
            JOIN LATERAL (
              SELECT ag_catalog.agtype_to_json(e.properties) AS props,
                     %s AS source_id,
                     %s AS target_id
              FROM %I.%I e
              WHERE ag_catalog.agtype_to_json(e.properties)->>'source_document_id' = pr.probe_id
                AND %s
                AND %s
                AND %s IS NOT NULL
                AND %s IS NOT NULL
              OFFSET 0
            ) hit ON true
        ) hit
        LIMIT 5000
    $q$,
        v_src, v_tgt, p_graph, 'EDGE', v_tenant_pred, v_ws_pred, v_src, v_tgt,
        v_src, v_tgt, p_graph, 'EDGE', v_tenant_pred, v_ws_pred, v_src, v_tgt);
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_age_lineage_scope(
    OUT tenant text,
    OUT workspace text,
    OUT blocked boolean
)
LANGUAGE plpgsql
STABLE
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_role text := NULLIF(current_setting('role', true), '');
BEGIN
    tenant := NULLIF(current_setting('app.current_tenant_id', true), '');
    workspace := NULLIF(current_setting('app.current_workspace_id', true), '');
    -- SET ROLE is visible via the role GUC even inside SECURITY DEFINER.
    -- An empty tenant under the tenant role must not fall open to every graph.
    blocked := v_role = 'edgequake_tenant_access' AND tenant IS NULL;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_age_lineage_require_bypass()
RETURNS void
LANGUAGE plpgsql
STABLE
SET search_path = pg_catalog, public
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_roles
        WHERE rolname = current_user AND (rolsuper OR rolbypassrls)
    ) THEN
        RAISE EXCEPTION 'eq_age_lineage lookup must be owned by a role that bypasses row security';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_age_lineage_discover_nodes(
    p_graph text,
    p_exact text[],
    p_prefixes text[],
    p_probe_limit integer
) RETURNS TABLE(props json)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_scope record;
    v_limit integer := LEAST(GREATEST(COALESCE(p_probe_limit, 1), 1), 256);
BEGIN
    PERFORM public.eq_age_lineage_require_bypass();
    SELECT * INTO v_scope FROM public.eq_age_lineage_scope();
    IF v_scope.blocked THEN
        RETURN;
    END IF;
    RETURN QUERY EXECUTE public.eq_age_lineage_sql('nodes', p_graph, v_scope.tenant, v_scope.workspace)
        USING COALESCE(p_exact, ARRAY[]::text[]),
              COALESCE(p_prefixes, ARRAY[]::text[]),
              v_limit,
              v_scope.tenant,
              v_scope.workspace;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_age_lineage_discover_edges(
    p_graph text,
    p_exact text[],
    p_prefixes text[],
    p_probe_limit integer
) RETURNS TABLE(props json, source_id text, target_id text)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_scope record;
    v_limit integer := LEAST(GREATEST(COALESCE(p_probe_limit, 1), 1), 256);
BEGIN
    PERFORM public.eq_age_lineage_require_bypass();
    SELECT * INTO v_scope FROM public.eq_age_lineage_scope();
    IF v_scope.blocked THEN
        RETURN;
    END IF;
    RETURN QUERY EXECUTE public.eq_age_lineage_sql('edges', p_graph, v_scope.tenant, v_scope.workspace)
        USING COALESCE(p_exact, ARRAY[]::text[]),
              COALESCE(p_prefixes, ARRAY[]::text[]),
              v_limit,
              v_scope.tenant,
              v_scope.workspace;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_age_lineage_discover_edges_singular(
    p_graph text,
    p_exact text[],
    p_prefixes text[],
    p_probe_limit integer
) RETURNS TABLE(props json, source_id text, target_id text)
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_scope record;
    v_limit integer := LEAST(GREATEST(COALESCE(p_probe_limit, 1), 1), 256);
BEGIN
    PERFORM public.eq_age_lineage_require_bypass();
    SELECT * INTO v_scope FROM public.eq_age_lineage_scope();
    IF v_scope.blocked THEN
        RETURN;
    END IF;
    RETURN QUERY EXECUTE public.eq_age_lineage_sql('edges_singular', p_graph, v_scope.tenant, v_scope.workspace)
        USING COALESCE(p_exact, ARRAY[]::text[]),
              COALESCE(p_prefixes, ARRAY[]::text[]),
              v_limit,
              v_scope.tenant,
              v_scope.workspace;
END;
$$;

-- Inner plan of the lookup the tenant role actually runs. EXPLAIN of the
-- function call itself is only a Function Scan; this returns the index plan.
CREATE OR REPLACE FUNCTION public.eq_age_lineage_explain(
    p_kind text,
    p_graph text,
    p_exact text[],
    p_prefixes text[],
    p_probe_limit integer
) RETURNS text
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, public
AS $$
DECLARE
    v_scope record;
    v_limit integer := LEAST(GREATEST(COALESCE(p_probe_limit, 1), 1), 256);
    v_sql text;
    v_line text;
    v_out text := '';
BEGIN
    PERFORM public.eq_age_lineage_require_bypass();
    SELECT * INTO v_scope FROM public.eq_age_lineage_scope();
    IF v_scope.blocked THEN
        RETURN 'blocked';
    END IF;
    v_sql := public.eq_age_lineage_sql(p_kind, p_graph, v_scope.tenant, v_scope.workspace);
    FOR v_line IN EXECUTE 'EXPLAIN (ANALYZE, BUFFERS) ' || v_sql
        USING COALESCE(p_exact, ARRAY[]::text[]),
              COALESCE(p_prefixes, ARRAY[]::text[]),
              v_limit,
              v_scope.tenant,
              v_scope.workspace
    LOOP
        v_out := v_out || v_line || E'\n';
    END LOOP;
    RETURN v_out;
END;
$$;

REVOKE ALL ON FUNCTION public.eq_age_lineage_sql(text, text, text, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.eq_age_lineage_scope() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.eq_age_lineage_require_bypass() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.eq_age_lineage_discover_nodes(text, text[], text[], integer) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.eq_age_lineage_discover_edges(text, text[], text[], integer) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.eq_age_lineage_discover_edges_singular(text, text[], text[], integer) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.eq_age_lineage_explain(text, text, text[], text[], integer) FROM PUBLIC;

GRANT EXECUTE ON FUNCTION public.eq_age_lineage_discover_nodes(text, text[], text[], integer) TO edgequake_tenant_access;
GRANT EXECUTE ON FUNCTION public.eq_age_lineage_discover_edges(text, text[], text[], integer) TO edgequake_tenant_access;
GRANT EXECUTE ON FUNCTION public.eq_age_lineage_discover_edges_singular(text, text[], text[], integer) TO edgequake_tenant_access;
GRANT EXECUTE ON FUNCTION public.eq_age_lineage_explain(text, text, text[], text[], integer) TO edgequake_tenant_access;
