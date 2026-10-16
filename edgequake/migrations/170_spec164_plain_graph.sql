-- SPEC-164 / v0.35 expand: plain adjacency tables beside Apache AGE.
-- No DROP. Mirror triggers copy committed AGE rows. Ledger stays "age"
-- until backfill + shadow (eq_graph_shadow_mismatches) mark it ready.
-- Greenfield (no ag_catalog): ledger is ready immediately (S8).

CREATE TABLE IF NOT EXISTS public.eq_graph_namespace (
    graph_name text PRIMARY KEY,
    namespace  text NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS eq_graph_namespace_ns_uidx
    ON public.eq_graph_namespace (namespace);

CREATE TABLE IF NOT EXISTS public.eq_graph_nodes (
    namespace    text NOT NULL,
    node_id      text NOT NULL,
    tenant_id    text NOT NULL DEFAULT '',
    workspace_id text NOT NULL DEFAULT '',
    entity_type  text,
    properties   jsonb NOT NULL DEFAULT '{}'::jsonb,
    -- Incident degree (source UNION ALL target, so a self-loop counts twice).
    -- Maintained by statement triggers. Top-k popular/search then stops after
    -- LIMIT rows instead of aggregating every edge.
    degree       bigint NOT NULL DEFAULT 0,
    PRIMARY KEY (namespace, node_id)
);

CREATE TABLE IF NOT EXISTS public.eq_graph_edges (
    namespace    text NOT NULL,
    source_id    text NOT NULL,
    target_id    text NOT NULL,
    rel_type     text NOT NULL DEFAULT '',
    tenant_id    text NOT NULL DEFAULT '',
    workspace_id text NOT NULL DEFAULT '',
    weight       double precision,
    properties   jsonb NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (namespace, source_id, target_id, rel_type)
);

CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_source
    ON public.eq_graph_edges (namespace, source_id);
CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_target
    ON public.eq_graph_edges (namespace, target_id);
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_tenant_ws
    ON public.eq_graph_nodes (namespace, tenant_id, workspace_id);
CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_tenant_ws
    ON public.eq_graph_edges (namespace, tenant_id, workspace_id);
-- jsonb_ops + 256 kB pending list, same bound as AGE lineage GINs (migration 157).
-- A default 4 MB pending list makes source-prefix `?|` several times slower
-- than the AGE index after a bulk load.
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_source_ids_gin
    ON public.eq_graph_nodes USING gin ((properties -> 'source_ids') jsonb_ops)
    WITH (gin_pending_list_limit = 256);
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_source_chunk_ids_gin
    ON public.eq_graph_nodes USING gin ((properties -> 'source_chunk_ids') jsonb_ops)
    WITH (gin_pending_list_limit = 256);
CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_source_ids_gin
    ON public.eq_graph_edges USING gin ((properties -> 'source_ids') jsonb_ops)
    WITH (gin_pending_list_limit = 256);
CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_source_chunk_ids_gin
    ON public.eq_graph_edges USING gin ((properties -> 'source_chunk_ids') jsonb_ops)
    WITH (gin_pending_list_limit = 256);

-- (namespace, degree DESC, node_id) is the popular/search access path:
-- ORDER BY degree DESC, node_id LIMIT k is an index scan that stops at k.
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_degree
    ON public.eq_graph_nodes (namespace, degree DESC, node_id);

-- Leading-wildcard LIKE cannot use a btree. A short pattern still walks the
-- degree index and stops at LIMIT; a selective pattern uses these GIN indexes.
-- pg_trgm is trusted contrib (PostgreSQL 13+): RDS, Aurora, and Azure Flexible
-- Server allow CREATE EXTENSION without shared_preload_libraries. It does not
-- require Apache AGE. Pin the extension to public so it is not created inside
-- ag_catalog (migration 015 follows search_path; 171 drops that schema).
DO $$
DECLARE
    ext_schema text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_extension WHERE extname = 'pg_trgm') THEN
        CREATE EXTENSION pg_trgm WITH SCHEMA public;
    END IF;
    SELECT n.nspname INTO ext_schema
    FROM pg_catalog.pg_extension e
    JOIN pg_catalog.pg_namespace n ON n.oid = e.extnamespace
    WHERE e.extname = 'pg_trgm';
    IF ext_schema IS DISTINCT FROM 'public' THEN
        ALTER EXTENSION pg_trgm SET SCHEMA public;
    END IF;
