-- Scoped DAL transactions SET LOCAL ROLE to this non-login, non-bypass role.
-- Administration/queue connections retain their explicitly privileged path.
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'edgequake_tenant_access') THEN
        CREATE ROLE edgequake_tenant_access NOLOGIN NOSUPERUSER NOBYPASSRLS NOINHERIT;
    END IF;
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'edgequake_tenant_access'
               AND (rolsuper OR rolbypassrls OR rolcanlogin)) THEN
        RAISE EXCEPTION 'edgequake_tenant_access must be NOLOGIN NOSUPERUSER NOBYPASSRLS';
    END IF;
END $$;
GRANT edgequake_tenant_access TO CURRENT_USER;
GRANT USAGE ON SCHEMA public, ag_catalog, edgequake TO edgequake_tenant_access;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public, edgequake TO edgequake_tenant_access;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public, edgequake TO edgequake_tenant_access;

-- A restrictive policy cannot be OR'ed away by older or installation-specific policies.
-- Tenant-wide reads are retained for authorized tenant operations; WITH CHECK uses
-- the identical predicate, so workspace-bound writes cannot escape their workspace.
DO $$
DECLARE t text; predicate text; workspace_setting text;
BEGIN
    FOREACH t IN ARRAY ARRAY[
        'documents','chunks','entities','relationships','conversation_history',
        'conversations','folders','tasks','tasks_history','failed_chunks',
        'data_bindings','mutation_requests','object_revisions','ingest_batches',
        'graph_contributions','embedding_manifests','embedding_projections',
        'projection_events','projection_visibility','projection_cleanup_intents',
        'vector_provider_cutovers','decision_cache','decision_review'
    ] LOOP
        IF to_regclass(format('public.%I', t)) IS NULL THEN CONTINUE; END IF;
        predicate := 'tenant_id = public.current_tenant_id() AND (public.current_workspace_id() IS NULL OR workspace_id = public.current_workspace_id())';
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('CREATE POLICY tenant_access_allow ON public.%I FOR ALL TO edgequake_tenant_access USING (true) WITH CHECK (true)', t);
        EXECUTE format('CREATE POLICY tenant_access_guard ON public.%I AS RESTRICTIVE FOR ALL TO PUBLIC USING (%s) WITH CHECK (%s)', t, predicate, predicate);
    END LOOP;
    -- Workspace-only content still needs tenant validation. A mismatched tenant/workspace
    -- pair must never grant access merely because the workspace UUID is known.
    FOREACH t IN ARRAY ARRAY[
        'document_originals','pdf_documents','document_pages','document_mm_assets',
        'page_layout_regions','document_page_states','chunk_embeddings',
        'entity_embeddings','relationship_embeddings','report_embeddings',
        'chunk_entity_links','chunk_relation_links'
    ] LOOP
        IF to_regclass(format('public.%I', t)) IS NULL THEN CONTINUE; END IF;
        SELECT CASE WHEN a.atttypid = 'uuid'::regtype THEN 'public.current_workspace_id()'
               ELSE 'public.current_workspace_id()::text' END INTO workspace_setting
          FROM pg_attribute a WHERE a.attrelid=to_regclass(format('public.%I',t)) AND a.attname='workspace_id';
        predicate := format('workspace_id = %s AND EXISTS (SELECT 1 FROM public.workspaces w WHERE w.workspace_id = public.current_workspace_id() AND w.tenant_id = public.current_tenant_id())', workspace_setting);
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('CREATE POLICY tenant_access_allow ON public.%I FOR ALL TO edgequake_tenant_access USING (true) WITH CHECK (true)', t);
        EXECUTE format('CREATE POLICY tenant_access_guard ON public.%I AS RESTRICTIVE FOR ALL TO PUBLIC USING (%s) WITH CHECK (%s)', t, predicate, predicate);
    END LOOP;
END $$;

