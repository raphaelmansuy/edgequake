-- Older migrations created EdgeQuake tables and functions in ag_catalog
-- because that schema was first on search_path while Apache AGE was
-- installed. Migration 171 drops the extension. The empty tables, indexes,
-- and helper functions in ag_catalog stay until this cleanup runs.
--
-- Apache AGE stays installed unless `edgequake migrate --drop-age` applies
-- 171. sqlx cannot skip this version to reach 206, so while the extension
-- is still installed the function returns and the version is recorded.
-- The same function runs again at the end of a later `--drop-age`, after
-- 171 has removed the extension.

CREATE OR REPLACE FUNCTION public.eq_drop_empty_ag_catalog()
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    rel text;
    has_row boolean;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = 'ag_catalog') THEN
        RETURN;
    END IF;

    -- Keep the extension. A later --drop-age calls this function again.
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RETURN;
    END IF;

    IF to_regclass('public.eq_graph_cutover') IS NULL
       OR NOT EXISTS (
           SELECT 1
           FROM public.eq_graph_cutover
           WHERE id = 1
             AND status = 'ready'
             AND age_absent IS TRUE
       )
    THEN
        RAISE EXCEPTION
            '205: refusing to drop ag_catalog before the plain graph cutover is ready';
    END IF;

    FOR rel IN
        SELECT c.relname
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'ag_catalog'
          AND c.relkind = 'r'
    LOOP
        EXECUTE format('SELECT EXISTS (SELECT 1 FROM ag_catalog.%I)', rel)
            INTO has_row;
        IF has_row THEN
            RAISE EXCEPTION '205: ag_catalog.% is not empty', rel;
        END IF;
    END LOOP;

    DROP SCHEMA ag_catalog CASCADE;
END $$;

SELECT public.eq_drop_empty_ag_catalog();