END $$;
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_id_trgm
    ON public.eq_graph_nodes USING gin (lower(node_id) public.gin_trgm_ops)
    WITH (gin_pending_list_limit = 256);
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_desc_trgm
    ON public.eq_graph_nodes USING gin (lower(COALESCE(properties->>'description', '')) public.gin_trgm_ops)
    WITH (gin_pending_list_limit = 256);
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_type_trgm
    ON public.eq_graph_nodes USING gin (lower(entity_type) public.gin_trgm_ops)
    WITH (gin_pending_list_limit = 256)
    WHERE entity_type IS NOT NULL;

-- Case-insensitive type pages. UPPER(COALESCE(entity_type,'')) is not sargable:
-- a missing type walks every node in primary-key order before LIMIT can stop.
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_type_upper
    ON public.eq_graph_nodes (namespace, upper(entity_type), node_id)
    WHERE entity_type IS NOT NULL;

-- Label ranks and distinct types. Index-only; the heap stays out of the aggregate.
CREATE INDEX IF NOT EXISTS idx_eq_graph_nodes_ws_type
    ON public.eq_graph_nodes (namespace, workspace_id, entity_type)
    WHERE entity_type IS NOT NULL AND entity_type <> '';

-- Relationship lookup is an equality on properties->>'id' or source_id||'_'||target_id.
-- Either expression without an index is a sequential scan of every edge (LIMIT 1
-- does not stop early on a miss).
CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_prop_id
    ON public.eq_graph_edges (namespace, (properties->>'id'));
CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_pair_key
    ON public.eq_graph_edges (namespace, (source_id || '_' || target_id));

-- rel_type pages. The primary key orders by source_id first, so a selective
-- type still read every edge in the namespace.
CREATE INDEX IF NOT EXISTS idx_eq_graph_edges_rel
    ON public.eq_graph_edges (namespace, rel_type, source_id, target_id);

CREATE OR REPLACE FUNCTION public.eq_graph_degree_skip()
RETURNS boolean
LANGUAGE sql STABLE AS $$
    SELECT current_setting('eq.skip_graph_degree', true) = 'on'
$$;

-- One aggregate per INSERT statement. A row trigger would update two nodes
-- per edge (1M updates on a 500k bulk load).
CREATE OR REPLACE FUNCTION public.eq_graph_degree_on_insert()
RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF public.eq_graph_degree_skip() THEN
        RETURN NULL;
    END IF;
    UPDATE public.eq_graph_nodes n
       SET degree = n.degree + d.delta
      FROM (
        SELECT namespace, node_id, COUNT(*)::bigint AS delta
        FROM (
            SELECT namespace, source_id AS node_id FROM new_edges
            UNION ALL
            SELECT namespace, target_id FROM new_edges
        ) s
        GROUP BY namespace, node_id
      ) d
     WHERE n.namespace = d.namespace AND n.node_id = d.node_id;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_graph_degree_on_delete()
RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF public.eq_graph_degree_skip() THEN
        RETURN NULL;
    END IF;
    UPDATE public.eq_graph_nodes n
       SET degree = n.degree - d.delta
      FROM (
        SELECT namespace, node_id, COUNT(*)::bigint AS delta
        FROM (
            SELECT namespace, source_id AS node_id FROM old_edges
            UNION ALL
            SELECT namespace, target_id FROM old_edges
        ) s
        GROUP BY namespace, node_id
      ) d
     WHERE n.namespace = d.namespace AND n.node_id = d.node_id;
    RETURN NULL;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_graph_degree_on_update()
RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF public.eq_graph_degree_skip() THEN
        RETURN NULL;
    END IF;
    UPDATE public.eq_graph_nodes n
       SET degree = n.degree + d.delta
      FROM (
        SELECT namespace, node_id, SUM(delta)::bigint AS delta
        FROM (
            SELECT namespace, source_id AS node_id, -1::bigint AS delta FROM old_edges
            UNION ALL
            SELECT namespace, target_id, -1 FROM old_edges
            UNION ALL
            SELECT namespace, source_id, 1 FROM new_edges
            UNION ALL
            SELECT namespace, target_id, 1 FROM new_edges
        ) s
        GROUP BY namespace, node_id
        HAVING SUM(delta) <> 0
      ) d
     WHERE n.namespace = d.namespace AND n.node_id = d.node_id;
    RETURN NULL;
END;
$$;

-- Edges can be mirrored before their node row. Index probes, not a scan of every edge.
CREATE OR REPLACE FUNCTION public.eq_graph_degree_on_node_insert()
RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF public.eq_graph_degree_skip() THEN
        RETURN NULL;
    END IF;
    UPDATE public.eq_graph_nodes n
       SET degree = sub.degree
      FROM (
        SELECT nn.namespace, nn.node_id,
               (SELECT COUNT(*)::bigint FROM public.eq_graph_edges e
                 WHERE e.namespace = nn.namespace AND e.source_id = nn.node_id)
             + (SELECT COUNT(*)::bigint FROM public.eq_graph_edges e
                 WHERE e.namespace = nn.namespace AND e.target_id = nn.node_id) AS degree
        FROM new_nodes nn
      ) sub
     WHERE n.namespace = sub.namespace
       AND n.node_id = sub.node_id
       AND sub.degree IS DISTINCT FROM n.degree;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS eq_graph_degree_insert ON public.eq_graph_edges;
CREATE TRIGGER eq_graph_degree_insert
    AFTER INSERT ON public.eq_graph_edges
    REFERENCING NEW TABLE AS new_edges
    FOR EACH STATEMENT EXECUTE FUNCTION public.eq_graph_degree_on_insert();

DROP TRIGGER IF EXISTS eq_graph_degree_delete ON public.eq_graph_edges;
CREATE TRIGGER eq_graph_degree_delete
    AFTER DELETE ON public.eq_graph_edges
    REFERENCING OLD TABLE AS old_edges
    FOR EACH STATEMENT EXECUTE FUNCTION public.eq_graph_degree_on_delete();

DROP TRIGGER IF EXISTS eq_graph_degree_update ON public.eq_graph_edges;
CREATE TRIGGER eq_graph_degree_update
    AFTER UPDATE ON public.eq_graph_edges
    REFERENCING OLD TABLE AS old_edges NEW TABLE AS new_edges
    FOR EACH STATEMENT EXECUTE FUNCTION public.eq_graph_degree_on_update();

DROP TRIGGER IF EXISTS eq_graph_degree_node_insert ON public.eq_graph_nodes;
CREATE TRIGGER eq_graph_degree_node_insert
    AFTER INSERT ON public.eq_graph_nodes
    REFERENCING NEW TABLE AS new_nodes
    FOR EACH STATEMENT EXECUTE FUNCTION public.eq_graph_degree_on_node_insert();