-- Direct child-table reads do not inherit a partition parent's policies.
DO $$ DECLARE child record; predicate text; BEGIN
    FOR child IN SELECT c.relname FROM pg_inherits i JOIN pg_class c ON c.oid=i.inhrelid
        JOIN pg_class p ON p.oid=i.inhparent JOIN pg_namespace n ON n.oid=c.relnamespace
        WHERE n.nspname='public' AND p.relname IN ('tasks','tasks_history') LOOP
        IF EXISTS (SELECT 1 FROM pg_policies WHERE schemaname='public' AND tablename=child.relname AND policyname='tenant_access_guard') THEN CONTINUE; END IF;
        predicate := 'tenant_id = public.current_tenant_id() AND (public.current_workspace_id() IS NULL OR workspace_id = public.current_workspace_id())';
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', child.relname);
        EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY', child.relname);
        EXECUTE format('CREATE POLICY tenant_access_allow ON public.%I FOR ALL TO edgequake_tenant_access USING (true) WITH CHECK (true)', child.relname);
        EXECUTE format('CREATE POLICY tenant_access_guard ON public.%I AS RESTRICTIVE FOR ALL TO PUBLIC USING (%s) WITH CHECK (%s)', child.relname,predicate,predicate);
    END LOOP;
END $$;

-- Provider receipts/manifests inherit the scope of their immutable parent event.
DO $$
DECLARE t text; predicate text;
BEGIN
    FOREACH t IN ARRAY ARRAY['projection_deliveries','projection_event_manifests'] LOOP
        IF to_regclass(format('public.%I', t)) IS NULL THEN CONTINUE; END IF;
        predicate := format('EXISTS (SELECT 1 FROM public.projection_events e WHERE e.event_id = %I.event_id AND e.tenant_id = public.current_tenant_id() AND e.workspace_id = public.current_workspace_id())', t);
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('CREATE POLICY tenant_access_allow ON public.%I FOR ALL TO edgequake_tenant_access USING (true) WITH CHECK (true)', t);
        EXECUTE format('CREATE POLICY tenant_access_guard ON public.%I AS RESTRICTIVE FOR ALL TO PUBLIC USING (%s) WITH CHECK (%s)', t, predicate, predicate);
    END LOOP;
END $$;

-- AGE inheritance does not propagate policies to child labels. Install on every label.
-- Invoker security is deliberate: only the graph/table owner may provision policies.
CREATE FUNCTION public.install_graph_tenant_rls(graph_name text) RETURNS void
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
DECLARE label record; predicate text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM ag_catalog.ag_graph WHERE name = graph_name) THEN
        RAISE EXCEPTION 'Unknown AGE graph';
    END IF;
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO edgequake_tenant_access', graph_name);
    FOR label IN SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
        WHERE n.nspname=graph_name AND c.relkind='r' AND EXISTS (
            SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname='properties' AND NOT a.attisdropped
        ) LOOP
        EXECUTE format('ALTER TABLE %I.%I ENABLE ROW LEVEL SECURITY', graph_name, label.relname);
        EXECUTE format('ALTER TABLE %I.%I FORCE ROW LEVEL SECURITY', graph_name, label.relname);
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %I.%I TO edgequake_tenant_access', graph_name, label.relname);
        predicate := 'ag_catalog.agtype_to_json(properties)->>''tenant_id'' = public.current_tenant_id()::text AND (public.current_workspace_id() IS NULL OR ag_catalog.agtype_to_json(properties)->>''workspace_id'' = public.current_workspace_id()::text)';
        IF NOT EXISTS (SELECT 1 FROM pg_policies WHERE schemaname=graph_name AND tablename=label.relname AND policyname='tenant_access_guard') THEN
            EXECUTE format('CREATE POLICY tenant_access_allow ON %I.%I FOR ALL TO edgequake_tenant_access USING (true) WITH CHECK (true)', graph_name, label.relname);
            EXECUTE format('CREATE POLICY tenant_access_guard ON %I.%I AS RESTRICTIVE FOR ALL TO PUBLIC USING (%s) WITH CHECK (%s)', graph_name, label.relname, predicate, predicate);
        END IF;
    END LOOP;
END $$;
DO $$ DECLARE g record; BEGIN
    FOR g IN SELECT name FROM ag_catalog.ag_graph LOOP
        PERFORM public.install_graph_tenant_rls(g.name);
    END LOOP;
END $$;
