-- SPEC-164 / v0.35 contract. Applied only with `edgequake migrate --confirm-drop`.
-- Refuses unless the cutover ledger is ready, shadow matched, and every AGE
-- graph was backfilled. Does not run from plain `edgequake migrate` (S1).

DO $$
DECLARE
    st text;
    shadow boolean;
    backfill boolean;
    pending integer := 0;
    nonempty bigint;
    g text;
    age_on boolean;
BEGIN
    SELECT status, shadow_ok, backfill_complete
      INTO st, shadow, backfill
      FROM public.eq_graph_cutover
     WHERE id = 1;

    IF st IS DISTINCT FROM 'ready' OR shadow IS NOT TRUE OR backfill IS NOT TRUE THEN
        RAISE EXCEPTION
            'SPEC-164 confirm-drop refused: cutover guard is red (status=%, shadow=%, backfill=%)',
            st, shadow, backfill;
    END IF;

    IF EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'ag_catalog') THEN
        BEGIN
            EXECUTE $q$
                SELECT count(*)::int
                FROM ag_catalog.ag_graph g
                WHERE NOT EXISTS (
                    SELECT 1 FROM public.eq_graph_backfill b
                    WHERE b.graph_name = g.name::text
                      AND b.finished_at IS NOT NULL
                      AND b.shadow_ok
                )
            $q$ INTO pending;
        EXCEPTION WHEN undefined_table THEN
            pending := 0;
        END;
        IF pending > 0 THEN
            RAISE EXCEPTION
                'SPEC-164 confirm-drop refused: % AGE graph(s) are not backfilled', pending;
        END IF;
    END IF;

    IF to_regclass('public.graph_nodes') IS NOT NULL THEN
        EXECUTE 'SELECT count(*) FROM public.graph_nodes' INTO nonempty;
        IF nonempty > 0 THEN
            RAISE EXCEPTION 'SPEC-164 confirm-drop refused: public.graph_nodes is not empty';
        END IF;
        DROP TABLE public.graph_nodes;
    END IF;
    IF to_regclass('public.graph_edges') IS NOT NULL THEN
        EXECUTE 'SELECT count(*) FROM public.graph_edges' INTO nonempty;
        IF nonempty > 0 THEN
            RAISE EXCEPTION 'SPEC-164 confirm-drop refused: public.graph_edges is not empty';
        END IF;
        DROP TABLE public.graph_edges;
    END IF;

    SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') INTO age_on;
    IF age_on THEN
        FOR g IN EXECUTE 'SELECT name::text FROM ag_catalog.ag_graph' LOOP
            IF g !~ '^[A-Za-z_][A-Za-z0-9_]*$' THEN
                RAISE EXCEPTION 'refusing unsafe graph name %', g;
            END IF;
            EXECUTE format('DROP TRIGGER IF EXISTS eq_graph_mirror_node ON %I."Node"', g);
            EXECUTE format('DROP TRIGGER IF EXISTS eq_graph_mirror_edge ON %I."EDGE"', g);
            PERFORM ag_catalog.drop_graph(g, true);
        END LOOP;
        DROP FUNCTION IF EXISTS public.eq_graph_mirror_node();
        DROP FUNCTION IF EXISTS public.eq_graph_mirror_edge();
        DROP FUNCTION IF EXISTS public.eq_graph_neighbor_ids_age(text, text, int, int);
        -- Routines from earlier migrations (eq_next_node_id returns graphid)
        -- keep a dependency on the extension. Drop those, then the extension.
        FOR g IN
            SELECT p.oid::regprocedure::text AS sig
            FROM pg_depend d
            JOIN pg_proc p ON p.oid = d.objid
            JOIN pg_extension e ON e.oid = d.refobjid
            WHERE e.extname = 'age'
              AND d.classid = 'pg_proc'::regclass
              AND p.pronamespace = 'public'::regnamespace
        LOOP
            EXECUTE format('DROP FUNCTION IF EXISTS %s CASCADE', g);
        END LOOP;
        BEGIN
            DROP EXTENSION age;
        EXCEPTION
            WHEN dependent_objects_still_exist THEN
                -- Graphs and known routines are already gone. Leftover agtype
                -- helpers still pin the extension; drop those with it.
                DROP EXTENSION age CASCADE;
        END;
    END IF;

    UPDATE public.eq_graph_cutover
       SET status = 'ready',
           age_absent = true,
           updated_at = now(),
           detail = COALESCE(detail, '{}'::jsonb) || jsonb_build_object('confirm_drop', true)
     WHERE id = 1;
END $$;