-- Absolute repair after a bulk copy that set eq.skip_graph_degree.
CREATE OR REPLACE FUNCTION public.eq_graph_recompute_degrees(p_namespace text)
RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    UPDATE public.eq_graph_nodes n
       SET degree = d.degree
      FROM (
        SELECT node_id, COUNT(*)::bigint AS degree
        FROM (
            SELECT source_id AS node_id
            FROM public.eq_graph_edges
            WHERE namespace = p_namespace
            UNION ALL
            SELECT target_id
            FROM public.eq_graph_edges
            WHERE namespace = p_namespace
        ) ends
        GROUP BY node_id
      ) d
     WHERE n.namespace = p_namespace
       AND n.node_id = d.node_id
       AND n.degree IS DISTINCT FROM d.degree;

    UPDATE public.eq_graph_nodes n
       SET degree = 0
     WHERE n.namespace = p_namespace
       AND n.degree <> 0
       AND NOT EXISTS (
           SELECT 1 FROM public.eq_graph_edges e
           WHERE e.namespace = p_namespace
             AND (e.source_id = n.node_id OR e.target_id = n.node_id)
       );
END;
$$;

CREATE TABLE IF NOT EXISTS public.eq_graph_cutover (
    id                   smallint PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    status               text NOT NULL DEFAULT 'age'
        CHECK (status IN ('age', 'backfilling', 'ready')),
    age_absent           boolean NOT NULL DEFAULT false,
    shadow_ok            boolean NOT NULL DEFAULT false,
    backfill_complete    boolean NOT NULL DEFAULT false,
    unbackfilled_graphs  integer NOT NULL DEFAULT 0,
    updated_at           timestamptz NOT NULL DEFAULT now(),
    detail               jsonb NOT NULL DEFAULT '{}'::jsonb
);

INSERT INTO public.eq_graph_cutover (id)
VALUES (1)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS public.eq_graph_backfill (
    graph_name      text PRIMARY KEY,
    namespace       text NOT NULL,
    nodes_copied    bigint NOT NULL DEFAULT 0,
    edges_copied    bigint NOT NULL DEFAULT 0,
    orphans_skipped bigint NOT NULL DEFAULT 0,
    shadow_ok       boolean NOT NULL DEFAULT false,
    finished_at     timestamptz
);

-- Depth-bounded neighbor id set on plain tables (product caps: depth ≤ 3, K ≤ 500).
CREATE OR REPLACE FUNCTION public.eq_graph_neighbor_ids_plain(
    p_namespace text,
    p_start text,
    p_depth int,
    p_cap int
) RETURNS text[]
LANGUAGE plpgsql STABLE AS $$
DECLARE
    visited text[] := ARRAY[p_start];
    frontier text[] := ARRAY[p_start];
    next_f text[];
    nbr text;
    d int;
BEGIN
    IF p_start IS NULL OR p_depth < 1 THEN
        RETURN ARRAY[]::text[];
    END IF;
    FOR d IN 1..LEAST(p_depth, 3) LOOP
        next_f := ARRAY[]::text[];
        FOR nbr IN
            SELECT endpoint
            FROM (
                SELECT e.target_id AS endpoint
                FROM public.eq_graph_edges e
                WHERE e.namespace = p_namespace AND e.source_id = ANY(frontier)
                UNION
                SELECT e.source_id
                FROM public.eq_graph_edges e
                WHERE e.namespace = p_namespace AND e.target_id = ANY(frontier)
            ) s
            WHERE NOT (endpoint = ANY(visited))
            ORDER BY endpoint
        LOOP
            EXIT WHEN cardinality(visited) >= p_cap;
            visited := visited || nbr;
            next_f := next_f || nbr;
        END LOOP;
        frontier := next_f;
        EXIT WHEN cardinality(frontier) = 0 OR cardinality(visited) >= p_cap;
    END LOOP;
    RETURN ARRAY(SELECT x FROM unnest(visited) AS x ORDER BY x);
END;
$$;

-- Same hop shape on an AGE "EDGE" child table. Dynamic graph name is an identifier.
CREATE OR REPLACE FUNCTION public.eq_graph_neighbor_ids_age(
    p_graph text,
    p_start text,
    p_depth int,
    p_cap int
) RETURNS text[]
LANGUAGE plpgsql STABLE AS $$
DECLARE
    visited text[] := ARRAY[p_start];
    frontier text[] := ARRAY[p_start];
    next_f text[];
    nbr text;
    d int;
BEGIN
    IF p_graph !~ '^[A-Za-z_][A-Za-z0-9_]*$' THEN
        RAISE EXCEPTION 'refusing unsafe graph name';
    END IF;
    IF p_start IS NULL OR to_regclass(format('%I."EDGE"', p_graph)) IS NULL THEN
        RETURN ARRAY(SELECT x FROM unnest(visited) AS x ORDER BY x);
    END IF;
    FOR d IN 1..LEAST(p_depth, 3) LOOP
        next_f := ARRAY[]::text[];
        FOR nbr IN EXECUTE format(
            $q$
            SELECT endpoint FROM (
                SELECT COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id') AS endpoint
                FROM %I."EDGE" e
                WHERE COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id') = ANY($1)
                UNION
                SELECT COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id')
                FROM %I."EDGE" e
                WHERE COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id') = ANY($1)
            ) s
            WHERE endpoint IS NOT NULL AND NOT (endpoint = ANY($2))
            ORDER BY endpoint
            $q$, p_graph, p_graph)
        USING frontier, visited
        LOOP
            EXIT WHEN cardinality(visited) >= p_cap;
            visited := visited || nbr;
            next_f := next_f || nbr;
        END LOOP;
        frontier := next_f;
        EXIT WHEN cardinality(frontier) = 0 OR cardinality(visited) >= p_cap;
    END LOOP;
    RETURN ARRAY(SELECT x FROM unnest(visited) AS x ORDER BY x);
END;
$$;

-- H-164-02: 50 starts, depth 2, cap 500. Returns mismatch count.
CREATE OR REPLACE FUNCTION public.eq_graph_shadow_mismatches(
    p_namespace text,
    p_graph text
) RETURNS integer
LANGUAGE plpgsql STABLE AS $$
DECLARE
    s text;
    mismatches integer := 0;
    plain_ids text[];
    age_ids text[];
BEGIN
    FOR s IN
        SELECT node_id
        FROM public.eq_graph_nodes
        WHERE namespace = p_namespace
        ORDER BY md5(node_id || '164')
        LIMIT 50
    LOOP
        plain_ids := public.eq_graph_neighbor_ids_plain(p_namespace, s, 2, 500);
        age_ids := public.eq_graph_neighbor_ids_age(p_graph, s, 2, 500);
        IF plain_ids IS DISTINCT FROM age_ids THEN
            mismatches := mismatches + 1;
        END IF;
    END LOOP;
    RETURN mismatches;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_graph_mirror_node()
RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    props jsonb;
    nid text;
    ns text;
BEGIN
    IF TG_OP = 'DELETE' THEN
        props := ag_catalog.agtype_to_json(OLD.properties)::jsonb;
        nid := COALESCE(OLD.eq_node_id, props->>'node_id');
    ELSE
        props := ag_catalog.agtype_to_json(NEW.properties)::jsonb;
        nid := COALESCE(NEW.eq_node_id, props->>'node_id');
    END IF;
    SELECT namespace INTO ns FROM public.eq_graph_namespace WHERE graph_name = TG_TABLE_SCHEMA;
    IF ns IS NULL THEN
        ns := TG_TABLE_SCHEMA;
    END IF;
    IF nid IS NULL OR nid = '' THEN
        IF TG_OP = 'DELETE' THEN RETURN OLD; ELSE RETURN NEW; END IF;
    END IF;
    IF TG_OP = 'DELETE' THEN
        DELETE FROM public.eq_graph_nodes WHERE namespace = ns AND node_id = nid;
        DELETE FROM public.eq_graph_edges
         WHERE namespace = ns AND (source_id = nid OR target_id = nid);
        RETURN OLD;
    END IF;
    INSERT INTO public.eq_graph_nodes (
        namespace, node_id, tenant_id, workspace_id, entity_type, properties
    ) VALUES (
        ns,
        nid,
        COALESCE(props->>'tenant_id', ''),
        COALESCE(props->>'workspace_id', ''),
        NULLIF(props->>'entity_type', ''),
        props
    )
    ON CONFLICT (namespace, node_id) DO UPDATE SET
        tenant_id = EXCLUDED.tenant_id,
        workspace_id = EXCLUDED.workspace_id,
        entity_type = EXCLUDED.entity_type,
        properties = EXCLUDED.properties;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_graph_mirror_edge()
RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    props jsonb;
    ns text;
    src text;
    tgt text;
    rel text;
BEGIN
    IF TG_OP = 'DELETE' THEN
        props := ag_catalog.agtype_to_json(OLD.properties)::jsonb;
        src := COALESCE(OLD.eq_source_id, props->>'source_id');
        tgt := COALESCE(OLD.eq_target_id, props->>'target_id');
        rel := COALESCE(NULLIF(OLD.eq_rel_type, ''), NULLIF(props->>'relation_type', ''), '');
    ELSE
        props := ag_catalog.agtype_to_json(NEW.properties)::jsonb;
        src := COALESCE(NEW.eq_source_id, props->>'source_id');
        tgt := COALESCE(NEW.eq_target_id, props->>'target_id');
        rel := COALESCE(NULLIF(NEW.eq_rel_type, ''), NULLIF(props->>'relation_type', ''), '');
    END IF;
    SELECT namespace INTO ns FROM public.eq_graph_namespace WHERE graph_name = TG_TABLE_SCHEMA;
    IF ns IS NULL THEN
        ns := TG_TABLE_SCHEMA;
    END IF;
    IF src IS NULL OR tgt IS NULL THEN
        IF TG_OP = 'DELETE' THEN RETURN OLD; ELSE RETURN NEW; END IF;
    END IF;
    IF TG_OP = 'DELETE' THEN
        DELETE FROM public.eq_graph_edges
         WHERE namespace = ns AND source_id = src AND target_id = tgt AND rel_type = rel;
        RETURN OLD;
    END IF;
    INSERT INTO public.eq_graph_edges (
        namespace, source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties
    ) VALUES (
        ns, src, tgt, rel,
        COALESCE(props->>'tenant_id', ''),
        COALESCE(props->>'workspace_id', ''),
        CASE
            WHEN COALESCE(props->>'weight', '') ~ '^-?[0-9]+(\.[0-9]+)?$'
            THEN (props->>'weight')::double precision
            ELSE NULL
        END,
        props
    )
    ON CONFLICT (namespace, source_id, target_id, rel_type) DO UPDATE SET
        tenant_id = EXCLUDED.tenant_id,
        workspace_id = EXCLUDED.workspace_id,
        weight = EXCLUDED.weight,
        properties = EXCLUDED.properties;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_graph_install_mirror_triggers(
    p_graph text,
    p_namespace text
) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    IF p_graph !~ '^[A-Za-z_][A-Za-z0-9_]*$' THEN
        RAISE EXCEPTION 'refusing unsafe graph name';
    END IF;
    INSERT INTO public.eq_graph_namespace (graph_name, namespace)
    VALUES (p_graph, p_namespace)
    ON CONFLICT (graph_name) DO UPDATE SET namespace = EXCLUDED.namespace;

    IF to_regclass(format('%I."Node"', p_graph)) IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM information_schema.columns
           WHERE table_schema = p_graph AND table_name = 'Node' AND column_name = 'eq_node_id'
       ) THEN
        EXECUTE format('DROP TRIGGER IF EXISTS eq_graph_mirror_node ON %I."Node"', p_graph);
        EXECUTE format(
            'CREATE TRIGGER eq_graph_mirror_node AFTER INSERT OR UPDATE OR DELETE ON %I."Node" FOR EACH ROW EXECUTE FUNCTION public.eq_graph_mirror_node()',
            p_graph
        );
    END IF;

    IF to_regclass(format('%I."EDGE"', p_graph)) IS NOT NULL
       AND EXISTS (
           SELECT 1 FROM information_schema.columns
           WHERE table_schema = p_graph AND table_name = 'EDGE' AND column_name = 'eq_source_id'
       ) THEN
        EXECUTE format('DROP TRIGGER IF EXISTS eq_graph_mirror_edge ON %I."EDGE"', p_graph);
        EXECUTE format(
            'CREATE TRIGGER eq_graph_mirror_edge AFTER INSERT OR UPDATE OR DELETE ON %I."EDGE" FOR EACH ROW EXECUTE FUNCTION public.eq_graph_mirror_edge()',
            p_graph
        );
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION public.eq_graph_install_plain_rls()
RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
    t text;
    predicate text;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'edgequake_tenant_access') THEN
        RETURN;
    END IF;
    IF to_regprocedure('public.current_tenant_id()') IS NULL THEN
        RETURN;
    END IF;
    predicate := 'tenant_id = public.current_tenant_id()::text AND (public.current_workspace_id() IS NULL OR workspace_id = public.current_workspace_id()::text)';
    FOREACH t IN ARRAY ARRAY['eq_graph_nodes', 'eq_graph_edges'] LOOP
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON public.%I TO edgequake_tenant_access', t);
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE public.%I FORCE ROW LEVEL SECURITY', t);
        IF NOT EXISTS (
            SELECT 1 FROM pg_policies
            WHERE schemaname = 'public' AND tablename = t AND policyname = 'tenant_access_guard'
        ) THEN
            EXECUTE format(
                'CREATE POLICY tenant_access_allow ON public.%I FOR ALL TO edgequake_tenant_access USING (true) WITH CHECK (true)',
                t
            );
            EXECUTE format(
                'CREATE POLICY tenant_access_guard ON public.%I AS RESTRICTIVE FOR ALL TO PUBLIC USING (%s) WITH CHECK (%s)',
                t, predicate, predicate
            );
        END IF;
    END LOOP;
END;
$$;

SELECT public.eq_graph_install_plain_rls();

-- Seed namespace + triggers for graphs that already exist. Empty / no AGE → ready.
DO $$
DECLARE
    age_ns boolean;
    g record;
    ngraphs integer := 0;
    derived text;
BEGIN
    SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'ag_catalog') INTO age_ns;
    IF age_ns THEN
        BEGIN
            FOR g IN EXECUTE 'SELECT name::text AS name FROM ag_catalog.ag_graph' LOOP
                ngraphs := ngraphs + 1;
                IF g.name LIKE 'eq\_eq\_%\_graph' ESCAPE '\' THEN
                    derived := substring(g.name FROM 7 FOR char_length(g.name) - 12);
                ELSE
                    derived := g.name;
                END IF;
                PERFORM public.eq_graph_install_mirror_triggers(g.name, derived);
            END LOOP;
        EXCEPTION WHEN undefined_table OR invalid_schema_name THEN
            age_ns := false;
            ngraphs := 0;
        END;
    END IF;

    IF NOT age_ns OR ngraphs = 0 THEN
        UPDATE public.eq_graph_cutover
           SET status = 'ready',
               age_absent = NOT age_ns,
               shadow_ok = true,
               backfill_complete = true,
               unbackfilled_graphs = 0,
               updated_at = now(),
               detail = jsonb_build_object('reason', 'greenfield_or_empty')
         WHERE id = 1;
    ELSE
        UPDATE public.eq_graph_cutover
           SET status = 'age',
               age_absent = false,
               shadow_ok = false,
               backfill_complete = false,
               unbackfilled_graphs = ngraphs,
               updated_at = now(),
               detail = jsonb_build_object('reason', 'awaiting_backfill', 'graphs', ngraphs)
         WHERE id = 1;
    END IF;
END $$;
